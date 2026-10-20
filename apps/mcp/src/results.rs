use serde_json::Value;

use crate::api::Failure;
use crate::constants::PROD_SITE_URL;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Poem,
    Poet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {
    Poem,
    Poet,
    Term,
    Feeling,
    Catalog,
}

impl Subject {
    fn noun(self) -> &'static str {
        match self {
            Self::Poem => "poem",
            Self::Poet => "poet",
            Self::Term => "term",
            Self::Feeling => "feeling",
            Self::Catalog => "catalog entry",
        }
    }

    fn slug_sources(self) -> &'static str {
        match self {
            Self::Poem => "Get valid slugs from search, list_poems, or random_poem.",
            Self::Poet => "Get valid slugs from list_poets or search.",
            Self::Term => "Get valid slugs from list_terms.",
            Self::Feeling => "Get valid slugs from list_feelings.",
            Self::Catalog => "Get valid slugs from list_terms, list_poets, or search.",
        }
    }
}

pub fn link(kind: Kind, slug: &str) -> String {
    match kind {
        Kind::Poem => format!("{PROD_SITE_URL}/poems/{slug}"),
        Kind::Poet => format!("{PROD_SITE_URL}/poets/{slug}"),
    }
}

pub fn add_link(object: &mut Value, kind: Kind) {
    let Some(map) = object.as_object_mut() else {
        return;
    };
    let Some(slug) = map.get("slug").and_then(Value::as_str) else {
        return;
    };
    let url = link(kind, slug);
    map.insert("url".into(), Value::String(url));
}

fn link_at(response: &mut Value, pointer: &str, kind: Kind) {
    if let Some(object) = response.pointer_mut(pointer) {
        add_link(object, kind);
    }
}

fn each_at<'a>(response: &'a mut Value, pointer: &str) -> impl Iterator<Item = &'a mut Value> {
    response
        .pointer_mut(pointer)
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
}

fn link_poems_at(response: &mut Value, pointer: &str) {
    for poem in each_at(response, pointer) {
        add_link(poem, Kind::Poem);
        link_at(poem, "/poet", Kind::Poet);
    }
}

fn link_poets_at(response: &mut Value, pointer: &str) {
    for poet in each_at(response, pointer) {
        add_link(poet, Kind::Poet);
    }
}

pub fn shape_search(response: &mut Value) {
    link_poems_at(response, "/poems/data");
    link_poets_at(response, "/poets/data");
}

pub fn shape_poem_list(response: &mut Value) {
    link_poems_at(response, "/data");
}

pub fn shape_poem(response: &mut Value, max_chars: usize) {
    if let Some(data) = response.get_mut("data").and_then(Value::as_object_mut) {
        data.remove("keywords");
    }
    link_at(response, "/data", Kind::Poem);
    link_at(response, "/data/poet", Kind::Poet);
    for pointer in ["/data/next", "/data/prev", "/data/recensionOf"] {
        link_at(response, pointer, Kind::Poem);
    }
    for recension in each_at(response, "/data/recensions") {
        add_link(recension, Kind::Poem);
    }
    link_poems_at(response, "/data/relatedPoems");
    cap_poem(response, max_chars);
}

pub fn shape_poet_list(response: &mut Value) {
    link_poets_at(response, "/data");
}

pub fn shape_poet(response: &mut Value) {
    link_at(response, "/data", Kind::Poet);
}

pub fn shape_feeling_verses(response: &mut Value) {
    for verse in each_at(response, "/data") {
        link_at(verse, "/poem", Kind::Poem);
        link_at(verse, "/poet", Kind::Poet);
    }
}

fn verse_chars(verse: &Value) -> usize {
    verse
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|part| part.chars().count())
        .fold(0, usize::saturating_add)
}

pub fn cap_poem(response: &mut Value, max_chars: usize) {
    let Some(data) = response.get_mut("data").and_then(Value::as_object_mut) else {
        return;
    };
    let Some(verses) = data.get("verses").and_then(Value::as_array) else {
        return;
    };
    let mut used: usize = 0;
    let shown = verses
        .iter()
        .take_while(|verse| {
            used = used.saturating_add(verse_chars(verse));
            used <= max_chars
        })
        .count();
    if shown == verses.len() {
        return;
    }
    let kept: Vec<Value> = verses.iter().take(shown).cloned().collect();
    let total = data
        .get("verseCount")
        .and_then(Value::as_u64)
        .map_or_else(|| verses.len().to_string(), |count| count.to_string());
    let url = data
        .get("slug")
        .and_then(Value::as_str)
        .map(|slug| link(Kind::Poem, slug));
    let note = match url {
        Some(url) => format!(
            "This result shows {shown} of {total} verses, to stay within the size that assistants accept. The whole poem is at {url}."
        ),
        None => format!(
            "This result shows {shown} of {total} verses, to stay within the size that assistants accept."
        ),
    };
    data.insert("verses".into(), Value::Array(kept));
    data.insert("versesShown".into(), Value::from(shown));
    data.insert("note".into(), Value::String(note));
}

pub fn failure_text(failure: &Failure, subject: Subject) -> String {
    match failure {
        Failure::RateLimited {
            retry_after: Some(1),
        } => "Qafiyah is refusing more requests for now. Try again in 1 second.".into(),
        Failure::RateLimited {
            retry_after: Some(seconds),
        } => format!("Qafiyah is refusing more requests for now. Try again in {seconds} seconds."),
        Failure::RateLimited { retry_after: None } => {
            "Qafiyah is refusing more requests for now. Try again later.".into()
        }
        Failure::BadRequest(Some(detail)) => {
            format!("The Qafiyah API refused a parameter: {detail}")
        }
        Failure::BadRequest(None) => "The Qafiyah API refused a parameter.".into(),
        Failure::NotFound => format!(
            "No {} has that slug. {}",
            subject.noun(),
            subject.slug_sources()
        ),
        Failure::Unavailable => {
            "The Qafiyah catalog is unavailable for the moment. Try again shortly.".into()
        }
    }
}

pub fn bad_slug_text(slug: &str, subject: Subject) -> String {
    let rule = match subject {
        Subject::Poem | Subject::Poet => "A poem or poet slug is four letters, such as Abcd.",
        Subject::Term | Subject::Feeling | Subject::Catalog => {
            "A term or feeling slug is lowercase letters and hyphens, such as tawil."
        }
    };
    format!(
        "\"{slug}\" is not a valid {} slug. {rule} {}",
        subject.noun(),
        subject.slug_sources()
    )
}

pub fn moved(kind: Kind, slug: &str, tool: &str) -> Value {
    serde_json::json!({
        "moved": true,
        "slug": slug,
        "url": link(kind, slug),
        "note": format!("This slug moved. Call {tool} again with the new slug {slug}."),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::api::Failure;

    #[test]
    fn a_link_is_the_website_page_of_the_slug() {
        assert_eq!(link(Kind::Poem, "Abcd"), "https://qafiyah.com/poems/Abcd");
        assert_eq!(link(Kind::Poet, "Wxyz"), "https://qafiyah.com/poets/Wxyz");
    }

    #[test]
    fn an_item_without_a_slug_gets_no_link() {
        let mut item = json!({ "title": "x" });
        add_link(&mut item, Kind::Poem);
        assert_eq!(item, json!({ "title": "x" }));
        let mut number = json!({ "slug": 7 });
        add_link(&mut number, Kind::Poem);
        assert_eq!(number, json!({ "slug": 7 }));
    }

    fn poem(max_chars: usize) -> Value {
        let mut response = json!({ "data": {
            "slug": "Abcd",
            "verseCount": 3,
            "verses": [["aaaa", "bbbb"], ["cccc", "dddd"], ["eeee", "ffff"]],
        }});
        cap_poem(&mut response, max_chars);
        response
    }

    #[test]
    fn a_long_poem_keeps_whole_verses_under_the_cap_and_says_so() {
        let response = poem(16);
        assert_eq!(
            response["data"]["verses"],
            json!([["aaaa", "bbbb"], ["cccc", "dddd"]])
        );
        assert_eq!(response["data"]["versesShown"], 2);
        let note = response["data"]["note"].as_str().expect("a note");
        for part in ["2", "3", "https://qafiyah.com/poems/Abcd"] {
            assert!(note.contains(part), "{note}");
        }
    }

    #[test]
    fn a_poem_result_leaves_out_the_keywords_that_repeat_its_text() {
        let mut response = json!({ "data": {
            "slug": "Abcd",
            "keywords": "every, word, of, the, poem, again",
            "verseCount": 1,
            "verses": [["a", "b"]],
        }});
        shape_poem(&mut response, 1_000);
        assert!(response["data"].get("keywords").is_none());
        assert_eq!(response["data"]["verses"], json!([["a", "b"]]));
    }

    #[test]
    fn a_short_poem_is_left_as_it_is() {
        let response = poem(1000);
        assert_eq!(response["data"]["verses"].as_array().map(Vec::len), Some(3));
        assert!(response["data"].get("versesShown").is_none());
        assert!(response["data"].get("note").is_none());
    }

    #[test]
    fn a_first_verse_over_the_cap_leaves_no_verses() {
        let response = poem(3);
        assert_eq!(response["data"]["verses"], json!([]));
        assert_eq!(response["data"]["versesShown"], 0);
        assert!(response["data"]["note"].is_string());
    }

    #[test]
    fn a_refusal_says_when_to_try_again_and_blames_no_address() {
        let refusal =
            |retry_after| failure_text(&Failure::RateLimited { retry_after }, Subject::Catalog);
        let later = refusal(None);
        assert!(!later.chars().any(|c| c.is_ascii_digit()), "{later}");
        assert!(refusal(Some(30)).ends_with("in 30 seconds."));
        assert!(refusal(Some(1)).ends_with("in 1 second."));
        for text in [later, refusal(Some(30)), refusal(Some(1))] {
            assert!(!text.contains("address"), "{text}");
        }
    }

    #[test]
    fn a_missing_slug_names_the_tools_that_give_its_kind_of_slug() {
        for (subject, tools, others) in [
            (
                Subject::Poem,
                &["search", "list_poems", "random_poem"][..],
                &["list_terms", "list_feelings"][..],
            ),
            (
                Subject::Poet,
                &["list_poets", "search"],
                &["list_terms", "list_feelings"],
            ),
            (
                Subject::Feeling,
                &["list_feelings"],
                &["list_terms", "list_poets", "search"],
            ),
        ] {
            let text = failure_text(&Failure::NotFound, subject);
            assert!(text.contains(subject.noun()), "{text}");
            for tool in tools {
                assert!(text.contains(tool), "{text}");
            }
            for tool in others {
                assert!(!text.contains(tool), "{text}");
            }
        }
    }

    #[test]
    fn a_bad_feeling_slug_points_to_list_feelings() {
        let text = bad_slug_text("A/b", Subject::Feeling);
        assert!(text.contains("list_feelings"), "{text}");
        assert!(!text.contains("list_terms"), "{text}");
    }

    #[test]
    fn a_bad_request_carries_the_detail_when_there_is_one() {
        let text = failure_text(
            &Failure::BadRequest(Some("page must be a positive integer".into())),
            Subject::Catalog,
        );
        assert!(text.contains("page must be a positive integer"), "{text}");
    }
}
