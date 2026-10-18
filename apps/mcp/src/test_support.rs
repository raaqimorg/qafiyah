use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::State;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use prometheus_client::encoding::prometheus_protobuf::prometheus_data_model::MetricFamily;
use prost::Message;
use tokio::sync::Mutex;
use tower::ServiceExt;

use crate::metrics::Metrics;

pub(crate) async fn scrape(metrics: &Metrics) -> Vec<MetricFamily> {
    let response = crate::metrics::router(metrics.clone())
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .expect("a request"),
        )
        .await
        .expect("the metrics router is infallible");
    let mut bytes = to_bytes(response.into_body(), 16 * 1024 * 1024)
        .await
        .expect("a readable body");
    let mut families = Vec::new();
    while !bytes.is_empty() {
        families.push(
            MetricFamily::decode_length_delimited(&mut bytes).expect("a delimited metric family"),
        );
    }
    families
}

pub(crate) fn histogram_count(
    families: &[MetricFamily],
    name: &str,
    labels: &[(&str, &str)],
) -> u64 {
    families
        .iter()
        .filter(|family| family.name == name)
        .flat_map(|family| &family.metric)
        .filter(|metric| {
            labels.iter().all(|(key, value)| {
                metric
                    .label
                    .iter()
                    .any(|pair| pair.name == *key && pair.value == *value)
            })
        })
        .filter_map(|metric| metric.histogram.as_ref())
        .map(|histogram| histogram.sample_count)
        .sum()
}

#[derive(Clone)]
struct Canned {
    status: StatusCode,
    headers: Vec<(String, String)>,
    body: String,
    delay: Duration,
    seen: Arc<Mutex<Vec<Seen>>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Seen {
    pub(crate) path_and_query: String,
    pub(crate) headers: HeaderMap,
}

pub(crate) struct FakeApi {
    pub(crate) url: String,
    seen: Arc<Mutex<Vec<Seen>>>,
}

async fn answer(State(canned): State<Canned>, request: Request<Body>) -> Response {
    canned.seen.lock().await.push(Seen {
        path_and_query: request
            .uri()
            .path_and_query()
            .map(ToString::to_string)
            .unwrap_or_default(),
        headers: request.headers().clone(),
    });
    tokio::time::sleep(canned.delay).await;
    let mut response = (canned.status, canned.body).into_response();
    for (name, value) in canned.headers {
        response.headers_mut().insert(
            HeaderName::try_from(name).expect("a header name"),
            HeaderValue::try_from(value).expect("a header value"),
        );
    }
    response
}

impl FakeApi {
    pub(crate) async fn serving(status: StatusCode, headers: &[(&str, &str)], body: &str) -> Self {
        Self::start(status, headers, body, Duration::ZERO).await
    }

    pub(crate) async fn slow(delay: Duration) -> Self {
        Self::start(StatusCode::OK, &[], "{}", delay).await
    }

    async fn start(
        status: StatusCode,
        headers: &[(&str, &str)],
        body: &str,
        delay: Duration,
    ) -> Self {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let router = Router::new().fallback(answer).with_state(Canned {
            status,
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
                .collect(),
            body: body.to_string(),
            delay,
            seen: seen.clone(),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        tokio::spawn(async move {
            let _result = axum::serve(listener, router).await;
        });
        Self {
            url: format!("http://{address}"),
            seen,
        }
    }

    pub(crate) async fn requests(&self) -> Vec<Seen> {
        self.seen.lock().await.clone()
    }
}
