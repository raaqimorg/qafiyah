use std::time::Duration;

use axum::extract::MatchedPath;
use axum::http::{Request, Response};
use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
use tower_http::trace::{MakeSpan, OnResponse, TraceLayer};
use tracing::Span;
use tracing::field::Empty;
use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::MakeWriter;

const SLOW_REQUEST: Duration = Duration::from_secs(2);
const UNMATCHED_ROUTE: &str = "unmatched";
const PRODUCTION: &str = "production";

fn skipped(path: &str) -> bool {
    path == "/healthz"
        || path == "/v1/openapi.json"
        || path == "/v1/docs"
        || path.starts_with("/v1/docs/")
}

pub fn default_directives(environment: &str) -> &'static str {
    if environment == PRODUCTION {
        "warn,qafiyah_api=info"
    } else {
        "warn,qafiyah_api=debug"
    }
}

pub fn subscriber<W>(directives: &str, writer: W) -> impl tracing::Subscriber + Send + Sync
where
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_current_span(true)
        .with_span_list(false)
        .with_target(false)
        .with_env_filter(EnvFilter::new(directives))
        .with_writer(writer)
        .finish()
}

pub fn init(environment: &str) -> Result<(), SetGlobalDefaultError> {
    let directives = std::env::var("RUST_LOG")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_directives(environment).to_string());
    tracing::subscriber::set_global_default(subscriber(&directives, std::io::stdout))
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RequestSpan;

impl<B> MakeSpan<B> for RequestSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let path = request.uri().path();
        if skipped(path) {
            return Span::none();
        }
        let route = request
            .extensions()
            .get::<MatchedPath>()
            .map_or(UNMATCHED_ROUTE, MatchedPath::as_str);
        let request_id = request
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        tracing::info_span!(
            "request",
            method = %request.method(),
            route,
            path,
            request_id,
            keyed = Empty,
            api_key_id = Empty,
            rate_limit_remaining = Empty,
            result_count = Empty,
            page = Empty,
            page_size = Empty,
            total_pages = Empty,
            poem_id = Empty,
            poet_id = Empty,
            alias_of = Empty,
            era = Empty,
            meter = Empty,
            theme = Empty,
            rhyme = Empty,
            collection = Empty,
            poem_type = Empty,
            query_text = Empty,
            poems_count = Empty,
            poets_count = Empty,
        )
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RequestLine;

impl<B> OnResponse<B> for RequestLine {
    fn on_response(self, response: &Response<B>, latency: Duration, span: &Span) {
        if span.is_none() {
            return;
        }
        let status = response.status().as_u16();
        let duration_ms = u64::try_from(latency.as_millis()).unwrap_or(u64::MAX);
        if response.status().is_server_error() {
            tracing::error!(status, duration_ms, "request");
        } else if latency > SLOW_REQUEST {
            tracing::warn!(status, duration_ms, "request");
        } else {
            tracing::debug!(status, duration_ms, "request");
        }
    }
}

pub type Trace =
    TraceLayer<SharedClassifier<ServerErrorsAsFailures>, RequestSpan, (), RequestLine, (), (), ()>;

pub fn trace() -> Trace {
    TraceLayer::new_for_http()
        .make_span_with(RequestSpan)
        .on_request(())
        .on_response(RequestLine)
        .on_body_chunk(())
        .on_eos(())
        .on_failure(())
}

pub fn record_results(total: u64) {
    Span::current().record("result_count", total);
    if total == 0 {
        tracing::info!("found nothing");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_the_document_the_reference_page_and_the_health_check() {
        assert!(skipped("/v1/openapi.json"));
        assert!(skipped("/v1/docs"));
        assert!(skipped("/v1/docs/anything"));
        assert!(skipped("/healthz"));
        assert!(!skipped("/v1/poems"));
        assert!(!skipped("/v1/search"));
        assert!(!skipped("/v1/poems/random"));
        assert!(!skipped("/v1/docsXYZ"));
        assert!(!skipped("/healthz/x"));
        assert!(!skipped("/v1/healthz"));
    }

    #[test]
    fn production_drops_debug_lines_and_keeps_info_and_above() {
        let production = EnvFilter::new(default_directives("production"));
        assert_eq!(
            production.max_level_hint(),
            Some(tracing_subscriber::filter::LevelFilter::INFO)
        );
        for elsewhere in ["development", "unknown", ""] {
            assert_eq!(
                EnvFilter::new(default_directives(elsewhere)).max_level_hint(),
                Some(tracing_subscriber::filter::LevelFilter::DEBUG),
                "{elsewhere}"
            );
        }
    }
}
