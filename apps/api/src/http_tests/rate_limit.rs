use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderValue, Request, StatusCode};

use crate::auth::Keys;
use crate::constants::VISITOR_REQUESTS;
use crate::http_tests::empty_hits;
use crate::test_support::{FakeEs, request, send, state_with};

fn app(es: &FakeEs, anon: u32) -> axum::Router {
    crate::app(state_with(
        es,
        Keys::new(Some("internal".into()), Some("full".into())),
        anon,
    ))
}

fn from(path: &str, ip: &str, key: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().uri(path).header("cf-connecting-ip", ip);
    if let Some(key) = key {
        builder = builder.header("x-api-key", key);
    }
    builder.body(Body::empty()).expect("a request")
}

fn from_peer(peer: &str, claimed_ip: &str, key: Option<&str>) -> Request<Body> {
    let mut request = from("/v1/openapi.json", claimed_ip, key);
    let address: SocketAddr = format!("{peer}:40000").parse().expect("a socket address");
    request.extensions_mut().insert(ConnectInfo(address));
    request
}

fn unforwarded(path: &str, key: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header("x-api-key", key)
        .body(Body::empty())
        .expect("a request")
}

#[tokio::test]
async fn anonymous_callers_are_counted_per_address_and_refused_at_the_ceiling() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let app = app(&es, 2);
    let first = send(app.clone(), from("/v1/openapi.json", "203.0.113.1", None)).await;
    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(first.header("x-ratelimit-limit"), Some("2"));
    assert_eq!(first.header("x-ratelimit-remaining"), Some("1"));
    assert!(first.header("x-ratelimit-reset").is_some());
    let second = send(app.clone(), from("/v1/openapi.json", "203.0.113.1", None)).await;
    assert_eq!(second.header("x-ratelimit-remaining"), Some("0"));
    let third = send(app.clone(), from("/v1/openapi.json", "203.0.113.1", None)).await;
    assert_eq!(third.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        third.header("content-type"),
        Some("application/problem+json")
    );
    assert_eq!(third.header("cache-control"), Some("no-store"));
    assert_eq!(third.header("x-ratelimit-remaining"), Some("0"));
    let retry: i64 = third
        .header("retry-after")
        .expect("retry-after")
        .parse()
        .expect("an integer");
    assert!((1..=3600).contains(&retry));
    let body = third.json();
    assert_eq!(body["code"], "TOO_MANY_REQUESTS");
    assert_eq!(body["instance"], "/v1/openapi.json");
    let other = send(app, from("/v1/openapi.json", "203.0.113.2", None)).await;
    assert_eq!(
        other.status,
        StatusCode::OK,
        "another address has its own bucket"
    );
}

#[tokio::test]
async fn an_untrusted_peer_is_counted_on_its_own_address_whatever_address_it_claims() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let app = app(&es, 1);
    let first = send(app.clone(), from_peer("198.51.100.7", "203.0.113.10", None)).await;
    assert_eq!(first.status, StatusCode::OK);
    let second = send(app, from_peer("198.51.100.7", "203.0.113.11", None)).await;
    assert_eq!(
        second.status,
        StatusCode::TOO_MANY_REQUESTS,
        "a different claimed address must not buy a fresh bucket"
    );
}

#[tokio::test]
async fn the_full_key_and_the_sites_own_calls_bypass_the_limiter_and_carry_no_headers() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let app = app(&es, 1);
    for _ in 0..3 {
        for (label, request) in [
            (
                "full key with an address",
                from("/v1/openapi.json", "203.0.113.3", Some("full")),
            ),
            (
                "internal key with no forwarded address",
                unforwarded("/v1/openapi.json", "internal"),
            ),
            (
                "internal key from an untrusted peer",
                from_peer("127.0.0.1", "203.0.113.3", Some("internal")),
            ),
        ] {
            let sent = send(app.clone(), request).await;
            assert_eq!(sent.status, StatusCode::OK, "{label}");
            assert!(sent.header("x-ratelimit-limit").is_none(), "{label}");
        }
    }
}

#[tokio::test]
async fn a_visitor_the_website_forwards_is_counted_per_slash_64_at_the_visitor_ceiling() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let app = app(&es, 1);
    let ceiling = VISITOR_REQUESTS.to_string();
    for (ip, remaining) in [
        ("2001:db8:7:7::1", VISITOR_REQUESTS - 1),
        ("2001:db8:7:7::2", VISITOR_REQUESTS - 2),
        ("2001:db8:7:8::1", VISITOR_REQUESTS - 1),
    ] {
        let sent = send(app.clone(), from("/v1/openapi.json", ip, Some("internal"))).await;
        assert_eq!(sent.status, StatusCode::OK, "{ip}");
        assert_eq!(
            sent.header("x-ratelimit-limit"),
            Some(ceiling.as_str()),
            "{ip}"
        );
        assert_eq!(
            sent.header("x-ratelimit-remaining"),
            Some(remaining.to_string().as_str()),
            "{ip}"
        );
    }
}

#[tokio::test]
async fn a_visitor_forwarded_by_the_trusted_web_container_is_counted() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let app = app(&es, 1);
    let sent = send(
        app,
        from_peer("172.26.0.5", "2001:db8:9:9::1", Some("internal")),
    )
    .await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(
        sent.header("x-ratelimit-limit"),
        Some(VISITOR_REQUESTS.to_string().as_str())
    );
    assert_eq!(
        sent.header("x-ratelimit-remaining"),
        Some((VISITOR_REQUESTS - 1).to_string().as_str())
    );
}

#[tokio::test]
async fn a_forwarded_visitor_never_touches_the_key_cache_even_with_a_key_shaped_internal_key() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let internal = "qaf_abcdefghijklmnopqrstuvwxyz012345";
    let state = state_with(
        &es,
        Keys::new(Some(internal.into()), Some("full".into())),
        1,
    );
    let app = crate::app(state.clone());
    let sent = send(
        app,
        from("/v1/openapi.json", "2001:db8:9:9::1", Some(internal)),
    )
    .await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(
        sent.header("x-ratelimit-limit"),
        Some(VISITOR_REQUESTS.to_string().as_str())
    );
    assert!(state.key_cache.is_empty());
}

#[tokio::test]
async fn website_visitors_and_anonymous_callers_are_counted_apart() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let app = app(&es, 1);
    let first = send(app.clone(), from("/v1/openapi.json", "203.0.113.40", None)).await;
    assert_eq!(first.status, StatusCode::OK);
    let second = send(app.clone(), from("/v1/openapi.json", "203.0.113.40", None)).await;
    assert_eq!(second.status, StatusCode::TOO_MANY_REQUESTS);
    let visitor = send(
        app,
        from("/v1/openapi.json", "203.0.113.40", Some("internal")),
    )
    .await;
    assert_eq!(visitor.status, StatusCode::OK);
    assert_eq!(
        visitor.header("x-ratelimit-limit"),
        Some(VISITOR_REQUESTS.to_string().as_str())
    );
}

#[tokio::test]
async fn a_keyed_caller_the_accounts_database_cannot_resolve_falls_back_to_anonymous_pinned() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let app = app(&es, 1);
    let unknown = "qaf_00000000000000000000000000000000";
    let first = send(
        app.clone(),
        from("/v1/openapi.json", "203.0.113.4", Some(unknown)),
    )
    .await;
    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(first.header("x-ratelimit-limit"), Some("1"));
    let second = send(app, from("/v1/openapi.json", "203.0.113.4", Some(unknown))).await;
    assert_eq!(second.status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn a_malformed_key_is_anonymous_and_never_touches_the_key_cache() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let state = state_with(
        &es,
        Keys::new(Some("internal".into()), Some("full".into())),
        5,
    );
    let app = crate::app(state.clone());
    for key in [
        "qaf_bad",
        "qaf_",
        "qaf_short",
        "qaf_a-bcdefghijklmnopqrstuvwxyz0123456789",
    ] {
        let sent = send(
            app.clone(),
            from("/v1/openapi.json", "203.0.113.6", Some(key)),
        )
        .await;
        assert_eq!(sent.status, StatusCode::OK, "{key}");
        assert_eq!(sent.header("x-ratelimit-limit"), Some("5"), "{key}");
    }
    assert!(state.key_cache.is_empty());
}

#[tokio::test]
async fn a_non_utf8_key_header_is_treated_as_anonymous() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let app = app(&es, 5);
    let request = Request::builder()
        .uri("/v1/openapi.json")
        .header(
            "x-api-key",
            HeaderValue::from_bytes(&[0xff, 0xfe]).expect("opaque bytes"),
        )
        .body(Body::empty())
        .expect("a request");
    let sent = send(app, request).await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(sent.header("x-ratelimit-limit"), Some("5"));
}

#[tokio::test]
async fn callers_without_a_resolvable_address_share_one_bucket() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let app = app(&es, 1);
    assert_eq!(
        send(app.clone(), request("GET", "/v1/openapi.json"))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        send(app, request("GET", "/v1/openapi.json")).await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn the_health_probe_and_the_account_api_sit_outside_the_limiter() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let app = app(&es, 1);
    for _ in 0..3 {
        assert_eq!(
            send(app.clone(), request("GET", "/healthz")).await.status,
            StatusCode::OK
        );
    }
    let sent = send(app, request("GET", "/account/nope")).await;
    assert_ne!(sent.status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn a_refused_request_does_not_touch_the_backend() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let app = app(&es, 1);
    let _result = send(app.clone(), from("/v1/search?q=x", "203.0.113.5", None)).await;
    let refused = send(app, from("/v1/search?q=x", "203.0.113.5", None)).await;
    assert_eq!(refused.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        es.requests().await.len(),
        2,
        "the first request asked both indices, the refused one asked nothing"
    );
}
