use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;

use crate::accounts::users::{Identity, Profile, UpsertError, UserRepository};
use crate::domain::StoreError;
use crate::http_tests::{app_with, empty_hits};
use crate::test_support::{FakeEs, request, send, state};

fn keyed(method: &str, path: &str, key: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("x-api-key", key)
        .header("content-type", "application/json")
        .body(Body::empty())
        .expect("a well-formed test request")
}

const ACCOUNT_ROUTES: [(&str, &str); 8] = [
    ("POST", "/account/users"),
    ("GET", "/account/users/1/keys"),
    ("POST", "/account/keys"),
    ("POST", "/account/keys/revoke"),
    ("POST", "/account/sessions"),
    ("DELETE", "/account/sessions"),
    ("GET", "/account/sessions/abc"),
    ("DELETE", "/account/sessions/abc"),
];

#[tokio::test]
async fn without_the_internal_key_every_account_route_is_unauthorized_at_its_full_path() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let keys = [
        None,
        Some(""),
        Some("wrong"),
        Some("full"),
        Some("mcp"),
        Some("qaf_0123456789abcdef0123456789abcdef"),
        Some("Internal"),
        Some("internal "),
        Some(" internal"),
        Some("internalx"),
        Some("interna"),
    ];
    for (method, path) in ACCOUNT_ROUTES {
        for key in keys {
            let mut builder = Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json");
            if let Some(key) = key {
                builder = builder.header("x-api-key", key);
            }
            let body = Body::from(json!({ "user_id": 1, "key_id": 1 }).to_string());
            let sent = send(app_with(&es), builder.body(body).expect("a request")).await;
            assert_eq!(
                sent.status,
                StatusCode::UNAUTHORIZED,
                "{method} {path} {key:?}"
            );
            assert_eq!(sent.header("cache-control"), Some("no-store"));
            let problem = sent.json();
            assert_eq!(problem["code"], "UNAUTHORIZED");
            assert_eq!(
                problem["instance"], path,
                "the instance must be the full path"
            );
        }
    }
}

#[tokio::test]
async fn the_internal_key_in_a_query_string_or_another_header_is_not_accepted() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    for (method, path) in ACCOUNT_ROUTES {
        for request in [
            Request::builder()
                .method(method)
                .uri(format!("{path}?api_key=internal"))
                .body(Body::empty()),
            Request::builder()
                .method(method)
                .uri(path)
                .header("authorization", "Bearer internal")
                .body(Body::empty()),
        ] {
            let sent = send(app_with(&es), request.expect("a request")).await;
            assert_eq!(sent.status, StatusCode::UNAUTHORIZED, "{method} {path}");
        }
    }
}

#[tokio::test]
async fn with_the_internal_key_the_guard_lets_the_request_through_and_forbids_storing_it() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let sent = send(
        app_with(&es),
        keyed("GET", "/account/sessions/not-base64!!", "internal"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::UNAUTHORIZED);
    assert_eq!(sent.json()["instance"], "/account/sessions/not-base64!!");
    assert_eq!(sent.header("cache-control"), Some("no-store"));
}

#[tokio::test]
async fn account_routes_carry_no_rate_limit_headers_and_no_etag() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let sent = send(
        app_with(&es),
        keyed("DELETE", "/account/sessions/not-base64!!", "internal"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::NO_CONTENT);
    assert!(sent.header("x-ratelimit-limit").is_none());
    assert!(sent.header("etag").is_none());
    assert_eq!(sent.header("cache-control"), Some("no-store"));
}

#[tokio::test]
async fn an_unknown_account_path_is_a_plain_404_that_reveals_nothing() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let sent = send(app_with(&es), request("GET", "/account/nope")).await;
    assert_eq!(sent.status, StatusCode::NOT_FOUND);
    assert_eq!(sent.json()["detail"], "No route matches GET /account/nope");
    assert_eq!(sent.header("cache-control"), Some("no-store"));
}

#[tokio::test]
async fn the_wildcard_cors_origin_is_stamped_on_account_responses_pinned_not_endorsed() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let sent = send(app_with(&es), request("GET", "/account/nope")).await;
    assert_eq!(sent.header("access-control-allow-origin"), Some("*"));
}

struct EmailTaken;

#[async_trait]
impl UserRepository for EmailTaken {
    async fn upsert(&self, _: &Identity, _: &str) -> Result<Profile, UpsertError> {
        Err(UpsertError::EmailTaken)
    }
    async fn find_or_create(&self, _: &str) -> Result<Profile, StoreError> {
        Err(StoreError::Database("not used here".into()))
    }
}

#[tokio::test]
async fn an_email_that_belongs_to_another_account_is_a_409_conflict() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let mut state = state(&es);
    state.users = Arc::new(EmailTaken);
    let identity = json!({
        "provider": "github",
        "provider_uid": "42",
        "email": "taken@example.test",
        "display_name": null,
        "avatar_url": null,
    });
    let sent = send(
        crate::app(state),
        Request::builder()
            .method("POST")
            .uri("/account/users")
            .header("x-api-key", "internal")
            .header("content-type", "application/json")
            .body(Body::from(identity.to_string()))
            .expect("a request"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::CONFLICT);
    assert_eq!(sent.json()["code"], "EMAIL_TAKEN");
}
