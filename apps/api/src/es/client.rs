use std::time::Duration;

use qafiyah_elasticsearch::Endpoint;
use reqwest::Method;
use serde_json::Value;

use crate::constants::ES_SEARCH_TIMEOUT_SECONDS;
use crate::error::AppError;

pub struct Es {
    endpoint: Endpoint,
    timeout: Duration,
    pub poems_alias: String,
    pub poets_alias: String,
}

impl Es {
    pub fn new(url: &str) -> Result<Self, String> {
        Self::with_timeout(url, Duration::from_secs(ES_SEARCH_TIMEOUT_SECONDS))
    }

    pub fn with_timeout(url: &str, timeout: Duration) -> Result<Self, String> {
        let identity = qafiyah_elasticsearch::load().identity;
        Ok(Self {
            endpoint: Endpoint::new(url)?,
            timeout,
            poems_alias: identity.poems_alias,
            poets_alias: identity.poets_alias,
        })
    }

    pub async fn search(&self, index: &str, body: &Value) -> Result<Value, AppError> {
        let response = self
            .endpoint
            .request(Method::POST, &format!("/{index}/_search"))
            .timeout(self.timeout)
            .json(body)
            .send()
            .await
            .map_err(|cause| AppError::Search(format!("{index}: {cause}")))?;
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|cause| AppError::Search(format!("{index}: {cause}")))?;
        if !status.is_success() {
            let refusal = serde_json::from_slice::<Value>(&body)
                .ok()
                .and_then(|value| refusal_reason(&value));
            return Err(AppError::Search(match refusal {
                Some(reason) => format!("{index}: {status}: {reason}"),
                None => format!("{index}: {status}"),
            }));
        }
        serde_json::from_slice(&body).map_err(|cause| AppError::Search(format!("{index}: {cause}")))
    }
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
        let es = Es::with_timeout(&format!("http://{address}"), timeout).expect("an endpoint");
        let outcome = tokio::time::timeout(
            timeout + Duration::from_secs(5),
            es.search("poems", &serde_json::json!({})),
        )
        .await;

        let result = outcome.expect("the search must give up on its own, not be rescued");
        assert!(matches!(result, Err(AppError::Search(_))));
    }

    #[tokio::test]
    async fn a_refused_search_carries_the_reason_elasticsearch_gave() {
        let es = crate::test_support::FakeEs::serving(
            axum::http::StatusCode::BAD_REQUEST,
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
        let client = Es::with_timeout(&es.url, Duration::from_secs(2)).expect("an endpoint");

        let result = client.search("poems", &serde_json::json!({})).await;

        assert!(
            matches!(&result, Err(AppError::Search(message)) if message == "poems: 400 Bad Request: no mapping found for `primaryId` in order to collapse on"),
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
        let client = Es::with_timeout(&format!("http://{address}"), Duration::from_secs(2))
            .expect("an endpoint");

        let result = client.search("poems", &serde_json::json!({})).await;

        assert!(
            matches!(&result, Err(AppError::Search(message)) if message == "poems: 502 Bad Gateway"),
            "{:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn a_non_success_status_with_a_json_body_is_a_search_error() {
        let es = crate::test_support::FakeEs::serving(
            axum::http::StatusCode::BAD_GATEWAY,
            serde_json::json!({ "error": "down" }),
        )
        .await;
        let client = Es::with_timeout(&es.url, Duration::from_secs(2)).expect("an endpoint");
        let result = client.search("poems", &serde_json::json!({})).await;
        assert!(matches!(result, Err(AppError::Search(message)) if message.contains("502")));
    }
}
