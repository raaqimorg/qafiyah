#![expect(
    clippy::expect_used,
    reason = "database tests treat a failed setup as a failed test"
)]
#![expect(clippy::panic, reason = "a non-JSON response body is a failed test")]
#![expect(clippy::print_stderr, reason = "the skip notice goes to stderr")]

mod db {
    pub(crate) mod accounts;
    pub(crate) mod contract;
    pub(crate) mod search;
}

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use diesel::pg::Pg;
use diesel::query_builder::{BoxedSqlQuery, SqlQuery};
use diesel::sql_types::{BigInt, Bool, Text};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, QueryableByName};
use diesel_async::RunQueryDsl;
use qafiyah_elasticsearch::Endpoint;
use reqwest::Method;
use serde_json::{Value, json};
use tokio::sync::OnceCell;
use tower::ServiceExt;

use qafiyah_api::auth::Keys;
use qafiyah_api::db::accounts_schema::users;
use qafiyah_api::db::lower;
use qafiyah_api::es::client::Es;
use qafiyah_api::state::AppState;

pub const INTERNAL: &str = "internal-test-key";
pub const FULL: &str = "full-test-key";

#[derive(Clone)]
pub struct Harness {
    pub app: Router,
    pub pg: qafiyah_api::db::PgPool,
    pub accounts: qafiyah_api::db::PgPool,
    pub state: AppState,
}

pub struct Sent {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
}

impl Sent {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_else(|_| panic!("not JSON: {}", self.body))
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }
}

pub async fn harness() -> Option<Harness> {
    let (Ok(pg_url), Ok(accounts_url), Ok(es_url)) = (
        std::env::var("QAFIYAH_TEST_DATABASE_URL"),
        std::env::var("QAFIYAH_TEST_DATABASE_URL_ACCOUNTS"),
        std::env::var("QAFIYAH_TEST_ELASTICSEARCH_URL"),
    ) else {
        eprintln!("skipping: QAFIYAH_TEST_* variables are not set");
        return None;
    };
    let pg = qafiyah_api::db::pool(
        &pg_url,
        4,
        Duration::from_secs(10),
        qafiyah_api::db::corpus_setup(),
    )
    .expect("the corpus database");
    static MIGRATED: OnceCell<()> = OnceCell::const_new();
    MIGRATED
        .get_or_init(async || {
            qafiyah_api::db::migrate(&accounts_url)
                .await
                .expect("migrations apply");
        })
        .await;
    let accounts = qafiyah_api::db::pool(
        &accounts_url,
        4,
        Duration::from_secs(10),
        qafiyah_api::db::accounts_setup(),
    )
    .expect("the accounts database");
    let state = AppState::new(
        pg.clone(),
        accounts.clone(),
        Es::new(&es_url).expect("an Elasticsearch endpoint"),
        Keys::new(Some(INTERNAL.into()), Some(FULL.into())),
        1_000_000,
    );
    Some(Harness {
        app: qafiyah_api::app(state.clone()),
        pg,
        accounts,
        state,
    })
}

#[derive(QueryableByName)]
struct TextRow {
    #[diesel(sql_type = Text)]
    value: String,
}

#[derive(QueryableByName)]
struct PairRow {
    #[diesel(sql_type = Text)]
    first: String,
    #[diesel(sql_type = Text)]
    second: String,
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    value: i64,
}

#[derive(QueryableByName)]
struct FlagRow {
    #[diesel(sql_type = Bool)]
    value: bool,
}

fn bound(sql: &str, binds: &[&str]) -> BoxedSqlQuery<'static, Pg, SqlQuery> {
    let mut query = diesel::sql_query(sql).into_boxed();
    for bind in binds {
        query = query.bind::<Text, _>((*bind).to_string());
    }
    query
}

impl Harness {
    pub async fn texts(&self, sql: &str, binds: &[&str]) -> Vec<String> {
        let mut conn = self.pg.get().await.expect("a corpus connection");
        bound(sql, binds)
            .load::<TextRow>(&mut conn)
            .await
            .expect(sql)
            .into_iter()
            .map(|row| row.value)
            .collect()
    }

    pub async fn text(&self, sql: &str, binds: &[&str]) -> Option<String> {
        let mut conn = self.pg.get().await.expect("a corpus connection");
        bound(sql, binds)
            .get_result::<TextRow>(&mut conn)
            .await
            .optional()
            .expect(sql)
            .map(|row| row.value)
    }

    pub async fn pair(&self, sql: &str, binds: &[&str]) -> Option<(String, String)> {
        let mut conn = self.pg.get().await.expect("a corpus connection");
        bound(sql, binds)
            .get_result::<PairRow>(&mut conn)
            .await
            .optional()
            .expect(sql)
            .map(|row| (row.first, row.second))
    }

    pub async fn count(&self, sql: &str, binds: &[&str]) -> i64 {
        let mut conn = self.pg.get().await.expect("a corpus connection");
        bound(sql, binds)
            .get_result::<CountRow>(&mut conn)
            .await
            .expect(sql)
            .value
    }

    pub async fn flag(&self, sql: &str, binds: &[&str]) -> bool {
        let mut conn = self.pg.get().await.expect("a corpus connection");
        bound(sql, binds)
            .get_result::<FlagRow>(&mut conn)
            .await
            .expect(sql)
            .value
    }

    pub async fn call(
        &self,
        method: &str,
        path: &str,
        key: Option<&str>,
        json: Option<Value>,
    ) -> Sent {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(key) = key {
            builder = builder.header("x-api-key", key);
        }
        let body = match json {
            Some(value) => {
                builder = builder.header("content-type", "application/json");
                Body::from(value.to_string())
            }
            None => Body::empty(),
        };
        let response = self
            .app
            .clone()
            .oneshot(builder.body(body).expect("a request"))
            .await
            .expect("infallible");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), 64 * 1024 * 1024)
            .await
            .expect("a body");
        Sent {
            status,
            headers,
            body: String::from_utf8_lossy(&bytes).into_owned(),
        }
    }

    pub async fn get(&self, path: &str) -> Sent {
        self.call("GET", path, Some(FULL), None).await
    }

    pub async fn delete_user(&self, email: &str) {
        let mut conn = self.accounts.get().await.expect("an accounts connection");
        diesel::delete(users::table.filter(lower(users::email).eq(lower(email))))
            .execute(&mut conn)
            .await
            .expect("cleanup");
    }

    pub async fn isolated<F, Fut>(&self, tag: &str, body: F)
    where
        F: FnOnce(Harness, String) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let email = unique_email(tag);
        let outcome = tokio::spawn(body(self.clone(), email.clone())).await;
        self.delete_user(&email).await;
        if let Err(error) = outcome {
            std::panic::resume_unwind(error.into_panic());
        }
    }
}

pub fn unique_email(tag: &str) -> String {
    format!("qafiyah-test-{tag}-{}@example.test", unique_suffix())
}

static SCRATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn unique_suffix() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!(
        "{nanos}-{}",
        SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}

#[test]
fn scratch_names_never_repeat_even_within_one_clock_tick() {
    let names: std::collections::HashSet<String> = (0..10_000).map(|_| unique_suffix()).collect();
    assert_eq!(names.len(), 10_000);
}

pub struct Admin {
    endpoint: Endpoint,
    url: String,
}

pub fn admin() -> Option<Admin> {
    let Ok(url) = std::env::var("QAFIYAH_TEST_ELASTICSEARCH_ADMIN_URL") else {
        eprintln!("skipping: QAFIYAH_TEST_ELASTICSEARCH_ADMIN_URL is not set");
        return None;
    };
    let endpoint = Endpoint::new(&url).expect("an Elasticsearch admin endpoint");
    Some(Admin { endpoint, url })
}

impl Admin {
    pub fn endpoint(&self) -> Endpoint {
        Endpoint::new(&self.url).expect("an Elasticsearch admin endpoint")
    }

    pub async fn with_poems<F, Fut>(&self, docs: &[Value], body: F)
    where
        F: FnOnce(Es, String) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.with_poems_and_settings(docs, &json!({}), body).await;
    }

    pub async fn with_poems_and_settings<F, Fut>(&self, docs: &[Value], settings: &Value, body: F)
    where
        F: FnOnce(Es, String) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let index = format!("test-guard-poems-{}", unique_suffix());
        let mut definition = qafiyah_elasticsearch::load().poems;
        if let (Some(target), Some(extra)) = (
            definition
                .get_mut("settings")
                .and_then(Value::as_object_mut),
            settings.as_object(),
        ) {
            target.extend(extra.clone());
        }
        let created = self
            .endpoint
            .request(Method::PUT, &format!("/{index}"))
            .json(&definition)
            .send()
            .await
            .expect("create the scratch index");
        let status = created.status();
        assert!(
            status.is_success(),
            "create {index}: {status}: {}",
            created.text().await.unwrap_or_default()
        );
        let mut bulk = String::new();
        for doc in docs {
            bulk.push_str(&json!({ "index": { "_index": index, "_id": doc["slug"] } }).to_string());
            bulk.push('\n');
            bulk.push_str(&doc.to_string());
            bulk.push('\n');
        }
        let report: Value = self
            .endpoint
            .request(Method::POST, "/_bulk?refresh=wait_for")
            .header("content-type", "application/x-ndjson")
            .body(bulk)
            .send()
            .await
            .expect("index the scratch documents")
            .json()
            .await
            .expect("a bulk report");
        let es = Es::new(&self.url).expect("an Elasticsearch endpoint");
        let outcome = tokio::spawn(body(es, index.clone())).await;
        self.endpoint
            .request(Method::DELETE, &format!("/{index}"))
            .send()
            .await
            .expect("delete the scratch index");
        assert_eq!(report.get("errors"), Some(&Value::Bool(false)), "{report}");
        if let Err(error) = outcome {
            std::panic::resume_unwind(error.into_panic());
        }
    }
}
