use std::time::Duration;

use async_trait::async_trait;
use qafiyah_elasticsearch::Endpoint;
use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde_json::{Value, json};

use crate::docs::Document;
use crate::error::IndexerError;
use crate::reindex::IndexStore;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const FORCE_MERGE_TIMEOUT: Duration = Duration::from_secs(1800);

pub(crate) struct Es {
    endpoint: Endpoint,
    timeout: Duration,
}

impl Es {
    pub(crate) fn new(url: &str) -> Result<Self, IndexerError> {
        Self::with_timeout(url, REQUEST_TIMEOUT)
    }

    fn with_timeout(url: &str, timeout: Duration) -> Result<Self, IndexerError> {
        Ok(Self {
            endpoint: Endpoint::new(url).map_err(IndexerError::Config)?,
            timeout,
        })
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.endpoint.request(method, path).timeout(self.timeout)
    }

    async fn send_json<T: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
    ) -> Result<(StatusCode, Value), IndexerError> {
        let mut req = self.request(method, path);
        if let Some(b) = body {
            req = req.json(b);
        }
        let res = req
            .send()
            .await
            .map_err(|e| IndexerError::Elasticsearch(format!("{path}: {e}")))?;
        let status = res.status();
        let value: Value = res.json().await.unwrap_or(Value::Null);
        Ok((status, value))
    }

    async fn expect_ok<T: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
    ) -> Result<Value, IndexerError> {
        let (status, value) = self.send_json(method, path, body).await?;
        if !status.is_success() {
            return Err(IndexerError::Elasticsearch(format!(
                "{path}: {status}: {value}"
            )));
        }
        Ok(value)
    }

    pub(crate) async fn index_exists(&self, index: &str) -> Result<bool, IndexerError> {
        let res = self
            .request(Method::HEAD, &format!("/{index}"))
            .send()
            .await
            .map_err(|e| IndexerError::Elasticsearch(format!("exists {index}: {e}")))?;
        Ok(res.status().is_success())
    }
}

#[async_trait]
impl IndexStore for Es {
    async fn list_indices_for_alias(&self, prefix: &str) -> Result<Vec<String>, IndexerError> {
        let path = format!("/_cat/indices/{prefix}*?format=json");
        let (status, value) = self.send_json::<()>(Method::GET, &path, None).await?;
        if status == StatusCode::NOT_FOUND {
            return Ok(vec![]);
        }
        if !status.is_success() {
            return Err(IndexerError::Elasticsearch(format!(
                "{path}: {status}: {value}"
            )));
        }
        Ok(index_names(&value))
    }

    async fn create_index(&self, index: &str, body: &Value) -> Result<(), IndexerError> {
        if self.index_exists(index).await? {
            return Ok(());
        }
        self.expect_ok(Method::PUT, &format!("/{index}"), Some(body))
            .await?;
        Ok(())
    }

    async fn put_refresh_interval(&self, index: &str, value: &str) -> Result<(), IndexerError> {
        let body = json!({ "refresh_interval": value });
        self.expect_ok(Method::PUT, &format!("/{index}/_settings"), Some(&body))
            .await?;
        Ok(())
    }

    async fn refresh(&self, index: &str) -> Result<(), IndexerError> {
        self.expect_ok::<()>(Method::POST, &format!("/{index}/_refresh"), None)
            .await?;
        Ok(())
    }

    async fn force_merge(&self, index: &str) -> Result<(), IndexerError> {
        let path = format!("/{index}/_forcemerge?max_num_segments=1");
        let res = self
            .endpoint
            .request(Method::POST, &path)
            .timeout(FORCE_MERGE_TIMEOUT)
            .send()
            .await
            .map_err(|e| IndexerError::Elasticsearch(format!("{path}: {e}")))?;
        let status = res.status();
        if !status.is_success() {
            let value: Value = res.json().await.unwrap_or(Value::Null);
            return Err(IndexerError::Elasticsearch(format!(
                "{path}: {status}: {value}"
            )));
        }
        Ok(())
    }

    async fn bulk(&self, index: &str, docs: &[Document]) -> Result<(), IndexerError> {
        let body = ndjson_body(index, docs)?;
        let res = self
            .request(Method::POST, "/_bulk")
            .header("Content-Type", "application/x-ndjson")
            .body(body)
            .send()
            .await
            .map_err(|e| IndexerError::Elasticsearch(format!("bulk: {e}")))?;
        let status = res.status();
        let value: Value = res.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            return Err(IndexerError::Elasticsearch(format!(
                "bulk: {status}: {value}"
            )));
        }
        if let Some(reason) = first_bulk_error(&value) {
            return Err(IndexerError::Elasticsearch(format!(
                "bulk errors: {reason}"
            )));
        }
        Ok(())
    }

    async fn swap_alias(
        &self,
        alias: &str,
        prefix: &str,
        to_index: &str,
    ) -> Result<(), IndexerError> {
        let body = json!({
            "actions": [
                { "remove": { "alias": alias, "index": format!("{prefix}*"), "must_exist": false } },
                { "add": { "alias": alias, "index": to_index } }
            ]
        });
        self.expect_ok(Method::POST, "/_aliases", Some(&body))
            .await?;
        Ok(())
    }

    async fn delete_index_quietly(&self, index: &str) {
        let _result = self
            .request(Method::DELETE, &format!("/{index}?ignore_unavailable=true"))
            .send()
            .await;
    }

    async fn ensure_read_only_user(
        &self,
        role: &str,
        username: &str,
        password: &str,
        index_patterns: &[String],
    ) -> Result<(), IndexerError> {
        let role_body = json!({
            "indices": [{ "names": index_patterns, "privileges": ["read", "view_index_metadata"] }]
        });
        self.expect_ok(
            Method::PUT,
            &format!("/_security/role/{role}"),
            Some(&role_body),
        )
        .await?;
        let user_body = json!({ "password": password, "roles": [role] });
        self.expect_ok(
            Method::PUT,
            &format!("/_security/user/{username}"),
            Some(&user_body),
        )
        .await?;
        Ok(())
    }
    async fn alias_count(&self, alias: &str) -> Result<Option<u64>, IndexerError> {
        let path = format!("/{alias}/_count");
        let (status, value) = self.send_json::<()>(Method::GET, &path, None).await?;
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(IndexerError::Elasticsearch(format!(
                "{path}: {status}: {value}"
            )));
        }
        Ok(Some(
            value.get("count").and_then(Value::as_u64).unwrap_or(0),
        ))
    }
}

pub(crate) fn ndjson_body(index: &str, docs: &[Document]) -> Result<String, IndexerError> {
    let mut body = String::new();
    for doc in docs {
        let source = serde_json::to_string(doc)
            .map_err(|e| IndexerError::Elasticsearch(format!("bulk {}: {e}", doc.slug())))?;
        body.push_str(&json!({ "index": { "_index": index, "_id": doc.slug() } }).to_string());
        body.push('\n');
        body.push_str(&source);
        body.push('\n');
    }
    Ok(body)
}

pub(crate) fn first_bulk_error(value: &Value) -> Option<&str> {
    if !value["errors"].as_bool().unwrap_or(false) {
        return None;
    }
    Some(
        value
            .get("items")
            .and_then(Value::as_array)
            .and_then(|items| {
                items.iter().find_map(|item| {
                    item.get("index")
                        .and_then(|index| index.get("error"))
                        .and_then(|error| error.get("reason"))
                        .and_then(Value::as_str)
                })
            })
            .unwrap_or("unknown"),
    )
}

pub(crate) fn index_names(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|r| r["index"].as_str().map(str::to_string))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bulk_body_is_one_action_line_and_one_document_line_per_doc() {
        let poet = |id: i32, slug: &str| {
            Document::Poet(crate::docs::to_poet_doc(
                crate::docs::PoetSource {
                    id,
                    slug: slug.into(),
                    name: "Poet".into(),
                    nickname: String::new(),
                    era_name: "Era".into(),
                    era_slug: "era".into(),
                    poems_count: 1,
                },
                &[],
            ))
        };
        let body = ndjson_body("poets_v1", &[poet(1, "TnKK"), poet(2, "abcd")]).expect("a body");
        let lines: Vec<Value> = body
            .lines()
            .map(|line| serde_json::from_str(line).expect("a JSON line"))
            .collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(
            lines[0],
            json!({ "index": { "_index": "poets_v1", "_id": "TnKK" } })
        );
        assert_eq!(
            (lines[1]["slug"].as_str(), lines[1]["id"].as_i64()),
            (Some("TnKK"), Some(1))
        );
        assert_eq!(
            lines[2],
            json!({ "index": { "_index": "poets_v1", "_id": "abcd" } })
        );
        assert_eq!(lines[3]["slug"], "abcd");
        assert!(body.ends_with('\n'));
        assert_eq!(ndjson_body("poets_v1", &[]).expect("an empty body"), "");
    }

    #[test]
    fn the_first_bulk_error_reason_is_surfaced_and_a_clean_response_has_none() {
        let failed = json!({ "errors": true, "items": [ { "index": { "status": 201 } }, { "index": { "error": { "reason": "strict_dynamic_mapping_exception" } } } ] });
        assert_eq!(
            first_bulk_error(&failed),
            Some("strict_dynamic_mapping_exception")
        );
        assert_eq!(
            first_bulk_error(&json!({ "errors": true, "items": [] })),
            Some("unknown")
        );
        assert_eq!(first_bulk_error(&json!({ "errors": false })), None);
        assert_eq!(first_bulk_error(&Value::Null), None);
    }

    #[test]
    fn cat_rows_yield_their_index_names_and_skip_blanks() {
        let rows = json!([{ "index": "poems_v1" }, { "index": "" }, { "health": "green" }, { "index": "poems_v2" }]);
        assert_eq!(
            index_names(&rows),
            vec!["poems_v1".to_string(), "poems_v2".to_string()]
        );
        assert!(index_names(&Value::Null).is_empty());
    }

    async fn serve_raw(status: &'static str, body: &'static str) -> std::net::SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _unused =
                    tokio::io::AsyncWriteExt::write_all(&mut socket, response.as_bytes()).await;
            }
        });
        address
    }

    async fn serve_slowly(
        delay: Duration,
        status: &'static str,
    ) -> (
        std::net::SocketAddr,
        std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = std::sync::Arc::clone(&requests);
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut head = vec![0u8; 1024];
                let read = tokio::io::AsyncReadExt::read(&mut socket, &mut head)
                    .await
                    .unwrap_or(0);
                let text = String::from_utf8_lossy(&head[..read]).to_string();
                seen.lock()
                    .expect("the request log")
                    .push(text.lines().next().unwrap_or_default().to_string());
                tokio::time::sleep(delay).await;
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{{}}"
                );
                let _unused =
                    tokio::io::AsyncWriteExt::write_all(&mut socket, response.as_bytes()).await;
            }
        });
        (address, requests)
    }

    #[tokio::test]
    async fn a_force_merge_asks_for_one_segment_and_outlasts_the_ordinary_request_timeout() {
        let (address, requests) = serve_slowly(Duration::from_millis(600), "200 OK").await;
        let es = Es::with_timeout(&format!("http://{address}"), Duration::from_millis(200))
            .expect("an endpoint");

        es.force_merge("poems_v4").await.expect("a merged index");

        assert_eq!(
            *requests.lock().expect("the request log"),
            ["POST /poems_v4/_forcemerge?max_num_segments=1 HTTP/1.1"]
        );
    }

    #[tokio::test]
    async fn a_refused_force_merge_is_an_error() {
        let (address, _) = serve_slowly(Duration::ZERO, "500 Internal Server Error").await;
        let es = Es::with_timeout(&format!("http://{address}"), Duration::from_secs(2))
            .expect("an endpoint");

        let refused = es.force_merge("poems_v4").await;

        assert!(
            refused
                .as_ref()
                .is_err_and(|e| e.to_string().contains("500")),
            "{refused:?}"
        );
    }

    #[tokio::test]
    async fn a_missing_alias_is_none_and_an_existing_one_is_its_count() {
        let address = serve_raw("404 Not Found", "{\"error\":\"index_not_found_exception\"}").await;
        let es = Es::with_timeout(&format!("http://{address}"), Duration::from_secs(2))
            .expect("an endpoint");
        assert_eq!(es.alias_count("poems").await.expect("a count"), None);

        let address = serve_raw("200 OK", "{\"count\":42}").await;
        let es = Es::with_timeout(&format!("http://{address}"), Duration::from_secs(2))
            .expect("an endpoint");
        assert_eq!(es.alias_count("poems").await.expect("a count"), Some(42));
    }

    #[tokio::test]
    async fn an_elasticsearch_that_never_answers_times_the_request_out() {
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

        let es = Es::with_timeout(&format!("http://{address}"), Duration::from_millis(250))
            .expect("an endpoint");
        let outcome = tokio::time::timeout(Duration::from_secs(5), es.alias_count("poems")).await;

        assert!(
            outcome
                .expect("the count must give up on its own, not be rescued")
                .is_err()
        );
    }

    type Captured = std::sync::Arc<std::sync::Mutex<Vec<(String, Value)>>>;

    async fn read_request(socket: &mut tokio::net::TcpStream) -> (String, Value) {
        let mut raw = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let read = tokio::io::AsyncReadExt::read(socket, &mut chunk)
                .await
                .unwrap_or(0);
            raw.extend_from_slice(&chunk[..read]);
            let text = String::from_utf8_lossy(&raw).to_string();
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let length = head
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                if body.len() >= length || read == 0 {
                    let line = head.lines().next().unwrap_or_default().to_string();
                    return (line, serde_json::from_str(body).unwrap_or(Value::Null));
                }
            }
            if read == 0 {
                return (String::new(), Value::Null);
            }
        }
    }

    async fn serve_capturing(statuses: Vec<&'static str>) -> (std::net::SocketAddr, Captured) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        let captured: Captured = std::sync::Arc::default();
        let seen = std::sync::Arc::clone(&captured);
        let mut statuses = statuses.into_iter();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let request = read_request(&mut socket).await;
                seen.lock().expect("the request log").push(request);
                let status = statuses.next().unwrap_or("500 Internal Server Error");
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{{}}"
                );
                let _unused =
                    tokio::io::AsyncWriteExt::write_all(&mut socket, response.as_bytes()).await;
            }
        });
        (address, captured)
    }

    #[tokio::test]
    async fn the_api_reader_gets_a_read_only_role_on_the_given_indices_then_its_user() {
        let (address, captured) = serve_capturing(vec!["200 OK", "200 OK"]).await;
        let es = Es::with_timeout(&format!("http://{address}"), Duration::from_secs(2))
            .expect("a local endpoint");
        es.ensure_read_only_user(
            "reader_role",
            "reader",
            "secret",
            &["poems".to_string(), "poems_v*".to_string()],
        )
        .await
        .expect("a provisioned reader");
        let requests = captured.lock().expect("the request log").clone();
        assert_eq!(
            requests,
            [
                (
                    "PUT /_security/role/reader_role HTTP/1.1".to_string(),
                    json!({ "indices": [{
                        "names": ["poems", "poems_v*"],
                        "privileges": ["read", "view_index_metadata"],
                    }] })
                ),
                (
                    "PUT /_security/user/reader HTTP/1.1".to_string(),
                    json!({ "password": "secret", "roles": ["reader_role"] })
                ),
            ]
        );
    }

    #[tokio::test]
    async fn a_refused_role_stops_before_any_user_is_created() {
        let (address, captured) = serve_capturing(vec!["403 Forbidden"]).await;
        let es = Es::with_timeout(&format!("http://{address}"), Duration::from_secs(2))
            .expect("a local endpoint");
        let outcome = es
            .ensure_read_only_user("reader_role", "reader", "secret", &["poems".to_string()])
            .await;
        assert!(outcome.is_err());
        assert_eq!(captured.lock().expect("the request log").len(), 1);
    }
}
