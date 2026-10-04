use std::collections::HashSet;

use axum::http::StatusCode;
use qafiyah_api::db::poems::PgPoems;
use qafiyah_api::db::taxonomy::PgTaxonomy;
use qafiyah_api::domain::poems::PoemRepository;
use qafiyah_api::domain::taxonomy::TaxonomyRepository;
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
        "/v1/poem-types",
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
        ("/v1/poem-types", "/v1/poem-types/{}", "Poem type"),
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
async fn a_poem_filter_that_matches_nothing_lists_an_empty_page() {
    let Some(h) = h().await else { return };
    let first = h.get("/v1/poems").await.json()["data"][0].clone();
    let meter = first["meter"]["slug"].as_str().expect("meter").to_string();
    for query in [
        "meter=zzzzz".to_string(),
        "poet=Zzzz".to_string(),
        "theme=zzzzz&theme=yyyyy".to_string(),
        format!("meter={meter}&rhyme=zzzzz"),
    ] {
        let sent = h.get(&format!("/v1/poems?{query}")).await;
        assert_eq!(sent.status, StatusCode::OK, "{query}: {}", sent.body);
        let body = sent.json();
        assert_eq!(total_items(&body), 0, "{query}");
        assert_eq!(body["data"].as_array().map(Vec::len), Some(0), "{query}");
    }
}

#[tokio::test]
async fn two_meters_list_the_poems_of_either_one() {
    let Some(h) = h().await else { return };
    let meters = h.get("/v1/meters").await.json();
    let slug = |index: usize| {
        meters["data"][index]["slug"]
            .as_str()
            .expect("meter")
            .to_string()
    };
    let (a, b) = (slug(0), slug(1));
    let total = |query: String| {
        let h = &h;
        async move { total_items(&h.get(&format!("/v1/poems?{query}")).await.json()) }
    };
    let either = total(format!("meter={a}&meter={b}")).await;
    let sum = total(format!("meter={a}")).await + total(format!("meter={b}")).await;
    assert_eq!(either, sum);
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

#[expect(
    clippy::expect_used,
    reason = "a malformed facet list is a failed test"
)]
fn facet_total(list: &Value) -> i64 {
    list.as_array()
        .expect("a facet list")
        .iter()
        .map(|entry| entry["poemsCount"].as_i64().expect("poemsCount"))
        .sum()
}

#[tokio::test]
async fn a_poets_facets_partition_its_poems_and_narrow_under_each_other() {
    let Some(h) = h().await else { return };
    let poet = h.get("/v1/poets").await.json()["data"][0]["slug"]
        .as_str()
        .expect("slug")
        .to_string();
    let poems_count = h.get(&format!("/v1/poets/{poet}")).await.json()["data"]["poemsCount"]
        .as_i64()
        .expect("poemsCount");

    let sent = h.get(&format!("/v1/poems/facets?poet={poet}")).await;
    assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);
    let facets = sent.json()["data"].clone();
    for kind in ["meters", "rhymes", "themes"] {
        assert_eq!(facet_total(&facets[kind]), poems_count, "{kind}");
        let counts: Vec<i64> = facets[kind]
            .as_array()
            .expect(kind)
            .iter()
            .map(|entry| entry["poemsCount"].as_i64().expect("poemsCount"))
            .collect();
        assert!(counts.iter().all(|count| *count > 0), "{kind}: {counts:?}");
        let mut sorted = counts.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(counts, sorted, "{kind} ordered by poem count descending");
    }

    let meter = facets["meters"][0]["slug"]
        .as_str()
        .expect("meter")
        .to_string();
    let narrowed = h
        .get(&format!("/v1/poems/facets?poet={poet}&meter={meter}"))
        .await
        .json()["data"]
        .clone();
    let matching = total_items(
        &h.get(&format!("/v1/poems?poet={poet}&meter={meter}"))
            .await
            .json(),
    );
    assert_eq!(
        narrowed["meters"], facets["meters"],
        "a facet ignores its own selection"
    );
    assert_eq!(
        facet_total(&narrowed["rhymes"]),
        i64::try_from(matching).expect("fits i64")
    );
    assert_eq!(
        facet_total(&narrowed["themes"]),
        i64::try_from(matching).expect("fits i64")
    );

    let unused: Option<String> = h
        .text(
            "SELECT m.slug AS value FROM public.meters m WHERE NOT EXISTS \
         (SELECT 1 FROM public.poems p JOIN public.poets pt ON pt.id = p.poet_id \
          WHERE pt.slug = $1 AND p.meter_id = m.id AND p.recension_of_id IS NULL) \
         ORDER BY m.slug LIMIT 1",
            &[poet.as_str()],
        )
        .await;
    if let Some(unused) = unused {
        let kept = h
            .get(&format!("/v1/poems/facets?poet={poet}&meter={unused}"))
            .await
            .json()["data"]["meters"]
            .clone();
        let listed = kept
            .as_array()
            .expect("meters")
            .iter()
            .find(|entry| entry["slug"] == unused.as_str())
            .expect("a selected meter stays listed");
        assert_eq!(listed["poemsCount"], 0);
    }

    assert_eq!(
        h.get("/v1/poems/facets?poet=zzzz").await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn the_poet_slug_stream_leaves_out_poets_without_a_primary_poem() {
    let Some(h) = h().await else { return };
    let with_poems: i64 = h
        .count(
            "SELECT count(*) AS value FROM public.poets pt WHERE NOT pt.is_hidden AND EXISTS \
         (SELECT 1 FROM public.poems p WHERE p.poet_id = pt.id AND p.recension_of_id IS NULL)",
            &[],
        )
        .await;
    let stream = h.get("/v1/poets/slugs").await.json();
    assert_eq!(
        total_items(&stream),
        u64::try_from(with_poems).expect("fits u64")
    );
    let empty: Option<String> = h
        .text(
            "SELECT pt.slug AS value FROM public.poets pt WHERE NOT EXISTS \
         (SELECT 1 FROM public.poems p WHERE p.poet_id = pt.id AND p.recension_of_id IS NULL) \
         ORDER BY pt.slug LIMIT 1",
            &[],
        )
        .await;
    if let Some(empty) = empty {
        let listed = stream["data"].as_array().expect("slugs");
        assert!(listed.iter().all(|entry| entry["slug"] != empty.as_str()));
    }
}

#[tokio::test]
async fn a_retired_poet_slug_redirects_permanently_to_the_poet_that_absorbed_it() {
    let Some(h) = h().await else { return };
    let pair: Option<(String, String)> = h
        .pair(
            "SELECT a.slug AS first, p.slug AS second FROM public.poet_aliases a \
         JOIN public.poets p ON p.id = a.poet_id ORDER BY a.slug LIMIT 1",
            &[],
        )
        .await;
    let Some((alias, survivor)) = pair else {
        return;
    };

    let moved = h.get(&format!("/v1/poets/{alias}")).await;
    assert_eq!(moved.status, StatusCode::MOVED_PERMANENTLY);
    let expected = format!("/v1/poets/{survivor}");
    assert_eq!(moved.header("location"), Some(expected.as_str()));

    let landed = h.get(&expected).await;
    assert_eq!(landed.status, StatusCode::OK);
    assert_eq!(landed.json()["data"]["slug"], survivor.as_str());
}

#[tokio::test]
async fn a_slug_that_is_neither_a_poet_nor_an_alias_is_still_not_found() {
    let Some(h) = h().await else { return };
    let free: Option<String> = h
        .text(
            "SELECT c AS value FROM unnest(ARRAY['Qzqz','Zqzq','Xqxq','Qxqx']) AS c \
         WHERE NOT EXISTS (SELECT 1 FROM public.poets WHERE slug = c) \
         AND NOT EXISTS (SELECT 1 FROM public.poet_aliases WHERE slug = c) LIMIT 1",
            &[],
        )
        .await;
    let Some(free) = free else { return };
    assert_eq!(
        h.get(&format!("/v1/poets/{free}")).await.status,
        StatusCode::NOT_FOUND
    );
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
        .get("/v1/search?q=%D8%AD%D8%A8&exact=true&types=poems")
        .await
        .json();
    assert!(exact["poets"].is_null());
    let era = h.get("/v1/eras").await.json()["data"][0]["slug"]
        .as_str()
        .expect("era")
        .to_string();
    let filtered = h
        .get(&format!(
            "/v1/search?q=%D8%AD%D8%A8&types=poems&eraSlugs={era}"
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
async fn every_random_poem_is_a_named_classical_amudi_poem_of_four_verses_or_more_with_a_known_meter()
 {
    let Some(h) = h().await else { return };
    for _ in 0..25 {
        let slug = h.get("/v1/poems/random").await;
        assert_eq!(slug.status, StatusCode::OK, "{}", slug.body);
        let breaks_a_rule: bool = h
            .flag(
                "SELECT (p.recension_of_id IS NOT NULL OR p.is_hidden OR po.is_anonymous \
             OR e.slug NOT IN ('jahili', 'islami', 'umawi', 'abbasi') OR ty.slug <> 'amudi' \
             OR p.verse_count < 4 OR m.slug = 'ghayrmaruf') AS value \
             FROM public.poems p JOIN public.poets po ON po.id = p.poet_id \
             JOIN public.eras e ON e.id = po.era_id \
             JOIN public.poem_types ty ON ty.id = p.poem_type_id \
             JOIN public.meters m ON m.id = p.meter_id WHERE p.slug = $1",
                &[slug.body.as_str()],
            )
            .await;
        assert!(!breaks_a_rule, "{} breaks a random poem rule", slug.body);
    }
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
    let pair: Option<(String, String)> = h
        .pair(
            "SELECT a.slug AS first, p.slug AS second FROM public.poem_aliases a \
         JOIN public.poems p ON p.id = a.poem_id ORDER BY a.slug LIMIT 1",
            &[],
        )
        .await;
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
    let taken: bool = h
        .flag(
            "SELECT (EXISTS (SELECT 1 FROM public.poems WHERE slug = $1) \
         OR EXISTS (SELECT 1 FROM public.poem_aliases WHERE slug = $1)) AS value",
            &[recased.as_str()],
        )
        .await;
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
    let free: Option<String> = h
        .text(
            "SELECT c AS value FROM unnest(ARRAY['Qzqz','Zqzq','Xqxq','Qxqx']) AS c \
         WHERE NOT EXISTS (SELECT 1 FROM public.poems WHERE slug = c) \
         AND NOT EXISTS (SELECT 1 FROM public.poem_aliases WHERE slug = c) LIMIT 1",
            &[],
        )
        .await;
    let Some(free) = free else { return };
    assert_eq!(
        h.get(&format!("/v1/poems/{free}")).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn a_hidden_poet_and_their_poems_are_not_found_or_listed() {
    let Some(h) = h().await else { return };
    let hidden: Option<(String, String)> = h
        .pair(
            "SELECT pt.slug AS first, p.slug AS second FROM public.poets pt JOIN public.poems p ON p.poet_id = pt.id \
         WHERE pt.is_hidden ORDER BY p.id LIMIT 1",
            &[],
        )
        .await;
    let Some((poet, poem)) = hidden else { return };
    assert_eq!(
        h.get(&format!("/v1/poets/{poet}")).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.get(&format!("/v1/poems/{poem}")).await.status,
        StatusCode::NOT_FOUND
    );
    let listed = h.get(&format!("/v1/poems?poet={poet}")).await.json();
    assert_eq!(total_items(&listed), 0);
    assert_eq!(
        h.get(&format!("/v1/poems/facets?poet={poet}")).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn a_single_term_total_from_the_stats_table_equals_a_live_count_of_primaries() {
    let Some(h) = h().await else { return };
    let body = h.get("/v1/poems?theme=almutafarriqat").await.json();
    let stats_total = total_items(&body);
    let live: i64 = h
        .count(
            "SELECT count(*) AS value FROM public.poems p JOIN public.themes t ON t.id = p.theme_id \
         WHERE t.slug = 'almutafarriqat' AND p.recension_of_id IS NULL AND NOT p.is_hidden",
            &[],
        )
        .await;
    assert_eq!(i64::try_from(stats_total).unwrap_or(-2), live);
}

#[tokio::test]
async fn the_unfiltered_total_and_the_poem_count_equal_a_live_count_of_primaries() {
    let Some(h) = h().await else { return };
    let live: i64 = h
        .count(
            "SELECT count(*) AS value FROM public.poems WHERE recension_of_id IS NULL AND NOT is_hidden",
            &[],
        )
        .await;
    let listed = total_items(&h.get("/v1/poems").await.json());
    let counted = h.get("/v1/poems/count").await.json()["data"]["total"].as_i64();
    assert_eq!(i64::try_from(listed).unwrap_or(-2), live);
    assert_eq!(counted, Some(live));
}

#[tokio::test]
async fn a_total_over_several_values_of_one_filter_equals_a_live_count_of_primaries() {
    let Some(h) = h().await else { return };
    for (param, list, table, column) in [
        ("poet", "/v1/poets", "public.poets", "poet_id"),
        ("era", "/v1/eras", "public.eras", "era_id"),
        ("meter", "/v1/meters", "public.meters", "meter_id"),
        ("theme", "/v1/themes", "public.themes", "theme_id"),
        ("rhyme", "/v1/rhymes", "public.rhymes", "rhyme_id"),
    ] {
        let terms = h.get(list).await.json();
        let slugs: Vec<String> = terms["data"]
            .as_array()
            .expect("terms")
            .iter()
            .take(2)
            .map(|term| term["slug"].as_str().expect("slug").to_string())
            .collect();
        assert_eq!(slugs.len(), 2, "{list}");
        let query: Vec<String> = slugs.iter().map(|slug| format!("{param}={slug}")).collect();
        let body = h
            .get(&format!("/v1/poems?{}", query.join("&")))
            .await
            .json();
        let live: i64 = h
            .count(
                &format!(
                    "SELECT count(*) AS value FROM public.poems p JOIN {table} t ON t.id = p.{column} \
             WHERE t.slug IN ($1, $2) AND p.recension_of_id IS NULL AND NOT p.is_hidden"
                ),
                &slugs.iter().map(String::as_str).collect::<Vec<_>>(),
            )
            .await;
        assert_eq!(
            i64::try_from(total_items(&body)).unwrap_or(-2),
            live,
            "{param}: {slugs:?}"
        );
    }
}

#[tokio::test]
async fn a_recension_names_its_primary_and_the_primary_lists_it() {
    let Some(h) = h().await else { return };
    let pair: Option<(String, String)> = h
        .pair(
            "SELECT r.slug AS first, p.slug AS second FROM public.poems r JOIN public.poems p ON p.id = r.recension_of_id \
         ORDER BY r.id LIMIT 1",
            &[],
        )
        .await;
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
    let chained: i64 = h
        .count(
            "SELECT count(*) AS value FROM public.poems r JOIN public.poems p ON p.id = r.recension_of_id \
         WHERE p.recension_of_id IS NOT NULL OR p.poet_id <> r.poet_id",
            &[],
        )
        .await;
    assert_eq!(
        chained, 0,
        "a recension must point at a primary of the same poet"
    );
}

#[derive(diesel::QueryableByName)]
struct Statement {
    #[diesel(sql_type = diesel::sql_types::Text)]
    statement: String,
}

#[tokio::test]
async fn a_read_that_finds_every_connection_busy_is_unavailable_not_a_database_error() {
    let Ok(url) = std::env::var("QAFIYAH_TEST_DATABASE_URL") else {
        return;
    };
    let pool = qafiyah_api::db::pool(
        &url,
        1,
        std::time::Duration::from_millis(200),
        qafiyah_api::db::corpus_setup(),
    )
    .expect("a one-connection pool");
    let held = pool.get().await.expect("the one connection");
    let result = PgPoems::new(pool.clone()).count().await;
    assert!(
        matches!(result, Err(qafiyah_api::domain::StoreError::Unavailable(_))),
        "{result:?}"
    );
    drop(held);
    assert!(PgPoems::new(pool).count().await.is_ok());
}

#[tokio::test]
async fn a_planner_sensitive_list_is_never_kept_as_a_prepared_statement() {
    let Ok(url) = std::env::var("QAFIYAH_TEST_DATABASE_URL") else {
        return;
    };
    let pool = qafiyah_api::db::pool(
        &url,
        1,
        std::time::Duration::from_secs(10),
        qafiyah_api::db::corpus_setup(),
    )
    .expect("a one-connection pool");
    let facets = qafiyah_api::domain::poems::Facets {
        meter: vec!["altawil".into(), "alkamil".into()],
        rhyme: vec!["meem".into(), "lam".into()],
        ..Default::default()
    };
    let poems = PgPoems::new(pool.clone());
    for _ in 0..6 {
        poems.list(&facets, 1, 30).await.expect("a list");
    }
    PgTaxonomy::new(pool.clone())
        .list_counted(qafiyah_api::domain::taxonomy::Counted::Meters)
        .await
        .expect("the meters");
    let mut conn = pool.get().await.expect("the one connection");
    let kept: Vec<String> = diesel_async::RunQueryDsl::load::<Statement>(
        diesel::sql_query("SELECT statement FROM pg_prepared_statements WHERE name <> ''"),
        &mut conn,
    )
    .await
    .expect("the prepared statements")
    .into_iter()
    .map(|row| row.statement)
    .collect();
    assert!(
        kept.iter().any(|sql| sql.contains("\"meter_stats\"")),
        "an ordinary query is kept: {kept:?}"
    );
    assert!(
        !kept
            .iter()
            .any(|sql| sql.starts_with("SELECT \"poems\".\"title\"")
                || sql.contains("COUNT(*) FROM \"poems\"")),
        "the list and its count are not kept: {kept:?}"
    );
}

#[tokio::test]
async fn a_pooled_connection_carries_the_statement_timeout() {
    let Some(h) = h().await else { return };
    #[derive(diesel::QueryableByName)]
    struct Setting {
        #[diesel(sql_type = diesel::sql_types::Text)]
        statement_timeout: String,
    }
    let mut conn = h.pg.get().await.expect("a connection");
    let setting = diesel_async::RunQueryDsl::get_result::<Setting>(
        diesel::sql_query("SHOW statement_timeout"),
        &mut conn,
    )
    .await
    .expect("the setting");
    assert_eq!(setting.statement_timeout, "5s");
}

const SHOWN: &str = "recension_of_id IS NULL AND NOT is_hidden";

#[expect(clippy::expect_used, reason = "a missing page is a failed test")]
async fn listed_slugs(h: &Harness, query: &str) -> Vec<String> {
    let sent = h.get(&format!("/v1/poems?{query}")).await;
    assert_eq!(sent.status, StatusCode::OK, "{query}: {}", sent.body);
    sent.json()
        .get("data")
        .and_then(Value::as_array)
        .expect("a page")
        .iter()
        .filter_map(|poem| poem.get("slug").and_then(Value::as_str).map(str::to_string))
        .collect()
}

async fn page_holding(h: &Harness, unshown: &str) -> Option<i64> {
    let first = h
        .text(
            &format!(
                "SELECT id::text AS value FROM public.poems WHERE {unshown} ORDER BY id LIMIT 1"
            ),
            &[],
        )
        .await?;
    let before = h
        .count(
            &format!("SELECT count(*) AS value FROM public.poems WHERE {SHOWN} AND id < $1::int"),
            &[&first],
        )
        .await;
    Some(before.div_euclid(30).saturating_add(1))
}

#[tokio::test]
async fn a_list_page_is_the_next_thirty_shown_primary_poems_in_id_order() {
    let Some(h) = h().await else { return };
    let mut pages = vec![1];
    for unshown in ["recension_of_id IS NOT NULL", "is_hidden"] {
        if let Some(page) = page_holding(&h, unshown).await {
            pages.push(page);
        }
    }
    for page in pages {
        let offset = ((page - 1) * 30).to_string();
        let expected = h
            .texts(
                &format!(
                    "SELECT slug AS value FROM public.poems WHERE {SHOWN} \
                     ORDER BY id LIMIT 30 OFFSET $1::int"
                ),
                &[&offset],
            )
            .await;
        assert!(!expected.is_empty(), "page {page}");
        assert_eq!(
            listed_slugs(&h, &format!("page={page}")).await,
            expected,
            "page {page}"
        );
    }
    let altawil = h
        .count(
            &format!(
                "SELECT count(*) AS value FROM public.poems WHERE {SHOWN} \
                 AND meter_id = (SELECT id FROM public.meters WHERE slug = $1)"
            ),
            &["altawil"],
        )
        .await;
    let page: i64 = if altawil > 30 { 2 } else { 1 };
    let offset = ((page - 1) * 30).to_string();
    let expected = h
        .texts(
            &format!(
                "SELECT slug AS value FROM public.poems WHERE {SHOWN} \
                 AND meter_id = (SELECT id FROM public.meters WHERE slug = $1) \
                 ORDER BY id LIMIT 30 OFFSET $2::int"
            ),
            &["altawil", &offset],
        )
        .await;
    assert!(!expected.is_empty(), "altawil page {page}");
    assert_eq!(
        listed_slugs(&h, &format!("meter=altawil&page={page}")).await,
        expected
    );
}

#[tokio::test]
async fn previous_and_next_skip_the_poets_recensions_and_hidden_poems() {
    let Some(h) = h().await else { return };
    for unshown in ["r.recension_of_id IS NOT NULL", "r.is_hidden"] {
        let Some((before, after)) = h
            .pair(
                &format!(
                    "SELECT p.slug AS first, n.slug AS second FROM public.poems r \
                     CROSS JOIN LATERAL (SELECT q.slug FROM public.poems q WHERE q.poet_id = r.poet_id \
                       AND q.id < r.id AND q.recension_of_id IS NULL AND NOT q.is_hidden \
                       ORDER BY q.id DESC LIMIT 1) p \
                     CROSS JOIN LATERAL (SELECT q.slug FROM public.poems q WHERE q.poet_id = r.poet_id \
                       AND q.id > r.id AND q.recension_of_id IS NULL AND NOT q.is_hidden \
                       ORDER BY q.id LIMIT 1) n \
                     WHERE {unshown} ORDER BY r.id LIMIT 1"
                ),
                &[],
            )
            .await
        else {
            continue;
        };
        let earlier = h.get(&format!("/v1/poems/{before}")).await.json();
        assert_eq!(earlier["data"]["next"]["slug"], after.as_str(), "{unshown}");
        let later = h.get(&format!("/v1/poems/{after}")).await.json();
        assert_eq!(later["data"]["prev"]["slug"], before.as_str(), "{unshown}");
    }
}
