use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::constants::READ_CACHE_CONTROL;

const MAX_BODY: usize = 64 * 1024 * 1024;
const JSON: &str = "application/json";

fn fnv1a64_utf16(input: &str) -> String {
    const OFFSET: u64 = 14_695_981_039_346_656_037;
    const PRIME: u64 = 1_099_511_628_211;
    let mut hash = OFFSET;
    for unit in input.encode_utf16() {
        hash ^= u64::from(unit);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{hash:016x}")
}

fn weak_etag(body: &str) -> String {
    format!("W/\"{}\"", fnv1a64_utf16(body))
}

fn if_none_match_satisfied(header_value: Option<&str>, etag: &str) -> bool {
    let Some(value) = header_value else {
        return false;
    };
    if value.trim() == "*" {
        return true;
    }
    let strong_of = |v: &str| v.trim().trim_start_matches("W/").to_string();
    let target = strong_of(etag);
    value
        .split(',')
        .any(|candidate| strong_of(candidate) == target)
}

fn is_json(response: &Response) -> bool {
    response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains(JSON))
}

pub async fn layer(request: Request, next: Next) -> Response {
    let if_none_match = request
        .headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let response = next.run(request).await;

    if response.status().as_u16() >= 400 {
        return response;
    }
    if !is_json(&response) {
        return response;
    }

    let (mut parts, body) = response.into_parts();
    let Ok(bytes) = to_bytes(body, MAX_BODY).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return (parts, Body::from(bytes)).into_response();
    };

    let etag = weak_etag(text);
    if let Ok(value) = HeaderValue::from_str(&etag) {
        parts.headers.insert(header::ETAG, value);
    }
    parts.headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(READ_CACHE_CONTROL),
    );

    if if_none_match_satisfied(if_none_match.as_deref(), &etag) {
        let kept: HeaderMap = [header::ETAG, header::CACHE_CONTROL, header::VARY]
            .into_iter()
            .filter_map(|name| parts.headers.get(&name).cloned().map(|value| (name, value)))
            .collect();
        parts.headers = kept;
        parts.status = StatusCode::NOT_MODIFIED;
        return (parts, Body::empty()).into_response();
    }
    (parts, Body::from(bytes)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_utf16_code_units() {
        assert_eq!(fnv1a64_utf16(""), "cbf29ce484222325");
        assert_eq!(fnv1a64_utf16("a").len(), 16);
        assert_ne!(fnv1a64_utf16("عربي"), fnv1a64_utf16("عربى"));
    }

    #[test]
    fn a_weak_etag_wraps_the_hash() {
        assert_eq!(weak_etag(""), "W/\"cbf29ce484222325\"");
    }

    #[test]
    fn if_none_match_compares_weakly_and_honors_the_wildcard() {
        let etag = weak_etag("body");
        assert!(if_none_match_satisfied(Some(&etag), &etag));
        assert!(if_none_match_satisfied(
            Some(etag.trim_start_matches("W/")),
            &etag
        ));
        assert!(if_none_match_satisfied(
            Some(&format!("\"other\", {etag}")),
            &etag
        ));
        assert!(if_none_match_satisfied(Some(" * "), &etag));
        assert!(!if_none_match_satisfied(Some("\"other\""), &etag));
        assert!(!if_none_match_satisfied(None, &etag));
        assert!(!if_none_match_satisfied(Some(""), &etag));
    }

    #[tokio::test]
    async fn a_json_body_of_several_megabytes_is_passed_through_whole() {
        use axum::Router;
        use axum::middleware::from_fn;
        use axum::routing::get;
        use tower::ServiceExt;

        let large = format!("[\"{}\"]", "a".repeat(8 * 1024 * 1024));
        let expected = large.len();
        let app = Router::new()
            .route(
                "/",
                get(move || async move { ([(header::CONTENT_TYPE, JSON)], large) }),
            )
            .layer(from_fn(layer));
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("a request"),
            )
            .await
            .expect("a response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("a body");
        assert_eq!(bytes.len(), expected);
    }

    #[test]
    fn if_none_match_never_panics_on_arbitrary_input() {
        let mut rng = crate::test_support::Rng::new(4);
        let etag = weak_etag("body");
        for _ in 0..2_000 {
            let len = usize::try_from(rng.below(40)).expect("bounded length");
            let raw = String::from_utf8_lossy(&rng.bytes(len)).into_owned();
            let _ = if_none_match_satisfied(Some(&raw), &etag);
        }
    }
}
