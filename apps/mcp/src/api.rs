use std::net::IpAddr;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::{LOCATION, RETRY_AFTER};
use url::Url;

use crate::constants::{API_KEY_HEADER, API_V1_PREFIX, CF_CONNECTING_IP_HEADER};

#[derive(Clone)]
pub struct Api {
    client: reqwest::Client,
    base: Url,
    key: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Reply {
    Found(String),
    Moved(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    RateLimited { retry_after: Option<u64> },
    BadRequest(Option<String>),
    NotFound,
    Unavailable,
}

impl Api {
    pub fn new(base: &str, key: Option<String>, timeout: Duration) -> Result<Self, String> {
        let base = Url::parse(base).map_err(|e| format!("INTERNAL_API_URL is not a URL: {e}"))?;
        if base.cannot_be_a_base() {
            return Err("INTERNAL_API_URL cannot be a base URL".into());
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(timeout)
            .build()
            .map_err(|e| format!("the API client could not be built: {e}"))?;
        Ok(Self { client, base, key })
    }

    fn url(&self, segments: &[&str], query: &[(&str, String)]) -> Result<Url, Failure> {
        let mut url = self.base.clone();
        url.path_segments_mut()
            .map_err(|()| Failure::Unavailable)?
            .pop_if_empty()
            .push(API_V1_PREFIX.trim_start_matches('/'))
            .extend(segments);
        if !query.is_empty() {
            url.query_pairs_mut()
                .extend_pairs(query.iter().map(|(name, value)| (*name, value.as_str())));
        }
        Ok(url)
    }

    pub async fn get(
        &self,
        segments: &[&str],
        query: &[(&str, String)],
        caller: Option<IpAddr>,
    ) -> Result<Reply, Failure> {
        let mut request = self.client.get(self.url(segments, query)?);
        if let Some(key) = &self.key {
            request = request.header(API_KEY_HEADER, key);
        }
        if let Some(caller) = caller {
            request = request.header(CF_CONNECTING_IP_HEADER, caller.to_string());
        }
        let response = request.send().await.map_err(|error| {
            if error.is_builder() {
                Failure::BadRequest(None)
            } else {
                Failure::Unavailable
            }
        })?;
        match response.status() {
            StatusCode::OK => response
                .text()
                .await
                .map(Reply::Found)
                .map_err(|_| Failure::Unavailable),
            StatusCode::MOVED_PERMANENTLY => response
                .headers()
                .get(LOCATION)
                .and_then(|location| location.to_str().ok())
                .map(|location| Reply::Moved(location.to_string()))
                .ok_or(Failure::Unavailable),
            StatusCode::BAD_REQUEST => {
                let detail = response
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|problem| problem.get("detail")?.as_str().map(str::to_string));
                Err(Failure::BadRequest(detail))
            }
            StatusCode::NOT_FOUND => Err(Failure::NotFound),
            StatusCode::TOO_MANY_REQUESTS => Err(Failure::RateLimited {
                retry_after: response
                    .headers()
                    .get(RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.trim().parse().ok()),
            }),
            _ => Err(Failure::Unavailable),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::time::Duration;

    use axum::http::StatusCode;

    use super::*;
    use crate::test_support::FakeApi;

    type Case = (
        StatusCode,
        &'static [(&'static str, &'static str)],
        &'static str,
        Failure,
    );

    fn api(url: &str, timeout: Duration) -> Api {
        Api::new(url, Some("mcp-key".into()), timeout).expect("an API client")
    }

    fn caller() -> Option<IpAddr> {
        Some("203.0.113.7".parse().expect("an address"))
    }

    fn query_of(path_and_query: &str) -> Vec<(String, String)> {
        let query = path_and_query.split_once('?').map_or("", |(_, q)| q);
        url::form_urlencoded::parse(query.as_bytes())
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect()
    }

    #[tokio::test]
    async fn a_call_sends_the_key_and_the_caller_address_and_nothing_else() {
        let fake = FakeApi::serving(StatusCode::OK, &[], "{}").await;
        let reply = api(&fake.url, Duration::from_secs(2))
            .get(&["poems", "Abcd"], &[], caller())
            .await;
        assert_eq!(reply, Ok(Reply::Found("{}".into())));
        let seen = fake.requests().await;
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].path_and_query, "/v1/poems/Abcd");
        assert_eq!(
            seen[0].headers.get("x-api-key").map(|v| v.as_bytes()),
            Some(&b"mcp-key"[..])
        );
        assert_eq!(
            seen[0]
                .headers
                .get("cf-connecting-ip")
                .map(|v| v.as_bytes()),
            Some(&b"203.0.113.7"[..])
        );
        assert!(seen[0].headers.get("x-forwarded-for").is_none());
    }

    #[tokio::test]
    async fn a_call_without_a_caller_sends_no_address() {
        let fake = FakeApi::serving(StatusCode::OK, &[], "{}").await;
        let _reply = api(&fake.url, Duration::from_secs(2))
            .get(&["feelings"], &[], None)
            .await;
        let seen = fake.requests().await;
        assert!(seen[0].headers.get("cf-connecting-ip").is_none());
    }

    #[tokio::test]
    async fn a_segment_cannot_change_the_route() {
        let fake = FakeApi::serving(StatusCode::OK, &[], "{}").await;
        let _reply = api(&fake.url, Duration::from_secs(2))
            .get(&["poems", "a/b"], &[], None)
            .await;
        assert_eq!(fake.requests().await[0].path_and_query, "/v1/poems/a%2Fb");
    }

    #[tokio::test]
    async fn arabic_query_text_reaches_the_api_unchanged() {
        let fake = FakeApi::serving(StatusCode::OK, &[], "{}").await;
        let text = "قِفا نَبكِ";
        let _reply = api(&fake.url, Duration::from_secs(2))
            .get(&["search"], &[("q", text.into())], None)
            .await;
        let seen = fake.requests().await;
        assert_eq!(
            query_of(&seen[0].path_and_query),
            vec![("q".to_string(), text.to_string())]
        );
    }

    #[tokio::test]
    async fn repeated_parameters_stay_repeated() {
        let fake = FakeApi::serving(StatusCode::OK, &[], "{}").await;
        let _reply = api(&fake.url, Duration::from_secs(2))
            .get(
                &["poems"],
                &[("era", "jahili".into()), ("era", "abbasi".into())],
                None,
            )
            .await;
        assert_eq!(
            fake.requests().await[0].path_and_query,
            "/v1/poems?era=jahili&era=abbasi"
        );
    }

    #[tokio::test]
    async fn a_url_too_long_to_send_is_a_bad_request_and_is_never_sent() {
        let fake = FakeApi::serving(StatusCode::OK, &[], "{}").await;
        let reply = api(&fake.url, Duration::from_secs(2))
            .get(&["search"], &[("q", "ب".repeat(12_000))], None)
            .await;
        assert_eq!(reply, Err(Failure::BadRequest(None)));
        assert!(fake.requests().await.is_empty());
    }

    #[tokio::test]
    async fn a_301_is_returned_as_moved_and_not_followed() {
        let fake = FakeApi::serving(
            StatusCode::MOVED_PERMANENTLY,
            &[("location", "/v1/poems/Wxyz")],
            "",
        )
        .await;
        let reply = api(&fake.url, Duration::from_secs(2))
            .get(&["poems", "Abcd"], &[], None)
            .await;
        assert_eq!(reply, Ok(Reply::Moved("/v1/poems/Wxyz".into())));
        assert_eq!(fake.requests().await.len(), 1);
    }

    #[tokio::test]
    async fn each_status_maps_to_its_failure() {
        let cases: [Case; 8] = [
            (
                StatusCode::TOO_MANY_REQUESTS,
                &[("retry-after", "30")],
                "",
                Failure::RateLimited {
                    retry_after: Some(30),
                },
            ),
            (
                StatusCode::TOO_MANY_REQUESTS,
                &[],
                "",
                Failure::RateLimited { retry_after: None },
            ),
            (
                StatusCode::BAD_REQUEST,
                &[],
                r#"{"detail":"page must be a positive integer"}"#,
                Failure::BadRequest(Some("page must be a positive integer".into())),
            ),
            (
                StatusCode::BAD_REQUEST,
                &[],
                "not json",
                Failure::BadRequest(None),
            ),
            (StatusCode::NOT_FOUND, &[], "", Failure::NotFound),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                &[],
                "",
                Failure::Unavailable,
            ),
            (
                StatusCode::SERVICE_UNAVAILABLE,
                &[],
                "",
                Failure::Unavailable,
            ),
            (StatusCode::MOVED_PERMANENTLY, &[], "", Failure::Unavailable),
        ];
        for (status, headers, body, failure) in cases {
            let fake = FakeApi::serving(status, headers, body).await;
            let reply = api(&fake.url, Duration::from_secs(2))
                .get(&["poems"], &[], None)
                .await;
            assert_eq!(reply, Err(failure), "{status} {headers:?}");
        }
        let teapot = FakeApi::serving(StatusCode::IM_A_TEAPOT, &[], "").await;
        assert_eq!(
            api(&teapot.url, Duration::from_secs(2))
                .get(&["poems"], &[], None)
                .await,
            Err(Failure::Unavailable)
        );
    }

    #[tokio::test]
    async fn a_slow_or_absent_api_is_unavailable() {
        let slow = FakeApi::slow(Duration::from_millis(500)).await;
        assert_eq!(
            api(&slow.url, Duration::from_millis(50))
                .get(&["poems"], &[], None)
                .await,
            Err(Failure::Unavailable)
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        drop(listener);
        assert_eq!(
            api(&format!("http://{address}"), Duration::from_secs(2))
                .get(&["poems"], &[], None)
                .await,
            Err(Failure::Unavailable)
        );
    }
}
