use std::time::{Duration, Instant};

use qafiyah_elasticsearch::Endpoint;
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::constants::ES_SEARCH_TIMEOUT_SECONDS;
use crate::domain::StoreError;
use crate::metrics::Metrics;

pub struct Es {
    endpoint: Endpoint,
    timeout: Duration,
    pub poems_alias: String,
    pub poets_alias: String,
    metrics: Metrics,
}

impl Es {
    pub fn new(url: &str, metrics: Metrics) -> Result<Self, String> {
        Self::with_timeout(url, Duration::from_secs(ES_SEARCH_TIMEOUT_SECONDS), metrics)
    }

    pub fn with_timeout(url: &str, timeout: Duration, metrics: Metrics) -> Result<Self, String> {
        let identity = qafiyah_elasticsearch::load().identity;
        Ok(Self {
            endpoint: Endpoint::new(url)?,
            timeout,
            poems_alias: identity.poems_alias,
            poets_alias: identity.poets_alias,
            metrics,
        })
    }

    pub async fn search<T: DeserializeOwned>(
        &self,
        index: &str,
        body: &Value,
    ) -> Result<T, StoreError> {
        let started = Instant::now();
        let outcome = self.send_search(index, body).await;
        self.metrics
            .observe_search(index, started.elapsed().as_secs_f64());
        outcome
    }

    async fn send_search<T: DeserializeOwned>(
        &self,
        index: &str,
        body: &Value,
    ) -> Result<T, StoreError> {
        let response = self
            .endpoint
            .request(Method::POST, &format!("/{index}/_search"))
            .timeout(self.timeout)
            .json(body)
            .send()
            .await
            .map_err(|cause| failed(index, &cause))?;
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|cause| failed(index, &cause))?;
        if !status.is_success() {
            let refusal = serde_json::from_slice::<Value>(&body)
                .ok()
                .and_then(|value| refusal_reason(&value));
            let message = match refusal {
                Some(reason) => format!("{index}: {status}: {reason}"),
                None => format!("{index}: {status}"),
            };
            return Err(match status {
                StatusCode::TOO_MANY_REQUESTS
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::GATEWAY_TIMEOUT => StoreError::Unavailable(message),
                _ => StoreError::Search(message),
            });
        }
        serde_json::from_slice(&body)
            .map_err(|cause| StoreError::Search(format!("{index}: {cause}")))
    }
}

fn failed(index: &str, cause: &reqwest::Error) -> StoreError {
    let message = format!("{index}: {}", with_causes(cause));
    if cause.is_timeout() || cause.is_connect() {
        StoreError::Unavailable(message)
    } else {
        StoreError::Search(message)
    }
}

fn with_causes(error: &(dyn std::error::Error + 'static)) -> String {
    std::iter::successors(Some(error), |level| level.source())
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}

fn refusal_reason(value: &Value) -> Option<String> {
    let error = value.get("error")?;
    error
        .get("root_cause")
        .and_then(Value::as_array)
        .and_then(|causes| causes.first())
        .and_then(|cause| cause.get("reason"))
        .or_else(|| error.get("reason"))
        .or(Some(error))
        .and_then(Value::as_str)
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_search_gives_up_on_an_elasticsearch_that_never_answers() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((socket, _)) = listener.accept().await {
                held.push(socket);
            }
        });

        let timeout = Duration::from_millis(250);
        let es = Es::with_timeout(&format!("http://{address}"), timeout, Metrics::default())
            .expect("an endpoint");
        let outcome = tokio::time::timeout(
            timeout + Duration::from_secs(5),
            es.search::<Value>("poems", &serde_json::json!({})),
        )
        .await;

        let result = outcome.expect("the search must give up on its own, not be rescued");
        assert!(
            matches!(&result, Err(StoreError::Unavailable(message)) if message.contains("timed out")),
            "{:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn a_search_that_cannot_connect_says_the_connection_was_refused() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        drop(listener);
        let es = Es::with_timeout(
            &format!("http://{address}"),
            Duration::from_secs(5),
            Metrics::default(),
        )
        .expect("an endpoint");

        let result = es.search::<Value>("poems", &serde_json::json!({})).await;

        assert!(
            matches!(&result, Err(StoreError::Unavailable(message)) if message.to_lowercase().contains("connection refused")),
            "{:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn a_search_whose_host_does_not_resolve_says_the_lookup_failed() {
        let es = Es::with_timeout(
            "http://qafiyah-no-such-host.invalid:9200",
            Duration::from_secs(10),
            Metrics::default(),
        )
        .expect("an endpoint");

        let result = es.search::<Value>("poems", &serde_json::json!({})).await;

        assert!(
            matches!(&result, Err(StoreError::Unavailable(message)) if message.contains("dns error")),
            "{:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn a_refused_search_carries_the_reason_elasticsearch_gave() {
        let es = crate::test_support::FakeEs::serving(
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "error": {
                    "root_cause": [{
                        "type": "illegal_argument_exception",
                        "reason": "no mapping found for `primaryId` in order to collapse on",
                    }],
                    "type": "search_phase_execution_exception",
                    "reason": "all shards failed",
                },
                "status": 400,
            }),
        )
        .await;
        let client = Es::with_timeout(&es.url, Duration::from_secs(2), Metrics::default())
            .expect("an endpoint");

        let result = client
            .search::<Value>("poems", &serde_json::json!({}))
            .await;

        assert!(
            matches!(&result, Err(StoreError::Search(message)) if message == "poems: 400 Bad Request: no mapping found for `primaryId` in order to collapse on"),
            "{:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn a_refused_search_whose_body_is_not_json_still_reports_its_status() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut request = vec![0u8; 4096];
                let _read = tokio::io::AsyncReadExt::read(&mut socket, &mut request).await;
                let page = "<html>bad gateway</html>";
                let response = format!(
                    "HTTP/1.1 502 Bad Gateway\r\ncontent-type: text/html\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{page}",
                    page.len()
                );
                let _written =
                    tokio::io::AsyncWriteExt::write_all(&mut socket, response.as_bytes()).await;
            }
        });
        let client = Es::with_timeout(
            &format!("http://{address}"),
            Duration::from_secs(2),
            Metrics::default(),
        )
        .expect("an endpoint");

        let result = client
            .search::<Value>("poems", &serde_json::json!({}))
            .await;

        assert!(
            matches!(&result, Err(StoreError::Search(message)) if message == "poems: 502 Bad Gateway"),
            "{:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn a_non_success_status_with_a_json_body_is_a_search_error() {
        let es = crate::test_support::FakeEs::serving(
            StatusCode::BAD_GATEWAY,
            serde_json::json!({ "error": "down" }),
        )
        .await;
        let client = Es::with_timeout(&es.url, Duration::from_secs(2), Metrics::default())
            .expect("an endpoint");
        let result = client
            .search::<Value>("poems", &serde_json::json!({}))
            .await;
        assert!(matches!(result, Err(StoreError::Search(message)) if message.contains("502")));
    }

    #[tokio::test]
    async fn a_busy_or_unreachable_elasticsearch_is_unavailable_not_a_search_error() {
        for status in [
            StatusCode::SERVICE_UNAVAILABLE,
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::GATEWAY_TIMEOUT,
        ] {
            let fake = crate::test_support::FakeEs::serving(
                status,
                serde_json::json!({ "error": "busy" }),
            )
            .await;
            let es = Es::new(&fake.url, Metrics::default()).expect("a fake endpoint");
            let result = es.search::<Value>("poems", &serde_json::json!({})).await;
            assert!(
                matches!(result, Err(StoreError::Unavailable(_))),
                "{status}"
            );
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        drop(listener);
        let es = Es::new(&format!("http://{address}"), Metrics::default()).expect("an endpoint");
        let result = es.search::<Value>("poems", &serde_json::json!({})).await;
        assert!(matches!(result, Err(StoreError::Unavailable(_))));
    }
}
