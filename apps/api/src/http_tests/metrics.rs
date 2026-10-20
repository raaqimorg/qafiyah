use axum::http::StatusCode;

use crate::http_tests::empty_hits;
use axum::body::Body;
use axum::http::Request;

use crate::auth::Keys;
use crate::rate_limit::McpLimits;
use crate::test_support::{
    FakeEs, counter_value, gauge_value, histogram_count, request, scrape, send, state, state_with,
};

const REQUESTS: &str = "http_server_request_duration_seconds";

#[tokio::test]
async fn a_request_is_counted_once_under_its_route_template() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let state = state(&es);
    let sent = send(
        crate::app(state.clone()),
        request("GET", "/v1/search?q=abc&types=poems"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::OK);
    let families = scrape(&state.metrics).await;
    assert_eq!(
        histogram_count(
            &families,
            REQUESTS,
            &[
                ("http_request_method", "GET"),
                ("http_route", "/v1/search"),
                ("http_response_status_code", "200"),
            ]
        ),
        1
    );
}

#[tokio::test]
async fn a_path_parameter_never_becomes_a_label() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let state = state(&es);
    send(crate::app(state.clone()), request("GET", "/v1/poems/gnNg")).await;
    let families = scrape(&state.metrics).await;
    assert_eq!(
        histogram_count(&families, REQUESTS, &[("http_route", "/v1/poems/{slug}")]),
        1
    );
    assert_eq!(
        histogram_count(&families, REQUESTS, &[("http_route", "/v1/poems/gnNg")]),
        0
    );
}

#[tokio::test]
async fn unmatched_paths_and_unknown_methods_share_one_label_each() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let state = state(&es);
    for path in ["/nope/1", "/nope/2", "/wp-login.php"] {
        let sent = send(crate::app(state.clone()), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::NOT_FOUND, "{path}");
    }
    send(crate::app(state.clone()), request("FOO", "/healthz")).await;
    let families = scrape(&state.metrics).await;
    assert_eq!(
        histogram_count(
            &families,
            REQUESTS,
            &[
                ("http_route", "unmatched"),
                ("http_response_status_code", "404")
            ]
        ),
        3
    );
    assert_eq!(
        histogram_count(&families, REQUESTS, &[("http_request_method", "_OTHER")]),
        1
    );
}

#[tokio::test]
async fn the_public_app_does_not_serve_metrics() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let sent = send(crate::app(state(&es)), request("GET", "/metrics")).await;
    assert_eq!(sent.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_metrics_router_answers_in_the_prometheus_protobuf_format() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let state = state(&es);
    send(crate::app(state.clone()), request("GET", "/healthz")).await;
    let sent = send(
        crate::metrics::router(state.metrics.clone()),
        request("GET", "/metrics"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(
        sent.header("content-type"),
        Some(crate::metrics::PROMETHEUS_PROTOBUF)
    );
}

const SEARCHES: &str = "db_client_operation_duration_seconds";

#[tokio::test]
async fn every_elasticsearch_search_is_timed_under_its_alias() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let state = state(&es);
    let identity = qafiyah_elasticsearch::load().identity;
    send(
        crate::app(state.clone()),
        request("GET", "/v1/search?q=abc&types=poems"),
    )
    .await;
    let families = scrape(&state.metrics).await;
    assert_eq!(
        histogram_count(
            &families,
            SEARCHES,
            &[
                ("db_system_name", "elasticsearch"),
                ("db_collection_name", identity.poems_alias.as_str()),
            ]
        ),
        1
    );
}

#[tokio::test]
async fn a_failed_search_is_timed_too() {
    let es = FakeEs::serving(StatusCode::INTERNAL_SERVER_ERROR, serde_json::json!({})).await;
    let state = state(&es);
    let sent = send(
        crate::app(state.clone()),
        request("GET", "/v1/search?q=abc&types=poems"),
    )
    .await;
    assert!(sent.status.is_server_error(), "{}", sent.status);
    let families = scrape(&state.metrics).await;
    assert_eq!(histogram_count(&families, SEARCHES, &[]), 1);
}

fn mcp_call(ip: &str) -> Request<Body> {
    Request::builder()
        .uri("/v1/openapi.json")
        .header("cf-connecting-ip", ip)
        .header("x-api-key", "mcp")
        .body(Body::empty())
        .expect("a request")
}

#[tokio::test]
async fn mcp_admissions_and_refusals_are_counted_by_the_limit_that_refused() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let mut state = state_with(
        &es,
        Keys::new(Some("internal".into()), Some("full".into())).with_mcp(Some("mcp".into())),
        1,
    );
    state.mcp = McpLimits {
        requests: 1,
        burst: 100,
        address_requests: 100,
    };
    let app = crate::app(state.clone());
    assert_eq!(
        send(app.clone(), mcp_call("203.0.113.1")).await.status,
        StatusCode::OK
    );
    assert_eq!(
        send(app, mcp_call("203.0.113.2")).await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
    let families = scrape(&state.metrics).await;
    assert_eq!(
        counter_value(&families, "mcp_rate_limit_admitted_total", &[]),
        1.0
    );
    assert_eq!(
        counter_value(
            &families,
            "mcp_rate_limit_refusals_total",
            &[("limit", "hourly")]
        ),
        1.0
    );
    assert_eq!(
        counter_value(
            &families,
            "mcp_rate_limit_refusals_total",
            &[("limit", "address")]
        ),
        0.0
    );
}

#[tokio::test]
async fn the_hourly_mcp_limit_is_exported() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let state = state(&es);
    state.metrics.set_mcp_hourly_limit(20_000);
    let families = scrape(&state.metrics).await;
    assert_eq!(
        gauge_value(&families, "mcp_rate_limit_hourly_limit", &[]),
        20_000.0
    );
}
