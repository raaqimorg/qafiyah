use std::sync::Arc;
use std::time::Duration;

use axum::body::{Body, to_bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::Value;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Mutex;
use tower::ServiceExt;

use crate::accounts::cache::KeyCache;
use crate::accounts::usage::UsageRecorder;
use crate::auth::Keys;
use crate::es::client::Es;
use crate::rate_limit::Limiter;
use crate::state::AppState;

const UNREACHABLE_POSTGRES: &str = "postgres://nobody:nothing@127.0.0.1:1/nothing";

pub fn lazy_pool() -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(250))
        .connect_lazy(UNREACHABLE_POSTGRES)
        .expect("a lazy pool never connects at construction")
}

pub fn lazy_corpus_pool() -> crate::db::PgPool {
    crate::db::pool(
        UNREACHABLE_POSTGRES,
        1,
        Duration::from_millis(250),
        String::new(),
    )
    .expect("a pool never connects at construction")
}

#[derive(Clone)]
struct Canned {
    status: StatusCode,
    body: Value,
    seen: Arc<Mutex<Vec<(String, Value)>>>,
}

pub struct FakeEs {
    pub url: String,
    seen: Arc<Mutex<Vec<(String, Value)>>>,
}

async fn answer(
    State(canned): State<Canned>,
    Path(index): Path<String>,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    canned.seen.lock().await.push((index, body));
    (canned.status, Json(canned.body.clone()))
}

impl FakeEs {
    pub async fn serving(status: StatusCode, body: Value) -> Self {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let router = Router::new()
            .route("/{index}/_search", post(answer))
            .with_state(Canned {
                status,
                body,
                seen: seen.clone(),
            });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("an ephemeral port");
        let address = listener.local_addr().expect("a bound address");
        tokio::spawn(async move {
            let _result = axum::serve(listener, router).await;
        });
        Self {
            url: format!("http://{address}"),
            seen,
        }
    }

    pub async fn requests(&self) -> Vec<(String, Value)> {
        self.seen.lock().await.clone()
    }
}

pub fn state_with(es: &FakeEs, keys: Keys, anon_requests: u32) -> AppState {
    AppState {
        pg: lazy_corpus_pool(),
        accounts: lazy_pool(),
        es: Arc::new(Es::with_timeout(&es.url, Duration::from_secs(2)).expect("a fake endpoint")),
        keys: Arc::new(keys),
        limiter: Arc::new(Limiter::default()),
        key_cache: Arc::new(KeyCache::default()),
        usage: Arc::new(UsageRecorder::default()),
        anon_requests,
    }
}

pub fn state(es: &FakeEs) -> AppState {
    state_with(
        es,
        Keys::new(Some("internal".into()), Some("full".into())),
        1_000_000,
    )
}

pub fn request(method: &str, path: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .expect("a well-formed test request")
}

pub struct Sent {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
}

impl Sent {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).expect("a JSON body")
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }
}

pub async fn send(app: Router, request: Request<Body>) -> Sent {
    let response = app
        .oneshot(request)
        .await
        .expect("the router is infallible");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 64 * 1024 * 1024)
        .await
        .expect("a readable body");
    Sent {
        status,
        headers,
        body: String::from_utf8_lossy(&bytes).into_owned(),
    }
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64().rem_euclid(n.max(1))
    }

    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next_u64().to_le_bytes()[0]).collect()
    }

    pub fn pick<'a>(&mut self, items: &'a [&'a str]) -> &'a str {
        let index = usize::try_from(self.below(u64::try_from(items.len()).expect("fits u64")))
            .expect("fits usize");
        items[index]
    }
}
