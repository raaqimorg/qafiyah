pub mod api;
pub mod config;
pub mod constants;
pub mod forwarding;
pub mod log;
pub mod metrics;
pub mod results;
pub mod slugs;
pub mod tools;

#[cfg(test)]
mod test_support;

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{BoxError, Router};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tower::ServiceBuilder;
use tower::timeout::TimeoutLayer;
use tower::timeout::error::Elapsed;

use crate::constants::{MAX_REQUEST_BODY_BYTES, REQUEST_DEADLINE_SECONDS};
use crate::tools::Catalog;

async fn deadline_passed(error: BoxError) -> StatusCode {
    if error.is::<Elapsed>() {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

pub fn with_deadline(router: Router, limit: Duration) -> Router {
    router.layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(deadline_passed))
            .layer(TimeoutLayer::new(limit)),
    )
}

pub fn transport_config(allowed_hosts: &[String]) -> StreamableHttpServerConfig {
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_max_request_body_bytes(MAX_REQUEST_BODY_BYTES)
        .enforce_origin_validation();
    if allowed_hosts.is_empty() {
        config
    } else {
        config.with_allowed_hosts(allowed_hosts.iter().cloned())
    }
}

pub fn app(catalog: Catalog, allowed_hosts: &[String]) -> Router {
    let mcp = StreamableHttpService::new(
        move || Ok(catalog.clone()),
        Arc::new(NeverSessionManager::default()),
        transport_config(allowed_hosts),
    );
    let routes = Router::new()
        .route_service("/mcp", mcp)
        .route("/healthz", get(|| async { StatusCode::OK }))
        .fallback(|| async { StatusCode::NOT_FOUND });
    with_deadline(routes, Duration::from_secs(REQUEST_DEADLINE_SECONDS))
}

pub async fn shutdown_signal() {
    let interrupt = async {
        let _interrupt = tokio::signal::ctrl_c().await;
    };
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    tokio::select! {
        () = interrupt => {},
        () = terminate => {},
    }
    tracing::info!(stage = "draining");
}

pub async fn serve<F>(
    listener: tokio::net::TcpListener,
    app: Router,
    shutdown: F,
) -> std::io::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use ipnet::IpNet;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::*;
    use crate::api::Api;
    use crate::metrics::Metrics;
    use crate::test_support::FakeApi;
    use crate::tools::Catalog;

    const ACCEPT: &str = "application/json, text/event-stream";

    async fn served(allowed_hosts: &[String]) -> (FakeApi, Router) {
        let fake =
            FakeApi::serving(StatusCode::OK, &[], r#"{"q":"سلام","poems":{"data":[]}}"#).await;
        let catalog = Catalog::new(
            Api::new(&fake.url, None, Duration::from_secs(2)).expect("a client"),
            Arc::from(Vec::<IpNet>::new()),
            Metrics::default(),
            1_000,
        );
        (fake, app(catalog, allowed_hosts))
    }

    fn post(host: &str, body: String, headers: &[(&str, &str)]) -> Request<Body> {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("host", host)
            .header("content-type", "application/json")
            .header("accept", ACCEPT);
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        builder.body(Body::from(body)).expect("a request")
    }

    fn rpc(method: &str, params: &Value) -> String {
        json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }).to_string()
    }

    fn initialize() -> String {
        rpc(
            "initialize",
            &json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "t", "version": "1" },
            }),
        )
    }

    async fn send(app: Router, request: Request<Body>) -> (StatusCode, Option<String>, Value) {
        let response = app
            .oneshot(request)
            .await
            .expect("the router is infallible");
        let status = response.status();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let bytes = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("a readable body");
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, content_type, body)
    }

    async fn status(app: Router, request: Request<Body>) -> StatusCode {
        send(app, request).await.0
    }

    fn get_request(path: &str) -> Request<Body> {
        Request::builder()
            .uri(path)
            .header("host", "localhost")
            .body(Body::empty())
            .expect("a request")
    }

    #[tokio::test]
    async fn the_health_probe_answers_200_and_any_other_path_404() {
        let (_fake, app) = served(&[]).await;
        assert_eq!(
            status(app.clone(), get_request("/healthz")).await,
            StatusCode::OK
        );
        assert_eq!(
            status(app, get_request("/nothing")).await,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn a_request_past_the_deadline_is_ended_with_503() {
        let slow = Router::new().route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(1)).await;
                "late"
            }),
        );
        let app = with_deadline(slow, Duration::from_millis(50));
        assert_eq!(
            status(app, get_request("/slow")).await,
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[tokio::test]
    async fn initialize_answers_with_the_server_info() {
        let (_fake, app) = served(&[]).await;
        let (status, content_type, body) = send(app, post("localhost", initialize(), &[])).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(
            content_type.is_some_and(|value| value.starts_with("application/json")),
            "a JSON response"
        );
        assert_eq!(body["result"]["serverInfo"]["name"], "qafiyah");
    }

    #[tokio::test]
    async fn tools_list_answers_the_nine_read_only_tools() {
        let (_fake, app) = served(&[]).await;
        let (status, _, body) = send(
            app,
            post(
                "localhost",
                rpc("tools/list", &json!({})),
                &[("mcp-protocol-version", "2025-06-18")],
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let tools = body["result"]["tools"].as_array().expect("a tools list");
        assert_eq!(tools.len(), 9);
        for tool in tools {
            assert_eq!(
                tool["annotations"]["readOnlyHint"], true,
                "{}",
                tool["name"]
            );
        }
    }

    #[tokio::test]
    async fn an_older_protocol_client_is_served_without_a_session() {
        let (fake, app) = served(&[]).await;
        let (status, _, body) = send(app.clone(), post("localhost", initialize(), &[])).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let call = rpc(
            "tools/call",
            &json!({ "name": "search", "arguments": { "q": "سلام" } }),
        );
        let (status, _, body) = send(
            app,
            post("localhost", call, &[("mcp-protocol-version", "2025-06-18")]),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["result"]["isError"], false, "{body}");
        assert_eq!(fake.requests().await.len(), 1);
    }

    #[tokio::test]
    async fn a_host_outside_the_allowed_list_is_refused() {
        let (_fake, app) = served(&["mcp.qafiyah.com".into()]).await;
        assert_eq!(
            status(app.clone(), post("evil.example", initialize(), &[])).await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            status(app, post("mcp.qafiyah.com", initialize(), &[])).await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn a_request_with_an_origin_is_refused() {
        let (_fake, app) = served(&[]).await;
        for origin in ["https://evil.example", "http://localhost:6274"] {
            assert_eq!(
                status(
                    app.clone(),
                    post("localhost", initialize(), &[("origin", origin)])
                )
                .await,
                StatusCode::FORBIDDEN,
                "{origin}"
            );
        }
    }

    #[tokio::test]
    async fn get_and_delete_on_mcp_answer_405() {
        let (_fake, app) = served(&[]).await;
        for method in ["GET", "DELETE"] {
            let request = Request::builder()
                .method(method)
                .uri("/mcp")
                .header("host", "localhost")
                .header("accept", ACCEPT)
                .body(Body::empty())
                .expect("a request");
            assert_eq!(
                status(app.clone(), request).await,
                StatusCode::METHOD_NOT_ALLOWED,
                "{method}"
            );
        }
    }

    async fn forwarded_over_a_real_connection(trusted: &str) -> Option<String> {
        let fake = FakeApi::serving(StatusCode::OK, &[], r#"{"data":[]}"#).await;
        let catalog = Catalog::new(
            Api::new(&fake.url, None, Duration::from_secs(2)).expect("a client"),
            Arc::from(vec![trusted.parse::<IpNet>().expect("a network")]),
            Metrics::default(),
            1_000,
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        tokio::spawn(serve(
            listener,
            app(catalog, &[]),
            std::future::pending::<()>(),
        ));
        let response = reqwest::Client::new()
            .post(format!("http://{address}/mcp"))
            .header("content-type", "application/json")
            .header("accept", ACCEPT)
            .header("mcp-protocol-version", "2025-06-18")
            .header("cf-connecting-ip", "203.0.113.7")
            .body(rpc(
                "tools/call",
                &json!({ "name": "list_feelings", "arguments": {} }),
            ))
            .send()
            .await
            .expect("an answer");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let seen = fake.requests().await;
        assert_eq!(seen.len(), 1, "one API call");
        seen.into_iter()
            .next()
            .and_then(|request| request.headers.get("cf-connecting-ip").cloned())
            .and_then(|value| value.to_str().ok().map(str::to_string))
    }

    #[tokio::test]
    async fn a_trusted_peers_caller_address_reaches_the_api_over_a_real_connection() {
        assert_eq!(
            forwarded_over_a_real_connection("127.0.0.0/8").await,
            Some("203.0.113.7".to_string())
        );
    }

    #[tokio::test]
    async fn an_untrusted_peers_claimed_address_never_reaches_the_api() {
        assert_eq!(
            forwarded_over_a_real_connection("172.27.0.0/24").await,
            None
        );
    }

    #[tokio::test]
    async fn a_body_over_64_kib_is_refused() {
        let (_fake, app) = served(&[]).await;
        let body = format!("{{\"pad\":\"{}\"}}", "a".repeat(65_537));
        assert_eq!(
            status(app, post("localhost", body, &[])).await,
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }
}
