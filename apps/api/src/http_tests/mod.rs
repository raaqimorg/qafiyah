mod account;
mod cache;
mod catalog;
mod cors;
mod errors;
mod go;
mod parsing;
mod rate_limit;
mod routing;
mod search;
mod site;

use axum::Router;
use serde_json::{Value, json};

use crate::test_support::{FakeEs, state};

pub(crate) fn empty_hits() -> Value {
    json!({ "timed_out": false, "_shards": { "total": 1, "successful": 1, "skipped": 0, "failed": 0 },
        "hits": { "total": { "value": 0 }, "hits": [] } })
}

pub(crate) fn app_with(es: &FakeEs) -> Router {
    crate::app(state(es))
}
