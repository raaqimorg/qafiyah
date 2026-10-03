use axum::http::StatusCode;
use serde_json::json;

use crate::http_tests::{app_with, empty_hits};
use crate::test_support::{FakeEs, request, send};

#[tokio::test]
async fn a_database_that_cannot_be_reached_is_a_temporary_problem_that_hides_its_cause() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let sent = send(app_with(&es), request("GET", "/v1/poems/count")).await;
    assert_eq!(sent.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(sent.header("retry-after"), Some("2"));
    assert_eq!(
        sent.header("content-type"),
        Some("application/problem+json")
    );
    assert_eq!(sent.header("cache-control"), Some("no-store"));
    assert!(sent.header("etag").is_none());
    let body = sent.json();
    assert_eq!(body["code"], "SERVICE_UNAVAILABLE");
    assert_eq!(body["detail"], "Temporarily unavailable, try again shortly");
    assert_eq!(body["instance"], "/v1/poems/count");
    assert!(
        !sent.body.contains("127.0.0.1"),
        "the cause must not leak: {}",
        sent.body
    );
}

#[tokio::test]
async fn a_search_failure_is_a_generic_problem_that_hides_its_cause() {
    let es = FakeEs::serving(
        StatusCode::INTERNAL_SERVER_ERROR,
        json!({ "error": "boom" }),
    )
    .await;
    let sent = send(app_with(&es), request("GET", "/v1/search?q=%D8%AD%D8%A8")).await;
    assert_eq!(sent.status, StatusCode::INTERNAL_SERVER_ERROR);
    let body = sent.json();
    assert_eq!(body["code"], "INTERNAL_SERVER_ERROR");
    assert!(!sent.body.contains("boom"));
}

#[tokio::test]
async fn a_malformed_slug_is_refused_before_any_backend_is_asked() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    for path in [
        "/v1/poems/abc",
        "/v1/poems/ab1d",
        "/v1/meters/ALTAWIL",
        "/v1/eras/-x",
        "/v1/poem-types/HURR",
        "/v1/poets/%D8%AD%D8%A8%D9%8A%D8%A8",
        "/v1/poems/facets",
        "/v1/poems/facets?poet=abc",
        "/v1/poems/facets?poet=yoFB&poet=abCD",
        "/v1/poems/facets?poet[]=yoFB",
        "/v1/poems/facets?poet=yoFB&meter=ALTAWIL",
        "/v1/poems/facets?poet=yoFB&rhyme[x]=meem",
    ] {
        let sent = send(app_with(&es), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::BAD_REQUEST, "{path}");
        let body = sent.json();
        assert_eq!(body["code"], "BAD_REQUEST");
        assert_eq!(body["title"], "Bad request");
        assert_eq!(body["detail"], "Input validation failed");
        assert_eq!(body["type"], "https://qafiyah.com/errors/bad-request");
    }
    assert!(es.requests().await.is_empty());
}

#[tokio::test]
async fn every_problem_has_the_six_members_in_contract_order() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let sent = send(app_with(&es), request("GET", "/v1/poems/abc")).await;
    let positions: Vec<usize> = [
        "\"type\"",
        "\"title\"",
        "\"status\"",
        "\"code\"",
        "\"instance\"",
        "\"detail\"",
    ]
    .iter()
    .map(|member| {
        sent.body
            .find(member)
            .unwrap_or_else(|| panic!("{member} in {}", sent.body))
    })
    .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "{}",
        sent.body
    );
    assert_eq!(sent.json().as_object().expect("an object").len(), 6);
}
