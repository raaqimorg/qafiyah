use axum::http::StatusCode;
use serde_json::{Value, json};

use qafiyah_api::es::client::Es;
use qafiyah_api::es::query::{PoemSearchParams, poem_search_body};

use crate::{Harness, admin, harness};

fn poem(id: i32, slug: &str, title: &str, content: &str) -> Value {
    json!({
        "id": id, "slug": slug, "title": title, "content": content,
        "poetName": "شاعر", "titleDisplay": title, "poetNameDisplay": "شاعر",
        "poetSlug": "Pppp", "poetHasAvatar": false, "poetIsAnonymous": false,
        "eraSlug": "hadith", "eraName": "حديث", "meterSlug": "altawil", "meterName": "الطويل",
        "themeSlug": "alnasib", "rhymeSlug": "meem", "poemTypeSlug": "amudi", "collectionSlug": "",
        "primaryId": id, "isPrimary": true,
    })
}

#[expect(clippy::expect_used, reason = "a failed search is a failed test")]
async fn poem_hits(es: &Es, index: &str, q: &str) -> Vec<Value> {
    let body = poem_search_body(&PoemSearchParams {
        q: q.into(),
        page: 1,
        ..PoemSearchParams::default()
    });
    let response = es.search(index, &body).await.expect("a search");
    response
        .get("hits")
        .and_then(|hits| hits.get("hits"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn slugs(hits: &[Value]) -> Vec<String> {
    hits.iter()
        .filter_map(|hit| {
            hit.get("_source")
                .and_then(|source| source.get("slug"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect()
}

fn encoded(q: &str) -> String {
    form_urlencoded::byte_serialize(q.as_bytes()).collect()
}

fn poet_slugs(list: Option<&Value>) -> Vec<String> {
    list.and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|poet| poet.get("slug").and_then(Value::as_str).map(str::to_string))
        .collect()
}

async fn searched_poets(h: &Harness, q: &str) -> Vec<String> {
    let sent = h
        .get(&format!("/v1/search?types[]=poets&q={}", encoded(q)))
        .await;
    assert_eq!(sent.status, StatusCode::OK, "{q}: {}", sent.body);
    poet_slugs(sent.json().get("poets").and_then(|poets| poets.get("data")))
}

async fn listed_poets(h: &Harness, q: &str) -> Vec<String> {
    let sent = h.get(&format!("/v1/poets?q={}", encoded(q))).await;
    assert_eq!(sent.status, StatusCode::OK, "{q}: {}", sent.body);
    poet_slugs(sent.json().get("data"))
}

async fn assert_first_poet(h: &Harness, q: &str, slug: &str) {
    assert_eq!(
        searched_poets(h, q).await.first().map(String::as_str),
        Some(slug),
        "search: {q}"
    );
    assert_eq!(
        listed_poets(h, q).await.first().map(String::as_str),
        Some(slug),
        "poets list: {q}"
    );
}

const FILLER_WORDS: [&str; 10] = [
    "الليل",
    "القمر",
    "النجوم",
    "الربيع",
    "السحاب",
    "المطر",
    "الريح",
    "الصحراء",
    "النخيل",
    "الطريق",
];

fn filler() -> Vec<Value> {
    (100..120)
        .zip(FILLER_WORDS.iter().cycle())
        .zip([true, false].iter().cycle())
        .map(|((id, word), particles)| {
            let content = if *particles {
                format!("يا {word} لي عندك وعد*أطل {word} لها على الدار")
            } else {
                format!("يا {word} من بعيد*أطل {word} على الدار")
            };
            poem(
                id,
                &format!("F{id}"),
                &format!("{word} وحيدا {id}"),
                &content,
            )
        })
        .collect()
}

#[tokio::test]
async fn the_words_as_typed_in_a_poem_outrank_one_stemmed_word_in_another_title() {
    let Some(admin) = admin() else {
        return;
    };
    let mut docs = vec![
        poem(
            1,
            "Aaaa",
            "سرى الطيف ليلا فاستهام فؤادي",
            "سرى الطيف ليلا فاستهام فؤادي*وطال على طول البعاد سهادي*من لي لها والدار تنأى بأهلها*ومن لي بقلب لا يذوب ودادي",
        ),
        poem(
            2,
            "Bbbb",
            "ما لي وللرقباء ما لي",
            "ما لي وللرقباء ما لي*إذ لا اصطبار لها على*مرض وحال دون حال",
        ),
    ];
    docs.extend(filler());
    admin
        .with_poems(&docs, |es, index| async move {
            let found = slugs(&poem_hits(&es, &index, "من لي لها").await);
            assert_eq!(found.first().map(String::as_str), Some("Aaaa"), "{found:?}");
        })
        .await;
}

#[tokio::test]
async fn a_one_letter_typo_in_a_poet_name_still_finds_the_poet() {
    let Some(h) = harness().await else {
        return;
    };
    assert_first_poet(&h, "عنترة بن شداذ", "imHZ").await;
    assert_first_poet(&h, "عمرو بن كلسوم", "qdtv").await;
}

#[tokio::test]
async fn a_line_of_verse_lists_no_poet() {
    let Some(h) = harness().await else {
        return;
    };
    for q in [
        "قفا نبك من ذكرى حبيب ومنزل",
        "الخيل والليل والبيداء تعرفني",
        "على قدر أهل العزم",
    ] {
        assert!(searched_poets(&h, q).await.is_empty(), "{q}");
    }
}

#[tokio::test]
async fn the_inflected_and_contracted_forms_of_a_name_find_the_same_poet() {
    let Some(h) = harness().await else {
        return;
    };
    for (q, slug) in [
        ("امرئ القيس", "iNUk"),
        ("امرأ القيس", "iNUk"),
        ("عنترة ابن شداد", "imHZ"),
        ("ذي الإصبع العدواني", "ndfb"),
    ] {
        assert_first_poet(&h, q, slug).await;
    }
}

#[tokio::test]
async fn a_name_written_joined_finds_the_poet_whose_name_is_written_apart() {
    let Some(h) = harness().await else {
        return;
    };
    assert_first_poet(&h, "عبدالله بن العجلان النهدي", "FzZx").await;
}

#[tokio::test]
async fn a_word_carrying_a_mark_the_folding_leaves_is_found_by_its_bare_letters_and_highlighted_whole()
 {
    let Some(admin) = admin() else {
        return;
    };
    let docs = [
        poem(1, "Cccc", "لا تبتئس", "قال الْحٓرُّ لا تبتئسْ*فإن الدهر ذو غير"),
        poem(2, "Dddd", "سرى الطيف", "سرى الطيف ليلا*فاستهام فؤادي"),
    ];
    admin
        .with_poems(&docs, |es, index| async move {
            let hits = poem_hits(&es, &index, "الحر").await;
            let first = hits.first().expect("the poem holding الْحٓرُّ");
            assert_eq!(first["_source"]["slug"], "Cccc");
            assert_eq!(
                first["highlight"]["content"][0],
                "قال <mark>الْحٓرُّ</mark> لا تبتئسْ*فإن الدهر ذو غير"
            );
        })
        .await;
}
