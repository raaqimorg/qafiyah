use axum::http::StatusCode;

use crate::http_tests::empty_hits;
use crate::test_support::{FakeEs, histogram_count, request, scrape, send, state};

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
