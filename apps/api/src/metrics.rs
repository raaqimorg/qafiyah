use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use axum::extract::{MatchedPath, Request, State};
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::encoding::prometheus_protobuf::{EncodeError, encode_to_vec};
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::histogram::{Histogram, NativeHistogramConfig};
use prometheus_client::registry::Registry;

pub const PROMETHEUS_PROTOBUF: &str =
    "application/vnd.google.protobuf;proto=io.prometheus.client.MetricFamily;encoding=delimited";

const UNMATCHED_ROUTE: &str = "unmatched";
const OTHER_METHOD: &str = "_OTHER";
const ELASTICSEARCH: &str = "elasticsearch";

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct RequestLabels {
    http_request_method: &'static str,
    http_route: String,
    http_response_status_code: u16,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct SearchLabels {
    db_system_name: &'static str,
    db_collection_name: String,
}

#[derive(Clone)]
pub struct Metrics {
    registry: Arc<Registry>,
    requests: Family<RequestLabels, Histogram>,
    searches: Family<SearchLabels, Histogram>,
}

fn native() -> Histogram {
    Histogram::new_native(NativeHistogramConfig::default())
}

fn method_label(method: &Method) -> &'static str {
    match *method {
        Method::GET => "GET",
        Method::HEAD => "HEAD",
        Method::POST => "POST",
        Method::PUT => "PUT",
        Method::PATCH => "PATCH",
        Method::DELETE => "DELETE",
        Method::OPTIONS => "OPTIONS",
        _ => OTHER_METHOD,
    }
}

impl Default for Metrics {
    fn default() -> Self {
        let constructor: fn() -> Histogram = native;
        let requests = Family::<RequestLabels, Histogram>::new_with_constructor(constructor);
        let searches = Family::<SearchLabels, Histogram>::new_with_constructor(constructor);
        let mut registry = Registry::default();
        registry.register(
            "http_server_request_duration_seconds",
            "Duration of every HTTP request the API answered",
            requests.clone(),
        );
        registry.register(
            "db_client_operation_duration_seconds",
            "Duration of every Elasticsearch search the API sent",
            searches.clone(),
        );
        Self {
            registry: Arc::new(registry),
            requests,
            searches,
        }
    }
}

impl Metrics {
    pub fn observe_request(
        &self,
        method: &Method,
        route: Option<&str>,
        status: StatusCode,
        seconds: f64,
    ) {
        self.requests
            .get_or_create(&RequestLabels {
                http_request_method: method_label(method),
                http_route: route.unwrap_or(UNMATCHED_ROUTE).to_string(),
                http_response_status_code: status.as_u16(),
            })
            .observe(seconds);
    }

    pub fn observe_search(&self, index: &str, seconds: f64) {
        self.searches
            .get_or_create(&SearchLabels {
                db_system_name: ELASTICSEARCH,
                db_collection_name: index.to_string(),
            })
            .observe(seconds);
    }

    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        encode_to_vec(&self.registry)
    }
}

pub async fn layer(State(metrics): State<Metrics>, request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|path| path.as_str().to_string());
    let started = Instant::now();
    let response = next.run(request).await;
    metrics.observe_request(
        &method,
        route.as_deref(),
        response.status(),
        started.elapsed().as_secs_f64(),
    );
    response
}

async fn scrape(State(metrics): State<Metrics>) -> Response {
    match metrics.encode() {
        Ok(body) => ([(header::CONTENT_TYPE, PROMETHEUS_PROTOBUF)], body).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub fn router(metrics: Metrics) -> Router {
    Router::new()
        .route("/metrics", get(scrape))
        .with_state(metrics)
}
