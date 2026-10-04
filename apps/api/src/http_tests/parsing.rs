use axum::http::StatusCode;

use crate::http_tests::{app_with, empty_hits};
use crate::test_support::{FakeEs, request, send};

#[tokio::test]
async fn page_zero_and_overflow_are_refused_before_any_backend_call() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    for path in [
        "/v1/poems?page=0",
        "/v1/poems?page=4294967296",
        "/v1/poems/slugs?page=0",
        "/v1/poets/slugs?page=0",
        "/v1/poets?page=1667",
        "/v1/poets?page=0",
        "/v1/search?poemsPage=501",
        "/v1/search?poetsPage=0",
    ] {
        let sent = send(app_with(&es), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::BAD_REQUEST, "{path}");
    }
    assert!(es.requests().await.is_empty());
}

#[tokio::test]
async fn search_refuses_a_poem_only_facet_when_poets_are_requested() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    for path in [
        "/v1/search?types[]=poets&meterSlugs[]=altawil",
        "/v1/search?rhymeSlugs[]=meem",
        "/v1/search?types[]=poems&types[]=poets&themeSlugs[]=alnasib",
        "/v1/search?types[]=poets&poemTypeSlugs[]=hurr",
    ] {
        assert_eq!(
            send(app_with(&es), request("GET", path)).await.status,
            StatusCode::BAD_REQUEST,
            "{path}"
        );
    }
    let ok = send(
        app_with(&es),
        request("GET", "/v1/search?types[]=poems&meterSlugs[]=altawil"),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK);
}

#[tokio::test]
async fn a_verse_form_filter_reaches_elasticsearch_as_a_terms_filter() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let sent = send(
        app_with(&es),
        request("GET", "/v1/search?types[]=poems&poemTypeSlugs[]=hurr"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::OK);
    let filter = &es.requests().await[0].1["query"]["function_score"]["query"]["bool"]["filter"];
    assert!(
        filter
            .as_array()
            .expect("filters")
            .contains(&serde_json::json!({ "terms": { "poemTypeSlug": ["hurr"] } })),
        "{filter}"
    );
}

#[tokio::test]
async fn a_whitespace_only_query_is_empty_on_search_and_the_poets_list_alike() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let search = send(
        app_with(&es),
        request("GET", "/v1/search?q=%20%20&types[]=poems"),
    )
    .await;
    assert_eq!(search.status, StatusCode::OK);
    assert_eq!(search.json()["q"], "");
    let sent_to_es = es.requests().await;
    assert!(
        sent_to_es[0].1["query"]["function_score"]["query"]["bool"]["must"].is_array(),
        "a trimmed empty query browses"
    );
    let es2 = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let poets = send(app_with(&es2), request("GET", "/v1/poets?q=%20%20")).await;
    assert_eq!(poets.status, StatusCode::OK);
    assert!(
        es2.requests().await[0].1["query"]["bool"]["must"].is_array(),
        "a trimmed empty query browses"
    );
}

#[tokio::test]
async fn unknown_parameters_and_injection_shaped_values_are_ignored() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let sent = send(
        app_with(&es),
        request(
            "GET",
            "/v1/search?foo=bar&injection=%27%20OR%201%3D1%20--&types[]=poems",
        ),
    )
    .await;
    assert_eq!(sent.status, StatusCode::OK);
}

#[tokio::test]
async fn the_poems_list_refuses_what_it_does_not_read_and_names_it() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let poems = "expected one of `page`, `poet`, `era`, `theme`, `meter`, `rhyme`, `collection`";
    let page = "must be a whole number from 1 to";
    for (path, detail) in [
        (
            "/v1/poems?Page=2",
            format!("Invalid query parameter `Page`: unknown field `Page`, {poems}"),
        ),
        (
            "/v1/poems?era=",
            "Invalid query parameter `era[0]`: must be a lowercase slug of letters and hyphens, at most 64 characters".to_string(),
        ),
        (
            "/v1/poems?page=01",
            format!("Invalid query parameter `page`: {page} 4294967295, without a sign or leading zeros"),
        ),
        (
            "/v1/poets?page=1667",
            format!("Invalid query parameter `page`: {page} 1666, without a sign or leading zeros"),
        ),
        (
            "/v1/poems/slugs?pages=2",
            "Invalid query parameter `pages`: unknown field `pages`, expected `page`".to_string(),
        ),
    ] {
        let sent = send(app_with(&es), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(sent.json()["detail"], detail, "{path}");
    }
    assert!(es.requests().await.is_empty());
}

#[tokio::test]
async fn empty_pairs_in_a_query_string_are_nothing_but_an_empty_value_is_still_a_value() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    for path in ["/v1/poets?", "/v1/poets?&page=2&"] {
        let sent = send(app_with(&es), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::OK, "{path}: {}", sent.body);
    }
    let empty = send(app_with(&es), request("GET", "/v1/poets?page=")).await;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        empty.json()["detail"],
        "Invalid query parameter `page`: must be a whole number from 1 to 1666, without a sign or leading zeros"
    );
}
