use axum::http::StatusCode;

use crate::http_tests::{app_with, empty_hits};
use crate::test_support::{FakeEs, request, send};

async fn es() -> FakeEs {
    FakeEs::serving(StatusCode::OK, empty_hits()).await
}

#[tokio::test]
async fn an_unknown_root_path_is_a_problem_document() {
    let es = es().await;
    let sent = send(app_with(&es), request("GET", "/nope")).await;
    assert_eq!(sent.status, StatusCode::NOT_FOUND);
    assert_eq!(
        sent.header("content-type"),
        Some("application/problem+json")
    );
    assert_eq!(sent.header("cache-control"), Some("no-store"));
    let body = sent.json();
    assert_eq!(body["code"], "NOT_FOUND");
    assert_eq!(body["instance"], "/nope");
    assert_eq!(body["detail"], "No route matches GET /nope");
}

#[tokio::test]
async fn an_unknown_v1_path_reports_the_full_path() {
    let es = es().await;
    let sent = send(app_with(&es), request("GET", "/v1/nope")).await;
    assert_eq!(sent.status, StatusCode::NOT_FOUND);
    let body = sent.json();
    assert_eq!(body["instance"], "/v1/nope");
    assert_eq!(body["detail"], "No route matches GET /v1/nope");
}

#[tokio::test]
async fn the_v1_root_redirects_to_the_reference() {
    let es = es().await;
    let sent = send(app_with(&es), request("GET", "/v1")).await;
    assert_eq!(sent.status, StatusCode::FOUND);
    assert_eq!(sent.header("location"), Some("/v1/docs"));
}

#[tokio::test]
async fn a_head_request_on_a_json_route_carries_the_get_headers() {
    let es = es().await;
    let sent = send(app_with(&es), request("HEAD", "/v1/openapi.json")).await;
    assert_eq!(sent.status, StatusCode::OK);
    assert!(sent.header("etag").is_some());
}

#[tokio::test]
async fn a_post_to_a_contract_route_is_a_problem_document() {
    let es = es().await;
    let sent = send(app_with(&es), request("POST", "/v1/meters")).await;
    assert_eq!(sent.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(
        sent.header("content-type"),
        Some("application/problem+json")
    );
    let body = sent.json();
    assert_eq!(body["code"], "METHOD_NOT_ALLOWED");
    assert_eq!(body["instance"], "/v1/meters");
}

#[tokio::test]
async fn an_unparseable_path_segment_is_a_problem_document() {
    let es = es().await;
    let sent = send(app_with(&es), request("GET", "/v1/poems/%FF%FE")).await;
    assert_eq!(sent.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        sent.header("content-type"),
        Some("application/problem+json")
    );
    let body = sent.json();
    assert_eq!(body["code"], "BAD_REQUEST");
    assert_eq!(body["instance"], "/v1/poems/%FF%FE");
}

#[tokio::test]
async fn every_contract_path_is_routed() {
    let es = es().await;
    let mut sends = tokio::task::JoinSet::new();
    for path in [
        "/v1/meters",
        "/v1/meters/altawil",
        "/v1/rhymes",
        "/v1/rhymes/meem",
        "/v1/eras",
        "/v1/eras/abbasi",
        "/v1/themes",
        "/v1/themes/alnasib",
        "/v1/collections",
        "/v1/collections/almuallaqat",
        "/v1/poem-types",
        "/v1/poem-types/amudi",
        "/v1/poems",
        "/v1/poems/slugs",
        "/v1/poems/count",
        "/v1/poems/TnKK",
        "/v1/poets",
        "/v1/poets/slugs",
        "/v1/poets/yoFB",
        "/v1/search",
        "/v1/poems/random",
        "/v1/openapi.json",
        "/v1/docs",
    ] {
        let app = app_with(&es);
        sends.spawn(async move { (path, send(app, request("GET", path)).await) });
    }
    while let Some(joined) = sends.join_next().await {
        let (path, sent) = joined.expect("a routing probe task never panics");
        assert_ne!(sent.status, StatusCode::NOT_FOUND, "{path} must be routed");
        assert_ne!(
            sent.status,
            StatusCode::METHOD_NOT_ALLOWED,
            "{path} must accept GET"
        );
    }
}
