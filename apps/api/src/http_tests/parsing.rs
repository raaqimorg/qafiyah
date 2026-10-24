use axum::http::StatusCode;

use crate::constants::ES_MAX_RESULT_WINDOW;
use crate::cursor;
use crate::domain::paging::Cursor;
use crate::http_tests::{app_with, empty_hits};
use crate::test_support::{FakeEs, request, send};

#[tokio::test]
async fn a_cursor_no_page_gave_out_is_refused_before_any_backend_call() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let poems = cursor::encode(Cursor::After(1264));
    let past_window = cursor::encode_offset(ES_MAX_RESULT_WINDOW);
    for path in [
        "/v1/poems?page=0".to_string(),
        "/v1/poems?cursor=0".to_string(),
        "/v1/poems/slugs?cursor=".to_string(),
        format!("/v1/poems?cursor={past_window}"),
        format!("/v1/poets?cursor={poems}"),
        format!("/v1/poets/slugs?cursor={past_window}"),
        format!("/v1/search?poemsCursor={past_window}"),
        format!("/v1/search?poetsCursor={poems}"),
    ] {
        let sent = send(app_with(&es), request("GET", &path)).await;
        assert_eq!(sent.status, StatusCode::BAD_REQUEST, "{path}");
    }
    assert!(es.requests().await.is_empty());
}

#[tokio::test]
async fn search_refuses_a_poem_only_facet_when_poets_are_requested() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    for (path, name) in [
        ("/v1/search?types=poets&meterSlugs=altawil", "meterSlugs"),
        ("/v1/search?rhymeSlugs=meem", "rhymeSlugs"),
        (
            "/v1/search?types=poems&types=poets&themeSlugs=alnasib",
            "themeSlugs",
        ),
        ("/v1/search?types=poets&poemTypeSlugs=hurr", "poemTypeSlugs"),
    ] {
        let sent = send(app_with(&es), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(
            sent.json()["detail"],
            format!("Invalid query parameter `{name}`: filters poems only, send `types=poems`"),
            "{path}"
        );
    }
    let ok = send(
        app_with(&es),
        request("GET", "/v1/search?types=poems&meterSlugs=altawil"),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK);
}

#[tokio::test]
async fn a_verse_form_filter_reaches_elasticsearch_as_a_terms_filter() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let sent = send(
        app_with(&es),
        request("GET", "/v1/search?types=poems&poemTypeSlugs=hurr"),
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
        request("GET", "/v1/search?q=%20%20&types=poems"),
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
async fn unknown_parameters_and_bracket_forms_are_refused_by_name() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    for (path, name) in [
        ("/v1/search?foo=bar&types=poems", "foo"),
        ("/v1/search?types[]=poems", "types[]"),
        ("/v1/search?eraSlugs[0]=jahili", "eraSlugs[0]"),
    ] {
        let sent = send(app_with(&es), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::BAD_REQUEST, "{path}");
        let detail = sent.json()["detail"]
            .as_str()
            .expect("a detail")
            .to_string();
        let expected = format!(
            "Invalid query parameter `{name}`: unknown field `{name}`, expected one of `q`, `types`"
        );
        assert!(detail.starts_with(&expected), "{path}: {detail}");
    }
    assert!(es.requests().await.is_empty());
}

#[tokio::test]
async fn injection_shaped_text_and_broken_escapes_are_searched_as_text() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    for path in [
        "/v1/search?q=%27%20OR%201%3D1%20--&types=poems",
        "/v1/search?q=%ZZ&types=poems",
    ] {
        let sent = send(app_with(&es), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::OK, "{path}: {}", sent.body);
    }
    let echoed = send(
        app_with(&es),
        request("GET", "/v1/search?q=%ZZ&types=poems"),
    )
    .await;
    assert_eq!(echoed.json()["q"], "%ZZ");
}

#[tokio::test]
async fn the_poets_section_ignores_poet_slugs_and_repeated_types_are_harmless() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    for path in [
        "/v1/search?types=poets&poetSlugs=PAKT",
        "/v1/search?types=poems&types=poems&types=poems",
    ] {
        let sent = send(app_with(&es), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::OK, "{path}: {}", sent.body);
    }
}

#[tokio::test]
async fn the_poems_list_refuses_what_it_does_not_read_and_names_it() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let poems = "expected one of `cursor`, `poet`, `era`, `theme`, `meter`, `rhyme`, `collection`";
    let cursor = "must be the `next` or `prev` value of a page of this list";
    for (path, detail) in [
        (
            "/v1/poems?page=2",
            format!("Invalid query parameter `page`: unknown field `page`, {poems}"),
        ),
        (
            "/v1/poems?era=",
            "Invalid query parameter `era[0]`: must be a lowercase slug of letters and hyphens, at most 64 characters".to_string(),
        ),
        (
            "/v1/poems?cursor=2",
            format!("Invalid query parameter `cursor`: {cursor}"),
        ),
        (
            "/v1/poets?cursor=eyJhZnRlciI6MX0",
            format!("Invalid query parameter `cursor`: {cursor}"),
        ),
        (
            "/v1/poets?q=x&cursor=eyJhZnRlciI6eyJwb2Vtc0NvdW50Ijo3LCJuYW1lU29ydCI6Im4iLCJpZCI6M319",
            format!("Invalid query parameter `cursor`: {cursor}"),
        ),
        (
            "/v1/poets?cursor=eyJhZnRlciI6eyJzY29yZSI6Mi4wLCJwb2Vtc0NvdW50Ijo3LCJuYW1lU29ydCI6Im4iLCJpZCI6M319",
            format!("Invalid query parameter `cursor`: {cursor}"),
        ),
        (
            "/v1/search?poemsCursor=eyJhZnRlciI6MX0",
            format!("Invalid query parameter `poemsCursor`: {cursor}"),
        ),
        (
            "/v1/search?poetsCursor=eyJvZmZzZXQiOjk5ODF9",
            format!("Invalid query parameter `poetsCursor`: {cursor}"),
        ),
        (
            &format!("/v1/poems/slugs?cursor={}", "A".repeat(513)),
            format!("Invalid query parameter `cursor`: {cursor}"),
        ),
        (
            "/v1/poems/slugs?pages=2",
            "Invalid query parameter `pages`: unknown field `pages`, expected `cursor`".to_string(),
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
    for path in ["/v1/poets?", "/v1/poets?&era=abbasi&"] {
        let sent = send(app_with(&es), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::OK, "{path}: {}", sent.body);
    }
    let empty = send(app_with(&es), request("GET", "/v1/poets?cursor=")).await;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        empty.json()["detail"],
        "Invalid query parameter `cursor`: must be the `next` or `prev` value of a page of this list"
    );
}

#[tokio::test]
async fn an_operation_that_takes_no_parameters_refuses_any() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    for path in [
        "/v1/eras?cursor=x",
        "/v1/meters?x=1",
        "/v1/rhymes?x=1",
        "/v1/themes?x=1",
        "/v1/collections?x=1",
        "/v1/poem-types?x=1",
        "/v1/eras/jahili?x=1",
        "/v1/meters/altawil?x=1",
        "/v1/rhymes/meem?x=1",
        "/v1/themes/alhikma?x=1",
        "/v1/collections/almuallaqat?x=1",
        "/v1/poem-types/amudi?x=1",
        "/v1/poems/count?x=1",
        "/v1/poems/gnNg?x=1",
        "/v1/poets/PAKT?x=1",
    ] {
        let sent = send(app_with(&es), request("GET", path)).await;
        assert_eq!(sent.status, StatusCode::BAD_REQUEST, "{path}");
        let name = path
            .split_once('?')
            .expect("a query")
            .1
            .split('=')
            .next()
            .expect("a name");
        assert_eq!(
            sent.json()["detail"],
            format!(
                "Invalid query parameter `{name}`: unknown field `{name}`, there are no fields"
            ),
            "{path}"
        );
    }
    assert!(es.requests().await.is_empty());
}
