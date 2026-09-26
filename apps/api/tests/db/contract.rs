use std::collections::HashSet;

use axum::http::StatusCode;
use serde_json::Value;

use crate::{Harness, harness};

async fn h() -> Option<Harness> {
    harness().await
}

#[expect(
    clippy::expect_used,
    reason = "a missing pagination field is a failed test"
)]
fn total_pages(body: &Value) -> u64 {
    body.get("pagination")
        .and_then(|pagination| pagination.get("totalPages"))
        .and_then(Value::as_u64)
        .expect("totalPages")
}

#[expect(
    clippy::expect_used,
    reason = "a missing pagination field is a failed test"
)]
fn total_items(body: &Value) -> u64 {
    body.get("pagination")
        .and_then(|pagination| pagination.get("totalItems"))
        .and_then(Value::as_u64)
        .expect("totalItems")
}

#[tokio::test]
async fn every_list_operation_returns_an_envelope_whose_counts_agree() {
    let Some(h) = h().await else { return };
    for path in [
        "/v1/meters",
        "/v1/rhymes",
        "/v1/eras",
        "/v1/themes",
        "/v1/collections",
    ] {
        let sent = h.get(path).await;
        assert_eq!(sent.status, StatusCode::OK, "{path}: {}", sent.body);
        let body = sent.json();
        let data = body["data"].as_array().expect("data");
        assert!(!data.is_empty(), "{path} is empty on this dump");
        assert_eq!(
            total_items(&body),
            u64::try_from(data.len()).expect("fits u64")
        );
        assert_eq!(total_pages(&body), 1);
        assert!(
            data.iter()
                .all(|row| row["poemsCount"].as_i64().is_some() && row["slug"].as_str().is_some())
        );
    }
}

#[tokio::test]
async fn a_detail_read_for_a_listed_slug_matches_the_list_row_and_an_unknown_slug_is_404() {
    let Some(h) = h().await else { return };
    for (list, detail, resource) in [
        ("/v1/meters", "/v1/meters/{}", "Meter"),
        ("/v1/rhymes", "/v1/rhymes/{}", "Rhyme"),
        ("/v1/eras", "/v1/eras/{}", "Era"),
        ("/v1/themes", "/v1/themes/{}", "Theme"),
        ("/v1/collections", "/v1/collections/{}", "Collection"),
    ] {
        let first = h.get(list).await.json()["data"][0].clone();
        let slug = first["slug"].as_str().expect("slug");
        let sent = h.get(&detail.replace("{}", slug)).await;
        assert_eq!(sent.status, StatusCode::OK);
        assert_eq!(sent.json()["data"], first);
        let missing = h.get(&detail.replace("{}", "zzzzzzzz")).await;
        assert_eq!(missing.status, StatusCode::NOT_FOUND);
        assert_eq!(missing.json()["detail"], format!("{resource} not found"));
    }
}

#[tokio::test]
async fn poems_paginate_consistently_with_the_count_and_the_slug_stream() {
    let Some(h) = h().await else { return };
    let count = h.get("/v1/poems/count").await.json()["data"]["total"]
        .as_u64()
        .expect("total");
    let page1 = h.get("/v1/poems").await.json();
    assert_eq!(total_items(&page1), count);
    assert_eq!(page1["pagination"]["pageSize"], 30);
    assert_eq!(total_pages(&page1), count.div_ceil(30).max(1));
    let past = h
        .get(&format!("/v1/poems?page={}", total_pages(&page1) + 1))
        .await;
    assert_eq!(
        past.status,
        StatusCode::OK,
        "a page past the last is empty, not an error (pinned)"
    );
    assert!(past.json()["data"].as_array().expect("data").is_empty());
    let slugs = h.get("/v1/poems/slugs").await.json();
    assert_eq!(total_items(&slugs), count);
    assert_eq!(slugs["pagination"]["pageSize"], 45_000);
    let listed: Vec<&str> = slugs["data"]
        .as_array()
        .expect("slugs")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(
        u64::try_from(listed.len()).expect("fits u64"),
        count.min(45_000)
    );
    let unique: HashSet<&str> = listed.iter().copied().collect();
    assert_eq!(
        unique.len(),
        listed.len(),
        "slugs stream without duplicates"
    );
}

#[tokio::test]
async fn a_poem_detail_carries_verses_prosody_and_navigation_consistent_with_its_neighbors() {
    let Some(h) = h().await else { return };
    let slug = h.get("/v1/poems").await.json()["data"][0]["slug"]
        .as_str()
        .expect("slug")
        .to_string();
    let sent = h.get(&format!("/v1/poems/{slug}")).await;
    assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);
    let poem = sent.json()["data"].clone();
    assert_eq!(poem["slug"], slug);
    let verses = poem["verses"].as_array().expect("verses");
    assert!(!verses.is_empty());
    assert!(
        verses
            .iter()
            .all(|v| v.as_array().is_some_and(|pair| pair.len() == 2))
    );
    assert_eq!(
        usize::try_from(poem["verseCount"].as_u64().expect("count")).expect("fits usize"),
        verses.len()
    );
    for key in ["poet", "era", "meter", "theme", "rhyme", "poemType"] {
        assert!(poem[key]["slug"].as_str().is_some(), "{key}");
    }
    assert!(poem["relatedPoems"].as_array().expect("related").len() <= 10);
    if let Some(next) = poem["next"]["slug"].as_str() {
        let neighbor = h.get(&format!("/v1/poems/{next}")).await.json()["data"].clone();
        assert_eq!(neighbor["prev"]["slug"], slug, "next.prev must point back");
    }
    assert_eq!(h.get("/v1/poems/zzzz").await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn poem_neighbors_by_a_grouping_follow_that_grouping_list_order() {
    let Some(h) = h().await else { return };
    let first = h.get("/v1/poems").await.json()["data"][0].clone();
    let slug = first["slug"].as_str().expect("slug").to_string();
    let poem = h.get(&format!("/v1/poems/{slug}")).await.json()["data"].clone();
    let mut cases = vec![
        ("theme", slug.clone(), poem["theme"]["slug"].clone()),
        ("meter", slug.clone(), poem["meter"]["slug"].clone()),
        ("rhyme", slug.clone(), poem["rhyme"]["slug"].clone()),
    ];
    let collections = h.get("/v1/collections").await.json()["data"].clone();
    let populated = collections
        .as_array()
        .expect("collections")
        .iter()
        .find(|c| c["poemsCount"].as_u64().is_some_and(|n| n > 0))
        .cloned();
    if let Some(collection) = populated {
        let group = collection["slug"].as_str().expect("collection slug");
        let collected =
            h.get(&format!("/v1/poems?collection={group}")).await.json()["data"][0]["slug"]
                .as_str()
                .expect("collected poem slug")
                .to_string();
        cases.push(("collection", collected, collection["slug"].clone()));
    }
    for (by, slug, group) in cases {
        let group = group.as_str().expect("group slug");
        let listed: Vec<String> = h.get(&format!("/v1/poems?{by}={group}")).await.json()["data"]
            .as_array()
            .expect("list")
            .iter()
            .map(|p| p["slug"].as_str().expect("slug").to_string())
            .collect();
        let at = listed
            .iter()
            .position(|s| *s == slug)
            .expect("poem in its group list");
        let sent = h.get(&format!("/v1/poems/{slug}?by={by}")).await;
        assert_eq!(sent.status, StatusCode::OK, "{by}: {}", sent.body);
        let detail = sent.json()["data"].clone();
        assert_eq!(detail[by]["slug"], group, "{by}: detail names its group");
        if let Some(expected) = listed.get(at.saturating_add(1)) {
            assert_eq!(
                detail["next"]["slug"],
                expected.as_str(),
                "{by}: next is the next listed"
            );
            let neighbor =
                h.get(&format!("/v1/poems/{expected}?by={by}")).await.json()["data"].clone();
            assert_eq!(
                neighbor["prev"]["slug"],
                slug.as_str(),
                "{by}: next.prev points back"
            );
            assert_eq!(neighbor[by]["slug"], group, "{by}: next stays in the group");
        }
    }
}

#[tokio::test]
async fn every_poem_facet_narrows_the_list_to_a_subset_of_the_unfiltered_total() {
    let Some(h) = h().await else { return };
    let unfiltered = h.get("/v1/poems").await.json();
    let total = total_items(&unfiltered);
    let first = unfiltered["data"][0].clone();
    let poet = first["poet"]["slug"].as_str().expect("poet").to_string();
    let meter = first["meter"]["slug"].as_str().expect("meter").to_string();
    let first_slug = |list: &str| {
        let h = &h;
        let list = list.to_string();
        async move {
            h.get(&list).await.json()["data"][0]["slug"]
                .as_str()
                .expect("slug")
                .to_string()
        }
    };
    let era = first_slug("/v1/eras").await;
    let theme = first_slug("/v1/themes").await;
    let rhyme = first_slug("/v1/rhymes").await;
    let collection = first_slug("/v1/collections").await;
    for query in [
        format!("poet={poet}"),
        format!("meter={meter}"),
        format!("era={era}"),
        format!("theme={theme}"),
        format!("rhyme={rhyme}"),
        format!("collection={collection}"),
        format!("poet={poet}&meter={meter}&era={era}"),
    ] {
        let sent = h.get(&format!("/v1/poems?{query}")).await;
        assert_eq!(sent.status, StatusCode::OK, "{query}: {}", sent.body);
        let body = sent.json();
        let filtered = total_items(&body);
        assert!(filtered <= total, "{query}");
        if query == format!("poet={poet}") {
            assert!(filtered >= 1);
            assert!(
                body["data"]
                    .as_array()
                    .expect("data")
                    .iter()
                    .all(|row| row["poet"]["slug"] == poet.as_str())
            );
        }
    }
    let twice = h
        .get(&format!("/v1/poems?meter={meter}&meter={meter}"))
        .await
        .json();
    let once = h.get(&format!("/v1/poems?meter={meter}")).await.json();
    assert_eq!(total_items(&twice), total_items(&once));
}

#[tokio::test]
async fn poets_are_listed_by_count_readable_by_slug_and_streamed_for_sitemaps() {
    let Some(h) = h().await else { return };
    let list = h.get("/v1/poets").await;
    assert_eq!(list.status, StatusCode::OK, "{}", list.body);
    let body = list.json();
    let counts: Vec<i64> = body["data"]
        .as_array()
        .expect("data")
        .iter()
        .map(|p| p["poemsCount"].as_i64().expect("count"))
        .collect();
    let mut sorted = counts.clone();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(counts, sorted, "ordered by poem count descending");
    let slug = body["data"][0]["slug"].as_str().expect("slug");
    let detail = h.get(&format!("/v1/poets/{slug}")).await;
    assert_eq!(detail.status, StatusCode::OK);
    assert_eq!(detail.json()["data"]["slug"], slug);
    assert_eq!(h.get("/v1/poets/zzzz").await.status, StatusCode::NOT_FOUND);
    let stream = h.get("/v1/poets/slugs").await.json();
    assert_eq!(stream["pagination"]["pageSize"], 45_000);
    assert!(
        stream["data"]
            .as_array()
            .expect("slugs")
            .iter()
            .all(|e| e["hasAvatar"].is_boolean())
    );
    let era = h.get("/v1/eras").await.json()["data"][0]["slug"]
        .as_str()
        .expect("era")
        .to_string();
    let filtered = h.get(&format!("/v1/poets?era={era}")).await;
    assert_eq!(filtered.status, StatusCode::OK);
    let named = h
        .get("/v1/poets?q=%D8%A7%D9%84%D9%85%D8%AA%D9%86%D8%A8%D9%8A")
        .await;
    assert_eq!(named.status, StatusCode::OK);
}

#[tokio::test]
async fn search_returns_both_envelopes_with_the_documented_hit_shape() {
    let Some(h) = h().await else { return };
    let sent = h.get("/v1/search?q=%D8%AD%D8%A8").await;
    assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);
    let body = sent.json();
    assert_eq!(body["q"], "حب");
    for (section, kind) in [("poems", "poem"), ("poets", "poet")] {
        let envelope = &body[section];
        assert_eq!(envelope["pagination"]["pageSize"], 20, "{section}");
        for hit in envelope["data"].as_array().expect("hits") {
            assert_eq!(hit["type"], kind);
            assert!(hit["slug"].as_str().is_some_and(|s| s.len() == 4));
            assert!(hit["relevance"].as_f64().is_some());
            if section == "poems" {
                assert!(hit["snippet"].as_str().is_some());
                assert!(hit["poet"]["slug"].as_str().is_some());
                assert!(hit["poet"]["hasAvatar"].is_boolean());
            }
        }
    }
    let exact = h
        .get("/v1/search?q=%D8%AD%D8%A8&exact=true&types[]=poems")
        .await
        .json();
    assert!(exact["poets"].is_null());
    let era = h.get("/v1/eras").await.json()["data"][0]["slug"]
        .as_str()
        .expect("era")
        .to_string();
    let filtered = h
        .get(&format!(
            "/v1/search?q=%D8%AD%D8%A8&types[]=poems&eraSlugs[]={era}"
        ))
        .await
        .json();
    assert!(total_items(&filtered["poems"]) <= total_items(&body["poems"]));
    let empty = h.get("/v1/search").await.json();
    assert_eq!(empty["q"], "");
    assert!(empty["poems"]["data"].as_array().is_some());
}

#[tokio::test]
async fn the_random_poem_answers_in_both_shapes() {
    let Some(h) = h().await else { return };
    let slug = h.get("/v1/poems/random").await;
    assert_eq!(slug.status, StatusCode::OK, "{}", slug.body);
    assert_eq!(
        slug.header("content-type"),
        Some("text/plain; charset=UTF-8")
    );
    assert_eq!(slug.header("cache-control"), Some("no-store"));
    assert_eq!(slug.body.len(), 4);
    let lines = h.get("/v1/poems/random?option=lines").await;
    assert_eq!(lines.status, StatusCode::OK, "{}", lines.body);
    assert!(lines.body.encode_utf16().count() <= 280);
    assert!(lines.body.contains('\n'));
    assert_eq!(
        h.get("/v1/poems/random?option=verses").await.status,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn every_json_success_carries_the_read_cache_policy_and_a_matching_conditional_is_304() {
    let Some(h) = h().await else { return };
    let first = h.get("/v1/meters").await;
    let etag = first.header("etag").expect("etag").to_string();
    assert_eq!(
        first.header("cache-control"),
        Some("private, max-age=300, stale-while-revalidate=86400")
    );
    let request = axum::http::Request::builder()
        .uri("/v1/meters")
        .header("x-api-key", crate::FULL)
        .header("if-none-match", etag)
        .body(axum::body::Body::empty())
        .expect("a request");
    let response = tower::ServiceExt::oneshot(h.app.clone(), request)
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn a_retired_poem_slug_redirects_permanently_to_the_poem_that_absorbed_it() {
    let Some(h) = h().await else { return };
    let pair: Option<(String, String)> = sqlx::query_as(
        "SELECT a.slug, p.slug FROM public.poem_aliases a \
         JOIN public.poems p ON p.id = a.poem_id ORDER BY a.slug LIMIT 1",
    )
    .fetch_optional(&h.pg)
    .await
    .expect("alias query");
    let Some((alias, survivor)) = pair else {
        return;
    };

    let moved = h.get(&format!("/v1/poems/{alias}")).await;
    assert_eq!(moved.status, StatusCode::MOVED_PERMANENTLY);
    let expected = format!("/v1/poems/{survivor}");
    assert_eq!(moved.header("location"), Some(expected.as_str()));

    let landed = h.get(&expected).await;
    assert_eq!(landed.status, StatusCode::OK);
    assert_eq!(landed.json()["data"]["slug"], survivor.as_str());

    let recased: String = alias
        .chars()
        .map(|c| {
            if c.is_ascii_uppercase() {
                c.to_ascii_lowercase()
            } else {
                c.to_ascii_uppercase()
            }
        })
        .collect();
    let taken: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM public.poems WHERE slug = $1) \
         OR EXISTS (SELECT 1 FROM public.poem_aliases WHERE slug = $1)",
    )
    .bind(&recased)
    .fetch_one(&h.pg)
    .await
    .expect("recased lookup");
    if !taken {
        assert_eq!(
            h.get(&format!("/v1/poems/{recased}")).await.status,
            StatusCode::NOT_FOUND
        );
    }
}

#[tokio::test]
async fn a_slug_that_is_neither_a_poem_nor_an_alias_is_still_not_found() {
    let Some(h) = h().await else { return };
    let free: Option<String> = sqlx::query_scalar(
        "SELECT c FROM unnest(ARRAY['Qzqz','Zqzq','Xqxq','Qxqx']) AS c \
         WHERE NOT EXISTS (SELECT 1 FROM public.poems WHERE slug = c) \
         AND NOT EXISTS (SELECT 1 FROM public.poem_aliases WHERE slug = c) LIMIT 1",
    )
    .fetch_optional(&h.pg)
    .await
    .expect("free slug query");
    let Some(free) = free else { return };
    assert_eq!(
        h.get(&format!("/v1/poems/{free}")).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn a_single_term_total_from_the_stats_table_equals_a_live_count_of_primaries() {
    let Some(h) = h().await else { return };
    let body = h.get("/v1/poems?theme=almutafarriqat").await.json();
    let stats_total = total_items(&body);
    let live: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.poems p JOIN public.themes t ON t.id = p.theme_id \
         WHERE t.slug = 'almutafarriqat' AND p.recension_of_id IS NULL",
    )
    .fetch_one(&h.pg)
    .await
    .unwrap_or(-1);
    assert_eq!(i64::try_from(stats_total).unwrap_or(-2), live);
}

#[tokio::test]
async fn a_recension_names_its_primary_and_the_primary_lists_it() {
    let Some(h) = h().await else { return };
    let pair: Option<(String, String)> = sqlx::query_as(
        "SELECT r.slug, p.slug FROM public.poems r JOIN public.poems p ON p.id = r.recension_of_id \
         ORDER BY r.id LIMIT 1",
    )
    .fetch_optional(&h.pg)
    .await
    .expect("recension query");
    let Some((recension, primary)) = pair else {
        return;
    };
    let variant = h.get(&format!("/v1/poems/{recension}")).await.json();
    assert_eq!(variant["data"]["recensionOf"]["slug"], primary.as_str());
    let main = h.get(&format!("/v1/poems/{primary}")).await.json();
    assert!(main["data"].get("recensionOf").is_none());
    let listed = main["data"]["recensions"]
        .as_array()
        .expect("recensions array");
    assert!(listed.iter().any(|r| r["slug"] == recension.as_str()));
    let chained: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.poems r JOIN public.poems p ON p.id = r.recension_of_id \
         WHERE p.recension_of_id IS NOT NULL OR p.poet_id <> r.poet_id",
    )
    .fetch_one(&h.pg)
    .await
    .expect("chain query");
    assert_eq!(
        chained, 0,
        "a recension must point at a primary of the same poet"
    );
}
