use axum::http::StatusCode;
use reqwest::Method;
use serde_json::{Value, json};

use qafiyah_api::domain::search;
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

async fn poem_hits(es: &Es, index: &str, q: &str) -> Vec<Value> {
    searched_poems(es, index, q, false).await
}

#[expect(clippy::expect_used, reason = "a failed search is a failed test")]
async fn searched_poems(es: &Es, index: &str, q: &str, exact: bool) -> Vec<Value> {
    let body = poem_search_body(&PoemSearchParams {
        q: q.into(),
        page: 1,
        exact,
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
async fn a_three_letter_word_is_not_stretched_to_a_poet_name_by_a_typo() {
    let Some(h) = harness().await else {
        return;
    };
    assert_eq!(searched_poets(&h, "موت").await, Vec::<String>::new());
    assert_eq!(listed_poets(&h, "موت").await, Vec::<String>::new());
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
    assert_first_poet(&h, "أبوقحفان", "PGxq").await;
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

fn searching(es: Es, index: String) -> Es {
    let mut es = es;
    es.poems_alias = index;
    es
}

#[expect(clippy::expect_used, reason = "a failed search is a failed test")]
async fn shown_snippets(es: &Es, q: &str, exact: bool) -> Vec<String> {
    search::search_poems(
        es,
        &PoemSearchParams {
            q: q.into(),
            page: 1,
            exact,
            ..PoemSearchParams::default()
        },
    )
    .await
    .expect("a search")
    .hits
    .into_iter()
    .map(|hit| hit.snippet)
    .collect()
}

const OPENING_VERSE: &str = "سرى الطيف في الظلام*فاستهام فؤادي";

#[tokio::test]
async fn a_word_deep_in_a_long_poem_is_highlighted_from_stored_offsets_whatever_search_finds_it() {
    let Some(admin) = admin() else {
        return;
    };
    let filler = "سرى الطيف في الظلام فاستهام فؤادي*وطال على طول البعاد سهادي*".repeat(20);
    let docs = [poem(
        1,
        "Long",
        "سرى الطيف",
        &format!("{filler}شربت ماء الغدير*ووقفت على الأطلال والليل ساكن"),
    )];
    let reanalysis_cap = json!({ "index.highlight.max_analyzed_offset": 40 });
    admin
        .with_poems_and_settings(&docs, &reanalysis_cap, |es, index| async move {
            let es = searching(es, index);
            for (q, exact, shown) in [
                (
                    "الأطلال",
                    false,
                    "شربت ماء الغدير*ووقفت على <mark>الأطلال</mark> والليل ساكن",
                ),
                (
                    "ليل",
                    false,
                    "شربت ماء الغدير*ووقفت على الأطلال <mark>والليل</mark> ساكن",
                ),
                (
                    "ماء",
                    false,
                    "شربت <mark>ماء</mark> الغدير*ووقفت على الأطلال والليل ساكن",
                ),
                (
                    "على الأطلال",
                    true,
                    "شربت ماء الغدير*ووقفت <mark>على الأطلال</mark> والليل ساكن",
                ),
            ] {
                assert_eq!(shown_snippets(&es, q, exact).await, [shown], "{q}");
            }
        })
        .await;
}

#[tokio::test]
async fn a_ranked_search_for_a_quoted_line_shows_the_verse_holding_it_with_the_line_marked_as_one()
{
    let Some(admin) = admin() else {
        return;
    };
    let docs = [poem(
        1,
        "Dyar",
        "منازل",
        "ذكرت عبلة والرماح نواهل*فهاج الشوق في قلبي*يا دار عبلة بالجواء تكلمي*وعمي صباحا واسلمي",
    )];
    admin
        .with_poems(&docs, |es, index| async move {
            let es = searching(es, index);
            assert_eq!(
                shown_snippets(&es, "يا دار عبلة", false).await,
                ["<mark>يا دار عبلة</mark> بالجواء تكلمي*وعمي صباحا واسلمي"]
            );
        })
        .await;
}

#[tokio::test]
async fn a_ranked_search_marks_the_word_as_written_when_only_its_stem_matches() {
    let Some(admin) = admin() else {
        return;
    };
    let docs = [poem(
        1,
        "Lyll",
        "سهر",
        &format!("{OPENING_VERSE}*سهرت والليل طويل*أعد النجوم"),
    )];
    admin
        .with_poems(&docs, |es, index| async move {
            let es = searching(es, index);
            assert_eq!(
                shown_snippets(&es, "ليل", false).await,
                ["سهرت <mark>والليل</mark> طويل*أعد النجوم"]
            );
        })
        .await;
}

#[tokio::test]
async fn a_search_made_only_of_stopwords_marks_them_in_the_verse_it_shows() {
    let Some(admin) = admin() else {
        return;
    };
    let docs = [poem(
        1,
        "Tayf",
        "طيف",
        &format!("{OPENING_VERSE}*فقلت من أنت يا طيف*فقال أنا الهوى"),
    )];
    admin
        .with_poems(&docs, |es, index| async move {
            let es = searching(es, index);
            assert_eq!(
                shown_snippets(&es, "من أنت", false).await,
                ["فقلت <mark>من أنت</mark> يا طيف*فقال أنا الهوى"]
            );
        })
        .await;
}

#[tokio::test]
async fn a_ranked_search_with_a_standalone_hamza_marks_the_word_as_typed_and_not_its_folded_twin() {
    let Some(admin) = admin() else {
        return;
    };
    let docs = [poem(
        1,
        "Nahr",
        "نهر",
        &format!("{OPENING_VERSE}*شربت ماء النهر*ما كان لي عندها"),
    )];
    admin
        .with_poems(&docs, |es, index| async move {
            let es = searching(es, index);
            assert_eq!(
                shown_snippets(&es, "ماء", false).await,
                ["شربت <mark>ماء</mark> النهر*ما كان لي عندها"]
            );
        })
        .await;
}

#[tokio::test]
async fn a_poem_found_only_by_its_title_shows_its_opening_verse_unmarked() {
    let Some(admin) = admin() else {
        return;
    };
    let docs = [poem(
        1,
        "Ttle",
        "حنين المسافر",
        &format!("أحن إلى بيت بعيد*وأمي تنتظر الغياب*{OPENING_VERSE}"),
    )];
    admin
        .with_poems(&docs, |es, index| async move {
            let es = searching(es, index);
            assert_eq!(
                shown_snippets(&es, "المسافر", false).await,
                ["أحن إلى بيت بعيد*وأمي تنتظر الغياب"]
            );
        })
        .await;
}

#[tokio::test]
async fn a_scratch_index_carries_the_extra_settings_it_was_created_with() {
    let Some(admin) = admin() else {
        return;
    };
    let endpoint = admin.endpoint();
    let docs = [poem(1, "Sttg", "سرى الطيف", OPENING_VERSE)];
    let reanalysis_cap = json!({ "index.highlight.max_analyzed_offset": 40 });
    admin
        .with_poems_and_settings(&docs, &reanalysis_cap, |_, index| async move {
            let settings: Value = endpoint
                .request(Method::GET, &format!("/{index}/_settings"))
                .send()
                .await
                .expect("the scratch index settings")
                .json()
                .await
                .expect("a settings report");
            assert_eq!(
                settings[&index]["settings"]["index"]["highlight"]["max_analyzed_offset"],
                "40"
            );
        })
        .await;
}

#[tokio::test]
async fn a_name_pasted_from_a_pdf_finds_the_poet() {
    let Some(h) = harness().await else {
        return;
    };
    let pasted = "\u{FECB}\u{FEE8}\u{FE98}\u{FEAE}\u{FE93} \u{FE91}\u{FEE6} \u{FEB7}\u{FEAA}\u{FE8D}\u{FEA9}";
    assert_first_poet(&h, pasted, "imHZ").await;
}

fn dated(id: i32, slug: &str, title: &str, content: &str, era: &str) -> Value {
    let mut doc = poem(id, slug, title, content);
    if let Some(fields) = doc.as_object_mut() {
        fields.insert("eraSlug".into(), json!(era));
    }
    doc
}

fn quoted_line() -> [Value; 3] {
    [
        dated(
            1,
            "Jahl",
            "لعمرك ما الأيام إلا معارة",
            "لعمرك ما الأيام إلا معارة*فما اسطعت من معروفها فتزود*ستبدي لك الأيام ما كنت جاهلا*ويأتيك بالأخبار من لم تزود",
            "jahili",
        ),
        dated(
            2,
            "Hdth",
            "ستبدي لك الأيام ما كنت جاهلا",
            "ستبدي لك الأيام ما كنت جاهلا*وتعرف من يبقى على العهد صادقا",
            "hadith",
        ),
        dated(
            3,
            "Mmlk",
            "تعلم فإن الدهر فيه عجائب",
            "تعلم فإن الدهر فيه عجائب*وقد قال من قبلي ستبدي لك الأيام ما كنت جاهلا*فخذها حكمة",
            "mamluki",
        ),
    ]
}

#[tokio::test]
async fn a_line_quoted_verbatim_ranks_the_oldest_classical_poem_first_from_three_words() {
    let Some(admin) = admin() else {
        return;
    };
    admin
        .with_poems(&quoted_line(), |es, index| async move {
            for q in ["ستبدي لك الأيام ما كنت جاهلا", "ستبدي لك الأيام"]
            {
                let found = slugs(&poem_hits(&es, &index, q).await);
                assert_eq!(found, ["Jahl", "Mmlk", "Hdth"], "{q}");
            }
            for q in ["ستبدي لك", "ستبدي لك ؟", "ستبدي لك ..."] {
                let found = slugs(&poem_hits(&es, &index, q).await);
                assert_eq!(
                    found.first().map(String::as_str),
                    Some("Hdth"),
                    "{q}: {found:?}"
                );
            }
        })
        .await;
}

fn alternate(id: i32, slug: &str, primary_id: i32, doc: Value) -> Value {
    let mut doc = doc;
    if let Some(fields) = doc.as_object_mut() {
        fields.insert("id".into(), json!(id));
        fields.insert("slug".into(), json!(slug));
        fields.insert("primaryId".into(), json!(primary_id));
        fields.insert("isPrimary".into(), json!(false));
    }
    doc
}

#[tokio::test]
async fn a_line_found_only_in_an_alternate_reading_keeps_the_era_rank_of_its_poem() {
    let Some(admin) = admin() else {
        return;
    };
    let docs = [
        dated(
            1,
            "Jahl",
            "هل غادر الشعراء من متردم",
            "هل غادر الشعراء من متردم*أم هل عرفت الدار بعد توهم",
            "jahili",
        ),
        alternate(
            2,
            "JhlB",
            1,
            dated(
                0,
                "",
                "هل غادر الشعراء من متردم",
                "هل غادر الشعراء من متردم*ولقد ذكرتك والرماح نواهل",
                "jahili",
            ),
        ),
        dated(
            3,
            "Hdth",
            "ولقد ذكرتك والرماح نواهل",
            "ولقد ذكرتك والرماح نواهل*مني وبيض الهند تقطر من دمي",
            "hadith",
        ),
    ];
    admin
        .with_poems(&docs, |es, index| async move {
            let found = slugs(&poem_hits(&es, &index, "ولقد ذكرتك والرماح نواهل").await);
            assert_eq!(found, ["JhlB", "Hdth"]);
        })
        .await;
}

fn highlighted(hits: &[Value]) -> Vec<&str> {
    hits.iter()
        .map(|hit| {
            hit.get("highlight")
                .and_then(|highlight| highlight.get("content"))
                .and_then(|content| content.get(0))
                .and_then(Value::as_str)
                .unwrap_or_default()
        })
        .collect()
}

#[tokio::test]
async fn an_exact_search_of_three_words_ranks_the_oldest_classical_poem_first_and_marks_the_line() {
    let Some(admin) = admin() else {
        return;
    };
    admin
        .with_poems(&quoted_line(), |es, index| async move {
            let hits = searched_poems(&es, &index, "ستبدي لك الأيام ما كنت جاهلا", true).await;
            assert_eq!(slugs(&hits), ["Jahl", "Mmlk", "Hdth"]);
            for marked in highlighted(&hits) {
                assert!(marked.contains("<mark>"), "{marked}");
            }
        })
        .await;
}

#[tokio::test]
async fn an_exact_search_keeps_a_standalone_hamza_apart_and_marks_the_word_as_typed() {
    let Some(admin) = admin() else {
        return;
    };
    let docs = [
        poem(1, "Watr", "ظمأ", "شربت ماء النهر*وعدت إلى الدار"),
        poem(2, "Negn", "سؤال", "ما كان لي عندها*وعد ولا دار"),
    ];
    admin
        .with_poems(&docs, |es, index| async move {
            let hits = searched_poems(&es, &index, "ماء", true).await;
            assert_eq!(slugs(&hits), ["Watr"]);
            assert_eq!(
                highlighted(&hits),
                ["شربت <mark>ماء</mark> النهر*وعدت إلى الدار"]
            );
            let found = slugs(&searched_poems(&es, &index, "ما", true).await);
            assert_eq!(found, ["Negn"]);
        })
        .await;
}

#[tokio::test]
async fn punctuation_in_the_query_keeps_the_exact_title_first() {
    let Some(admin) = admin() else {
        return;
    };
    let docs = [
        poem(1, "Qalb", "يا قلب", "يا قلب صبرا على ما كان*فالدهر يومان"),
        poem(
            2,
            "Rpts",
            "يا قلب يا قلب كم تصادر",
            "يا قلب يا قلب كم تصادر*يا قلب يا قلب لا تحزن*يا قلب",
        ),
    ];
    admin
        .with_poems(&docs, |es, index| async move {
            for q in ["يا قلب", "يا قلب ؟", "يا قلب؟", "، يا قلب", "(يا قلب)"]
            {
                let found = slugs(&poem_hits(&es, &index, q).await);
                assert_eq!(
                    found.first().map(String::as_str),
                    Some("Qalb"),
                    "{q}: {found:?}"
                );
            }
        })
        .await;
}

#[tokio::test]
async fn an_exact_search_finds_a_poem_by_a_title_that_is_not_in_its_text() {
    let Some(admin) = admin() else {
        return;
    };
    let docs = [
        poem(
            1,
            "Ttle",
            "حنين المسافر",
            "أحن إلى بيت بعيد*وأمي تنتظر الغياب",
        ),
        poem(2, "Othr", "سرى الطيف", "سرى الطيف ليلا*فاستهام فؤادي"),
    ];
    admin
        .with_poems(&docs, |es, index| async move {
            let found = slugs(&searched_poems(&es, &index, "حنين المسافر", true).await);
            assert_eq!(found, ["Ttle"]);
        })
        .await;
}
