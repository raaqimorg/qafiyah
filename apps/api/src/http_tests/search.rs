use axum::http::StatusCode;
use serde_json::json;

use crate::http_tests::{app_with, empty_hits};
use crate::test_support::{FakeEs, request, send};

fn one_poem_and_one_poet() -> serde_json::Value {
    json!({ "timed_out": false, "_shards": { "total": 1, "successful": 1, "skipped": 0, "failed": 0 },
        "hits": { "total": { "value": 1 }, "hits": [ {
        "_score": 2.0,
        "_source": {
            "slug": "TnKK", "title": "t", "titleDisplay": "T", "content": "a*b*c",
            "poetNameDisplay": "P", "poetSlug": "yoFB", "poetHasAvatar": false,
            "poetIsAnonymous": false, "meterName": "m", "meterSlug": "altawil", "eraName": "e", "eraSlug": "abbasi",
            "name": "n", "nameDisplay": "N", "poemsCount": 7
        },
        "highlight": { "content": ["a*<mark>b</mark>*c"] }
    } ] } })
}

#[tokio::test]
async fn a_search_asks_both_indices_concurrently_and_returns_two_envelopes() {
    let es = FakeEs::serving(StatusCode::OK, one_poem_and_one_poet()).await;
    let sent = send(
        app_with(&es),
        request("GET", "/v1/search?q=%D8%AD%D8%A8&poemsPage=2"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);
    let body = sent.json();
    assert_eq!(body["q"], "حب");
    assert_eq!(body["poems"]["data"][0]["type"], "poem");
    assert_eq!(body["poems"]["data"][0]["title"], "T");
    assert_eq!(body["poems"]["data"][0]["snippet"], "a*<mark>b</mark>");
    assert_eq!(body["poems"]["data"][0]["poet"]["hasAvatar"], false);
    assert_eq!(body["poems"]["pagination"]["page"], 2);
    assert_eq!(body["poems"]["pagination"]["pageSize"], 20);
    assert_eq!(body["poets"]["data"][0]["type"], "poet");
    assert_eq!(body["poets"]["data"][0]["name"], "N");
    let asked = es.requests().await;
    let indices: Vec<&str> = asked.iter().map(|(index, _)| index.as_str()).collect();
    assert!(indices.contains(&"poems") && indices.contains(&"poets"));
    let poems_body = &asked
        .iter()
        .find(|(index, _)| index == "poems")
        .expect("poems body")
        .1;
    assert_eq!(poems_body["from"], 20);
}

#[tokio::test]
async fn a_types_filter_skips_the_other_index_entirely() {
    let es = FakeEs::serving(StatusCode::OK, one_poem_and_one_poet()).await;
    let sent = send(
        app_with(&es),
        request("GET", "/v1/search?q=x&types[]=poets"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::OK);
    let body = sent.json();
    assert!(body["poems"].is_null());
    assert!(body["poets"].is_object());
    assert_eq!(es.requests().await.len(), 1);
}

#[tokio::test]
async fn the_poets_list_browses_by_poem_count_with_the_wide_window() {
    let es = FakeEs::serving(StatusCode::OK, one_poem_and_one_poet()).await;
    let sent = send(app_with(&es), request("GET", "/v1/poets?page=2&era=abbasi")).await;
    assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);
    let body = sent.json();
    assert_eq!(body["data"][0]["poemsCount"], 7);
    assert_eq!(body["pagination"]["pageSize"], 30);
    let asked = es.requests().await;
    assert_eq!(asked[0].0, "poets");
    assert_eq!(asked[0].1["from"], 30);
    assert_eq!(asked[0].1["track_total_hits"], 50_000);
    assert_eq!(
        asked[0].1["query"]["bool"]["filter"][0]["terms"]["eraSlug"],
        json!(["abbasi"])
    );
    assert!(asked[0].1.get("highlight").is_none());
}

#[tokio::test]
async fn a_search_response_is_cacheable_json_with_an_etag() {
    let es = FakeEs::serving(StatusCode::OK, one_poem_and_one_poet()).await;
    let sent = send(app_with(&es), request("GET", "/v1/search?q=x")).await;
    assert!(sent.header("etag").is_some());
    assert_eq!(sent.header("content-type"), Some("application/json"));
}

const SEARCH_PATHS: [&str; 4] = [
    "/v1/search?q=x",
    "/v1/search?q=x&types=poems",
    "/v1/search?q=x&types=poets",
    "/v1/poets?q=x",
];

async fn assert_incomplete_search_is_unavailable(body: serde_json::Value) {
    let es = FakeEs::serving(StatusCode::OK, body).await;
    for path in SEARCH_PATHS {
        for conditional in [false, true] {
            let mut req = request("GET", path);
            if conditional {
                req.headers_mut()
                    .insert("if-none-match", "*".parse().unwrap());
            }
            let sent = send(app_with(&es), req).await;
            assert_eq!(
                sent.status,
                StatusCode::SERVICE_UNAVAILABLE,
                "{path}: {}",
                sent.body
            );
            assert_eq!(sent.header("cache-control"), Some("no-store"));
            assert_eq!(sent.header("retry-after"), Some("2"));
            assert_eq!(
                sent.header("content-type"),
                Some("application/problem+json")
            );
            assert!(sent.header("etag").is_none());
            assert_eq!(sent.json()["code"], "SERVICE_UNAVAILABLE");
        }
    }
}

#[tokio::test]
async fn an_elasticsearch_timeout_never_becomes_a_cacheable_empty_search() {
    let mut body = empty_hits();
    body["timed_out"] = json!(true);
    assert_incomplete_search_is_unavailable(body).await;
}

#[tokio::test]
async fn failed_shards_never_become_cacheable_partial_search_results() {
    let mut body = one_poem_and_one_poet();
    body["_shards"] = json!({ "total": 2, "successful": 1, "skipped": 0, "failed": 1 });
    assert_incomplete_search_is_unavailable(body).await;
}

#[tokio::test]
async fn a_complete_empty_search_with_skipped_shards_remains_cacheable() {
    let mut body = empty_hits();
    body["_shards"] = json!({ "total": 2, "successful": 2, "skipped": 1, "failed": 0 });
    let es = FakeEs::serving(StatusCode::OK, body).await;
    for path in SEARCH_PATHS {
        let app = app_with(&es);
        let sent = send(app.clone(), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::OK, "{path}: {}", sent.body);
        assert_eq!(
            sent.header("cache-control"),
            Some(crate::constants::READ_CACHE_CONTROL)
        );
        let etag = sent.header("etag").expect("a complete search has an ETag");
        let mut req = request("GET", path);
        req.headers_mut()
            .insert("if-none-match", etag.parse().unwrap());
        assert_eq!(send(app, req).await.status, StatusCode::NOT_MODIFIED);
    }
}

fn pdf_query() -> String {
    let pasted = " \u{FECB}\u{FEE8}\u{FE98}\u{FEAE}\u{FE93}  \u{FE91}\u{FEE6} \u{FEB7}\u{FEAA}\u{FE8D}\u{FEA9} ";
    form_urlencoded::byte_serialize(pasted.as_bytes()).collect()
}

#[tokio::test]
async fn a_search_query_reaches_elasticsearch_normalized_and_is_echoed_that_way() {
    let es = FakeEs::serving(StatusCode::OK, one_poem_and_one_poet()).await;
    let path = format!("/v1/search?types[]=poets&q={}", pdf_query());
    let sent = send(app_with(&es), request("GET", &path)).await;
    assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);
    assert_eq!(sent.json()["q"], "عنترة بن شداد");
    let asked = es.requests().await;
    assert_eq!(
        asked[0].1["query"]["bool"]["should"][0]["term"]["name.exact"]["value"],
        "عنترة بن شداد"
    );
}

#[tokio::test]
async fn the_poets_list_normalizes_its_query_the_same_way() {
    let es = FakeEs::serving(StatusCode::OK, one_poem_and_one_poet()).await;
    let path = format!("/v1/poets?q={}", pdf_query());
    let sent = send(app_with(&es), request("GET", &path)).await;
    assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);
    let asked = es.requests().await;
    assert_eq!(
        asked[0].1["query"]["bool"]["should"][0]["term"]["name.exact"]["value"],
        "عنترة بن شداد"
    );
}

#[tokio::test]
async fn the_length_limit_counts_the_query_as_sent() {
    let es = FakeEs::serving(StatusCode::OK, one_poem_and_one_poet()).await;
    let ligatures: String =
        form_urlencoded::byte_serialize("\u{FDFA}\u{FDFA}\u{FDFA}".as_bytes()).collect();
    let sent = send(
        app_with(&es),
        request("GET", &format!("/v1/search?q={ligatures}")),
    )
    .await;
    assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);
}
