use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::encoding::prometheus_protobuf::{EncodeError, encode_to_vec};
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::histogram::{Histogram, NativeHistogramConfig};
use prometheus_client::registry::Registry;

const PROMETHEUS_PROTOBUF: &str =
    "application/vnd.google.protobuf;proto=io.prometheus.client.MetricFamily;encoding=delimited";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Search,
    ListPoems,
    GetPoem,
    RandomPoem,
    ListPoets,
    GetPoet,
    ListTerms,
    ListFeelings,
    FeelingVerses,
}

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::ListPoems => "list_poems",
            Self::GetPoem => "get_poem",
            Self::RandomPoem => "random_poem",
            Self::ListPoets => "list_poets",
            Self::GetPoet => "get_poet",
            Self::ListTerms => "list_terms",
            Self::ListFeelings => "list_feelings",
            Self::FeelingVerses => "feeling_verses",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    BadInput,
    Refused,
    NotFound,
    Unavailable,
}

impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::BadInput => "bad_input",
            Self::Refused => "refused",
            Self::NotFound => "not_found",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct CallLabels {
    tool: &'static str,
    outcome: &'static str,
}

#[derive(Clone)]
pub struct Metrics {
    registry: Arc<Registry>,
    calls: Family<CallLabels, Histogram>,
}

fn native() -> Histogram {
    Histogram::new_native(NativeHistogramConfig::default())
}

impl Default for Metrics {
    fn default() -> Self {
        let constructor: fn() -> Histogram = native;
        let calls = Family::<CallLabels, Histogram>::new_with_constructor(constructor);
        let mut registry = Registry::default();
        registry.register(
            "mcp_tool_call_duration_seconds",
            "Duration of every MCP tool call, by tool and outcome",
            calls.clone(),
        );
        Self {
            registry: Arc::new(registry),
            calls,
        }
    }
}

impl Metrics {
    pub fn observe_call(&self, tool: Tool, outcome: Outcome, seconds: f64) {
        self.calls
            .get_or_create(&CallLabels {
                tool: tool.name(),
                outcome: outcome.label(),
            })
            .observe(seconds);
    }

    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        encode_to_vec(&self.registry)
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{histogram_count, scrape};

    #[tokio::test]
    async fn a_tool_call_is_recorded_by_tool_and_outcome() {
        let metrics = Metrics::default();
        metrics.observe_call(Tool::Search, Outcome::Refused, 0.1);
        let families = scrape(&metrics).await;
        assert_eq!(
            histogram_count(
                &families,
                "mcp_tool_call_duration_seconds",
                &[("tool", "search"), ("outcome", "refused")]
            ),
            1
        );
        assert_eq!(
            histogram_count(
                &families,
                "mcp_tool_call_duration_seconds",
                &[("tool", "search"), ("outcome", "ok")]
            ),
            0
        );
    }

    #[test]
    fn every_label_comes_from_a_fixed_set() {
        let tools = [
            Tool::Search,
            Tool::ListPoems,
            Tool::GetPoem,
            Tool::RandomPoem,
            Tool::ListPoets,
            Tool::GetPoet,
            Tool::ListTerms,
            Tool::ListFeelings,
            Tool::FeelingVerses,
        ]
        .map(Tool::name);
        assert_eq!(
            tools,
            [
                "search",
                "list_poems",
                "get_poem",
                "random_poem",
                "list_poets",
                "get_poet",
                "list_terms",
                "list_feelings",
                "feeling_verses",
            ]
        );
        let outcomes = [
            Outcome::Ok,
            Outcome::BadInput,
            Outcome::Refused,
            Outcome::NotFound,
            Outcome::Unavailable,
        ]
        .map(Outcome::label);
        assert_eq!(
            outcomes,
            ["ok", "bad_input", "refused", "not_found", "unavailable"]
        );
    }
}
