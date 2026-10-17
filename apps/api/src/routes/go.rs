use axum::Router;
use axum::response::Response;
use axum::routing::get;
use serde::Deserialize;

use crate::constants::{
    FACEBOOK_SHARE_URL, GITHUB_AVATARS_URL, GITHUB_DB_DUMPS_URL, GITHUB_URL, LINKEDIN_SHARE_URL,
    NO_STORE_CACHE_CONTROL, PROD_SITE_URL, RAAQIM_URL, REDDIT_SHARE_URL, TELEGRAM_SHARE_URL,
    TELEGRAM_URL, WHATSAPP_SHARE_URL, X_INTENT_TWEET_URL, X_PROFILE_URL,
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

fn page_url(path: Option<&str>) -> Option<String> {
    let path = path?;
    if !path.starts_with('/') || path.starts_with("//") {
        return None;
    }
    let without_query = path.split(['?', '#']).next().unwrap_or(path);
    let rest = without_query
        .strip_prefix("/poems/")
        .or_else(|| without_query.strip_prefix("/poets/"))?;
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    Some(format!("{PROD_SITE_URL}{without_query}"))
}

enum ShareNetwork {
    X,
    Facebook,
    WhatsApp,
    Telegram,
    Reddit,
    Linkedin,
}

impl ShareNetwork {
    fn intent_url(self, page_url: &str) -> String {
        let encoded = encode_uri_component(page_url);
        match self {
            Self::X => format!("{X_INTENT_TWEET_URL}?url={encoded}"),
            Self::Facebook => format!("{FACEBOOK_SHARE_URL}?u={encoded}"),
            Self::WhatsApp => format!("{WHATSAPP_SHARE_URL}?text={encoded}"),
            Self::Telegram => format!("{TELEGRAM_SHARE_URL}?url={encoded}"),
            Self::Reddit => format!("{REDDIT_SHARE_URL}?url={encoded}"),
            Self::Linkedin => format!("{LINKEDIN_SHARE_URL}?url={encoded}"),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShareParams {
    path: Option<String>,
}

async fn share(
    network: ShareNetwork,
    SafeQuery(params): SafeQuery<ShareParams>,
) -> Result<Response, AppError> {
    page_url(params.path.as_deref())
        .map(|url| redirect(&network.intent_url(&url), Some(NO_STORE_CACHE_CONTROL)))
        .ok_or_else(|| {
            RouteProblem::bad_request(
                "Invalid or missing ?path (expected an internal /poems/<slug> or /poets/<slug> path)",
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
        .route(
            "/go/x-share",
            get(|query: SafeQuery<ShareParams>| share(ShareNetwork::X, query)),
        )
        .route(
            "/go/facebook-share",
            get(|query: SafeQuery<ShareParams>| share(ShareNetwork::Facebook, query)),
        )
        .route(
            "/go/whatsapp-share",
            get(|query: SafeQuery<ShareParams>| share(ShareNetwork::WhatsApp, query)),
        )
        .route(
            "/go/telegram-share",
            get(|query: SafeQuery<ShareParams>| share(ShareNetwork::Telegram, query)),
        )
        .route(
            "/go/reddit-share",
            get(|query: SafeQuery<ShareParams>| share(ShareNetwork::Reddit, query)),
        )
        .route(
            "/go/linkedin-share",
            get(|query: SafeQuery<ShareParams>| share(ShareNetwork::Linkedin, query)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shares_only_same_origin_poem_and_poet_paths() {
        assert_eq!(
            page_url(Some("/poems/TnKK")).unwrap(),
            "https://qafiyah.com/poems/TnKK"
        );
        assert_eq!(
            page_url(Some("/poets/mutanabbi")).unwrap(),
            "https://qafiyah.com/poets/mutanabbi"
        );
        for path in [
            None,
            Some(""),
            Some("/poems/"),
            Some("/poets/"),
            Some("/poems/a/b"),
            Some("/poets/a/b"),
            Some("poems/TnKK"),
            Some("https://evil.test/poems/TnKK"),
            Some("//evil.test/poems/TnKK"),
            Some("/about"),
        ] {
            assert!(page_url(path).is_none(), "should refuse {path:?}");
        }
    }

    #[test]
    fn drops_a_query_or_fragment_from_the_shared_path() {
        assert_eq!(
            page_url(Some("/poems/TnKK?x=1#top")).unwrap(),
            "https://qafiyah.com/poems/TnKK"
        );
    }

    #[test]
    fn accepts_a_non_slug_segment_which_is_pinned_not_endorsed() {
        assert_eq!(
            page_url(Some("/poems/..")).unwrap(),
            "https://qafiyah.com/poems/.."
        );
    }

    #[test]
    fn builds_each_network_intent_url_from_the_page_url() {
        let page = "https://qafiyah.com/poems/TnKK";
        let encoded = "https%3A%2F%2Fqafiyah.com%2Fpoems%2FTnKK";
        assert_eq!(
            ShareNetwork::X.intent_url(page),
            format!("https://x.com/intent/tweet?url={encoded}")
        );
        assert_eq!(
            ShareNetwork::Facebook.intent_url(page),
            format!("https://www.facebook.com/sharer/sharer.php?u={encoded}")
        );
        assert_eq!(
            ShareNetwork::WhatsApp.intent_url(page),
            format!("https://wa.me/?text={encoded}")
        );
        assert_eq!(
            ShareNetwork::Telegram.intent_url(page),
            format!("https://t.me/share/url?url={encoded}")
        );
        assert_eq!(
            ShareNetwork::Reddit.intent_url(page),
            format!("https://www.reddit.com/submit?url={encoded}")
        );
        assert_eq!(
            ShareNetwork::Linkedin.intent_url(page),
            format!("https://www.linkedin.com/sharing/share-offsite/?url={encoded}")
        );
    }
}
