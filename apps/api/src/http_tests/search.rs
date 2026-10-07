use axum::http::StatusCode;
use serde_json::json;

use crate::http_tests::app_with;
use crate::test_support::{FakeEs, request, send};

fn one_poem_and_one_poet() -> serde_json::Value {
    json!({ "hits": { "total": { "value": 1 }, "hits": [ {
        "_score": 2.0,
        "_source": {
            "slug": "TnKK", "title": "t", "titleDisplay": "T", "content": "a*b\nc",
            "poetNameDisplay": "P", "poetSlug": "yoFB", "poetHasAvatar": false,
            "poetIsAnonymous": false, "meterName": "m", "meterSlug": "altawil", "eraName": "e", "eraSlug": "abbasi",
            "name": "n", "nameDisplay": "N", "poemsCount": 7
        },
        "highlight": { "content": ["a*<mark>b</mark>\nc"] }
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
    let sent = send(app_with(&es), request("GET", "/v1/search?q=x&types=poets")).await;
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

fn pdf_query() -> String {
    let pasted = " \u{FECB}\u{FEE8}\u{FE98}\u{FEAE}\u{FE93}  \u{FE91}\u{FEE6} \u{FEB7}\u{FEAA}\u{FE8D}\u{FEA9} ";
    form_urlencoded::byte_serialize(pasted.as_bytes()).collect()
}

#[tokio::test]
async fn a_search_query_reaches_elasticsearch_normalized_and_is_echoed_that_way() {
    let es = FakeEs::serving(StatusCode::OK, one_poem_and_one_poet()).await;
    let path = format!("/v1/search?types=poets&q={}", pdf_query());
    let sent = send(app_with(&es), request("GET", &path)).await;
    assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);
    assert_eq!(sent.json()["q"], "عنترة بن شداد");
    let asked = es.requests().await;
    assert_eq!(
        asked[0].1["query"]["function_score"]["query"]["bool"]["should"][0]["term"]["name.exact"]["value"],
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
        asked[0].1["query"]["function_score"]["query"]["bool"]["should"][0]["term"]["name.exact"]["value"],
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
