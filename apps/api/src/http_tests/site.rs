use axum::http::StatusCode;

use crate::constants::{
    FAVICON_CACHE_CONTROL, LLMS_CACHE_CONTROL, ROBOTS_CACHE_CONTROL, SECURITY_CACHE_CONTROL,
};
use crate::http_tests::{app_with, empty_hits};
use crate::test_support::{FakeEs, request, send};

const TEMPLATE_TOKENS: [&str; 5] = ["{NAME}", "{BASE}", "{SITE}", "{API}", "{EMAIL}"];

async fn es() -> FakeEs {
    FakeEs::serving(StatusCode::OK, empty_hits()).await
}

fn fully_rendered(body: &str) -> bool {
    TEMPLATE_TOKENS.iter().all(|token| !body.contains(token))
}

#[tokio::test]
async fn the_llms_index_is_rendered_from_its_template() {
    let es = es().await;
    let sent = send(app_with(&es), request("GET", "/llms.txt")).await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(
        sent.header("content-type"),
        Some("text/plain; charset=utf-8")
    );
    assert_eq!(sent.header("cache-control"), Some(LLMS_CACHE_CONTROL));
    assert!(sent.body.contains("https://api.qafiyah.com/v1/poems"));
    assert!(sent.body.contains("https://qafiyah.com/developers"));
    assert!(fully_rendered(&sent.body), "{}", sent.body);
}

#[tokio::test]
async fn the_robots_policy_is_rendered_from_its_template() {
    let es = es().await;
    let sent = send(app_with(&es), request("GET", "/robots.txt")).await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(
        sent.header("content-type"),
        Some("text/plain; charset=utf-8")
    );
    assert_eq!(sent.header("cache-control"), Some(ROBOTS_CACHE_CONTROL));
    assert!(sent.body.contains("User-agent: *"));
    assert!(sent.body.contains("Disallow: /"));
    assert!(sent.body.contains("https://api.qafiyah.com/llms.txt"));
    assert!(fully_rendered(&sent.body), "{}", sent.body);
}

#[tokio::test]
async fn the_security_txt_names_the_security_mailbox() {
    let es = es().await;
    let sent = send(app_with(&es), request("GET", "/.well-known/security.txt")).await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(sent.header("cache-control"), Some(SECURITY_CACHE_CONTROL));
    assert!(sent.body.contains("mailto:security@qafiyah.com"));
    assert!(
        sent.body
            .contains("Canonical: https://qafiyah.com/.well-known/security.txt")
    );
    assert!(
        sent.body
            .contains("Canonical: https://api.qafiyah.com/.well-known/security.txt")
    );
    assert!(fully_rendered(&sent.body), "{}", sent.body);
}

#[tokio::test]
async fn the_favicon_is_an_inline_svg() {
    let es = es().await;
    let sent = send(app_with(&es), request("GET", "/favicon.ico")).await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(sent.header("content-type"), Some("image/svg+xml"));
    assert_eq!(sent.header("cache-control"), Some(FAVICON_CACHE_CONTROL));
    assert!(sent.body.starts_with("<svg"));
}

#[tokio::test]
async fn the_root_redirects_to_the_reference() {
    let es = es().await;
    let sent = send(app_with(&es), request("GET", "/")).await;
    assert_eq!(sent.status, StatusCode::FOUND);
    assert_eq!(sent.header("location"), Some("/v1/docs"));
    assert!(sent.header("cache-control").is_none());
}

#[tokio::test]
async fn the_reference_page_is_html_and_never_cached_by_the_api_layer() {
    let es = es().await;
    let sent = send(app_with(&es), request("GET", "/v1/docs")).await;
    assert_eq!(sent.status, StatusCode::OK);
    assert!(
        sent.header("content-type")
            .is_some_and(|value| value.starts_with("text/html"))
    );
    assert!(sent.header("etag").is_none());
    assert!(sent.header("cache-control").is_none());
}

#[tokio::test]
async fn the_openapi_document_is_served_as_json() {
    let es = es().await;
    let sent = send(app_with(&es), request("GET", "/v1/openapi.json")).await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(sent.header("content-type"), Some("application/json"));
    assert_eq!(sent.json()["paths"].as_object().expect("paths").len(), 22);
}
