use axum::http::StatusCode;

use crate::constants::{
    GITHUB_AVATARS_URL, GITHUB_DB_DUMPS_URL, GITHUB_URL, RAAQIM_URL, TELEGRAM_URL, X_PROFILE_URL,
};
use crate::http_tests::{app_with, empty_hits};
use crate::test_support::{FakeEs, request, send};

const INTENT: &str = "https://x.com/intent/tweet?url=https%3A%2F%2Fqafiyah.com%2Fpoems%2FTnKK";

async fn es() -> FakeEs {
    FakeEs::serving(StatusCode::OK, empty_hits()).await
}

#[tokio::test]
async fn every_fixed_go_link_is_an_uncached_redirect() {
    let es = es().await;
    for (path, target) in [
        ("/v1/go/x", X_PROFILE_URL),
        ("/v1/go/telegram", TELEGRAM_URL),
        ("/v1/go/github", GITHUB_URL),
        ("/v1/go/db", GITHUB_DB_DUMPS_URL),
        ("/v1/go/avatars", GITHUB_AVATARS_URL),
        ("/v1/go/raaqim", RAAQIM_URL),
    ] {
        let sent = send(app_with(&es), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::FOUND, "{path}");
        assert_eq!(sent.header("location"), Some(target), "{path}");
        assert_eq!(sent.header("cache-control"), Some("no-store"), "{path}");
        assert!(sent.body.is_empty());
    }
}

#[tokio::test]
async fn the_share_link_wraps_a_poem_path_into_the_intent_url() {
    let es = es().await;
    let sent = send(
        app_with(&es),
        request("GET", "/v1/go/x-share?path=/poems/TnKK"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::FOUND);
    assert_eq!(sent.header("location"), Some(INTENT));
    assert_eq!(sent.header("cache-control"), Some("no-store"));
}

#[tokio::test]
async fn the_share_link_refuses_anything_but_a_poem_path() {
    let es = es().await;
    for query in [
        "",
        "path=",
        "path=/poets/TnKK",
        "path=//evil.test/poems/TnKK",
        "path=https://evil.test/poems/TnKK",
        "path=/poems/a/b",
    ] {
        let sent = send(
            app_with(&es),
            request("GET", &format!("/v1/go/x-share?{query}")),
        )
        .await;
        assert_eq!(sent.status, StatusCode::BAD_REQUEST, "{query}");
        let body = sent.json();
        assert_eq!(body["code"], "BAD_REQUEST");
        assert_eq!(
            body["detail"],
            "Invalid or missing ?path (expected an internal /poems/<slug> path)"
        );
    }
}

#[tokio::test]
async fn the_share_link_drops_a_query_or_fragment_from_the_path() {
    let es = es().await;
    let sent = send(
        app_with(&es),
        request("GET", "/v1/go/x-share?path=/poems/TnKK%3Fx%3D1%23top"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::FOUND);
    assert_eq!(sent.header("location"), Some(INTENT));
}

#[tokio::test]
async fn the_share_link_refuses_a_second_path_and_unknown_parameters_by_name() {
    let es = es().await;
    for (query, detail) in [
        (
            "path=/poems/TnKK&path=/poets/zzzz",
            "Invalid query parameter `path`: unsupported value",
        ),
        (
            "path[]=/poems/TnKK",
            "Invalid query parameter `path[]`: unknown field `path[]`, expected `path`",
        ),
    ] {
        let sent = send(
            app_with(&es),
            request("GET", &format!("/v1/go/x-share?{query}")),
        )
        .await;
        assert_eq!(sent.status, StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(sent.json()["detail"], detail, "{query}");
    }
}

#[tokio::test]
async fn the_share_link_accepts_a_non_slug_segment_which_is_pinned_not_endorsed() {
    let es = es().await;
    let sent = send(
        app_with(&es),
        request("GET", "/v1/go/x-share?path=/poems/.."),
    )
    .await;
    assert_eq!(sent.status, StatusCode::FOUND);
    assert_eq!(
        sent.header("location"),
        Some("https://x.com/intent/tweet?url=https%3A%2F%2Fqafiyah.com%2Fpoems%2F..")
    );
}
