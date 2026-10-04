use axum::Router;
use axum::response::Response;
use axum::routing::get;
use serde::Deserialize;

use crate::constants::{
    GITHUB_AVATARS_URL, GITHUB_DB_DUMPS_URL, GITHUB_URL, NO_STORE_CACHE_CONTROL, PROD_SITE_URL,
    RAAQIM_URL, TELEGRAM_URL, X_INTENT_TWEET_URL, X_PROFILE_URL,
};
use crate::error::{AppError, RouteProblem};
use crate::extract::SafeQuery;
use crate::routes::redirect;
use crate::state::AppState;

fn encode_uri_component(raw: &str) -> String {
    let mut encoded = String::with_capacity(raw.len());
    for byte in raw.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => encoded.push(char::from(byte)),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn share_url(path: Option<&str>) -> Option<String> {
    let path = path?;
    if !path.starts_with('/') || path.starts_with("//") {
        return None;
    }
    let without_query = path.split(['?', '#']).next().unwrap_or(path);
    let rest = without_query.strip_prefix("/poems/")?;
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    let poem_url = format!("{PROD_SITE_URL}{without_query}");
    Some(format!(
        "{X_INTENT_TWEET_URL}?url={}",
        encode_uri_component(&poem_url)
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShareParams {
    path: Option<String>,
}

async fn x_share(SafeQuery(params): SafeQuery<ShareParams>) -> Result<Response, AppError> {
    share_url(params.path.as_deref())
        .map(|url| redirect(&url, Some(NO_STORE_CACHE_CONTROL)))
        .ok_or_else(|| {
            RouteProblem::bad_request(
                "Invalid or missing ?path (expected an internal /poems/<slug> path)",
            )
            .into()
        })
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/go/x",
            get(|| async { redirect(X_PROFILE_URL, Some(NO_STORE_CACHE_CONTROL)) }),
        )
        .route(
            "/go/telegram",
            get(|| async { redirect(TELEGRAM_URL, Some(NO_STORE_CACHE_CONTROL)) }),
        )
        .route(
            "/go/github",
            get(|| async { redirect(GITHUB_URL, Some(NO_STORE_CACHE_CONTROL)) }),
        )
        .route(
            "/go/db",
            get(|| async { redirect(GITHUB_DB_DUMPS_URL, Some(NO_STORE_CACHE_CONTROL)) }),
        )
        .route(
            "/go/avatars",
            get(|| async { redirect(GITHUB_AVATARS_URL, Some(NO_STORE_CACHE_CONTROL)) }),
        )
        .route(
            "/go/raaqim",
            get(|| async { redirect(RAAQIM_URL, Some(NO_STORE_CACHE_CONTROL)) }),
        )
        .route("/go/x-share", get(x_share))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shares_only_same_origin_poem_paths() {
        assert_eq!(
            share_url(Some("/poems/TnKK")).unwrap(),
            "https://x.com/intent/tweet?url=https%3A%2F%2Fqafiyah.com%2Fpoems%2FTnKK"
        );
        for path in [
            None,
            Some(""),
            Some("/poets/TnKK"),
            Some("/poems/"),
            Some("/poems/a/b"),
            Some("https://evil.test/poems/TnKK"),
            Some("//evil.test/poems/TnKK"),
        ] {
            assert!(share_url(path).is_none(), "should refuse {path:?}");
        }
    }
}
