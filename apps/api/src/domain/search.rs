use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::domain::{EraRef, MeterRef, PoetRef};
use crate::error::AppError;
use crate::es::client::Es;
use crate::es::query::{PoemSearchParams, PoetSearchParams, poem_search_body, poet_search_body};
use crate::js;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PoemKind {
    Poem,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PoetKind {
    Poet,
}

#[derive(Serialize, ToSchema)]
pub struct PoemResult {
    #[serde(rename = "type")]
    #[schema(value_type = PoemKind)]
    pub kind: &'static str,
    pub title: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "TnKK")]
    pub slug: String,
    pub snippet: String,
    pub poet: PoetRef,
    pub meter: MeterRef,
    pub era: EraRef,
    #[serde(serialize_with = "js::serialize_number")]
    pub relevance: f64,
}

#[derive(Serialize, ToSchema)]
pub struct PoetResult {
    #[serde(rename = "type")]
    #[schema(value_type = PoetKind)]
    pub kind: &'static str,
    pub name: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "yoFB")]
    pub slug: String,
    pub era: EraRef,
    #[serde(serialize_with = "js::serialize_number")]
    pub relevance: f64,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoetListItem {
    pub name: String,
    #[schema(pattern = "^[a-zA-Z]{4}$", example = "yoFB")]
    pub slug: String,
    #[schema(example = 42)]
    pub poems_count: i64,
}

pub struct Page<T> {
    pub hits: Vec<T>,
    pub total: u32,
}

fn text(source: &Value, key: &str) -> String {
    source
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn display(source: &Value, display_key: &str, plain_key: &str) -> String {
    match source.get(display_key).and_then(Value::as_str) {
        Some(value) => value.to_string(),
        None => text(source, plain_key),
    }
}

fn total_hits(response: &Value) -> u32 {
    let total = response.get("hits").and_then(|hits| hits.get("total"));
    let raw = total
        .and_then(Value::as_u64)
        .or_else(|| {
            total
                .and_then(|value| value.get("value"))
                .and_then(Value::as_u64)
        })
        .unwrap_or(0);
    u32::try_from(raw).unwrap_or(u32::MAX)
}

fn hits(response: &Value) -> Vec<&Value> {
    response
        .get("hits")
        .and_then(|hits| hits.get("hits"))
        .and_then(Value::as_array)
        .map(|hits| hits.iter().collect())
        .unwrap_or_default()
}

fn score(hit: &Value) -> f64 {
    hit["_score"].as_f64().unwrap_or(0.0)
}

const MARK_OPEN: &str = "<mark>";
const MARK_CLOSE: &str = "</mark>";

fn longest_mark_span(hemistich: Option<&&str>) -> usize {
    let Some(text) = hemistich else {
        return 0;
    };
    let mut longest = 0;
    let mut rest = *text;
    while let Some(open_at) = rest.find(MARK_OPEN) {
        let Some(after_open) = rest.get(open_at.saturating_add(MARK_OPEN.len())..) else {
            break;
        };
        let Some(close_at) = after_open.find(MARK_CLOSE) else {
            break;
        };
        if let Some(marked) = after_open.get(..close_at) {
            longest = longest.max(marked.encode_utf16().count());
        }
        let Some(remainder) = after_open.get(close_at.saturating_add(MARK_CLOSE.len())..) else {
            break;
        };
        rest = remainder;
    }
    longest
}

fn leading_verse(content: &str) -> String {
    content.split('*').take(2).collect::<Vec<&str>>().join("*")
}

fn ends_inside_mark(text: &str) -> bool {
    match (text.rfind(MARK_OPEN), text.rfind(MARK_CLOSE)) {
        (Some(open_at), Some(close_at)) => open_at > close_at,
        (Some(_), None) => true,
        _ => false,
    }
}

fn balanced_hemistichs(highlighted: &str) -> Vec<String> {
    let mut open = false;
    highlighted
        .split('*')
        .map(|hemistich| {
            let mut balanced = if open {
                format!("{MARK_OPEN}{hemistich}")
            } else {
                hemistich.to_string()
            };
            open = ends_inside_mark(&balanced);
            if open {
                balanced.push_str(MARK_CLOSE);
            }
            balanced
        })
        .collect()
}

fn poem_snippet(highlight: Option<&str>, content: &str) -> String {
    if let Some(highlighted) = highlight {
        let balanced = balanced_hemistichs(highlighted);
        let hemistichs: Vec<&str> = balanced.iter().map(String::as_str).collect();
        let mut best_start = None;
        let mut best_span = 0;
        let mut index = 0;
        while index < hemistichs.len() {
            let span = longest_mark_span(hemistichs.get(index))
                .max(longest_mark_span(hemistichs.get(index.saturating_add(1))));
            if span > best_span {
                best_span = span;
                best_start = Some(index);
            }
            index = index.saturating_add(2);
        }
        if let Some(start) = best_start {
            let end = start.saturating_add(2).min(hemistichs.len());
            if let Some(verse) = hemistichs.get(start..end) {
                return verse.join("*");
            }
        }
    }
    leading_verse(content)
}

fn highlighted_content(hit: &Value) -> Option<&str> {
    hit.get("highlight")
        .and_then(|highlight| highlight.get("content"))
        .and_then(Value::as_array)?
        .first()?
        .as_str()
        .filter(|value| !value.is_empty())
}

pub async fn search_poems(
    es: &Es,
    params: &PoemSearchParams,
) -> Result<Page<PoemResult>, AppError> {
    let response = es
        .search(&es.poems_alias, &poem_search_body(params))
        .await?;
    let hits = hits(&response)
        .into_iter()
        .map(|hit| {
            let source = &hit["_source"];
            PoemResult {
                kind: "poem",
                title: display(source, "titleDisplay", "title"),
                slug: text(source, "slug"),
                snippet: poem_snippet(highlighted_content(hit), &text(source, "content")),
                poet: PoetRef {
                    name: display(source, "poetNameDisplay", "poetName"),
                    slug: text(source, "poetSlug"),
                    has_avatar: source["poetHasAvatar"].as_bool().unwrap_or(false),
                    is_anonymous: source["poetIsAnonymous"].as_bool().unwrap_or(false),
                },
                meter: MeterRef {
                    name: text(source, "meterName"),
                    slug: text(source, "meterSlug"),
                },
                era: EraRef {
                    name: text(source, "eraName"),
                    slug: text(source, "eraSlug"),
                },
                relevance: score(hit),
            }
        })
        .collect();
    Ok(Page {
        hits,
        total: total_hits(&response),
    })
}

pub async fn search_poets(
    es: &Es,
    params: &PoetSearchParams,
) -> Result<Page<PoetResult>, AppError> {
    let response = es
        .search(&es.poets_alias, &poet_search_body(params))
        .await?;
    let hits = hits(&response)
        .into_iter()
        .map(|hit| {
            let source = &hit["_source"];
            PoetResult {
                kind: "poet",
                name: display(source, "nameDisplay", "name"),
                slug: text(source, "slug"),
                era: EraRef {
                    name: text(source, "eraName"),
                    slug: text(source, "eraSlug"),
                },
                relevance: score(hit),
            }
        })
        .collect();
    Ok(Page {
        hits,
        total: total_hits(&response),
    })
}

pub async fn list_poets(
    es: &Es,
    params: &PoetSearchParams,
) -> Result<Page<PoetListItem>, AppError> {
    let response = es
        .search(&es.poets_alias, &poet_search_body(params))
        .await?;
    let hits = hits(&response)
        .into_iter()
        .map(|hit| {
            let source = &hit["_source"];
            PoetListItem {
                name: display(source, "nameDisplay", "name"),
                slug: text(source, "slug"),
                poems_count: source["poemsCount"].as_i64().unwrap_or(0),
            }
        })
        .collect();
    Ok(Page {
        hits,
        total: total_hits(&response),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_verse_with_the_widest_highlight() {
        let highlighted = "one*two <mark>ab</mark>*three*four <mark>abcd</mark>";
        assert_eq!(
            poem_snippet(Some(highlighted), "ignored"),
            "three*four <mark>abcd</mark>"
        );
    }

    #[test]
    fn falls_back_to_the_opening_verse_when_nothing_is_highlighted() {
        assert_eq!(poem_snippet(None, "a*b*c*d"), "a*b");
        assert_eq!(poem_snippet(Some("a*b*c*d"), "x*y*z"), "x*y");
    }

    #[test]
    fn measures_a_span_in_utf16_code_units() {
        assert_eq!(longest_mark_span(Some(&"<mark>ab</mark>")), 2);
        assert_eq!(longest_mark_span(Some(&"<mark>عربي</mark>")), 4);
        assert_eq!(longest_mark_span(Some(&"<mark>\u{1F4DC}</mark>")), 2);
        assert_eq!(longest_mark_span(Some(&"no marks here")), 0);
        assert_eq!(longest_mark_span(Some(&"<mark>unclosed")), 0);
        assert_eq!(longest_mark_span(None), 0);
    }

    #[test]
    fn balances_a_mark_that_spans_the_two_halves_of_a_verse() {
        let highlighted =
            "طلمباتُ الطريق الزراعي*ما تزال في مكانها*«<mark>يا ليلُ،*الصَبُّ</mark> متى غدُه؟";
        assert_eq!(
            poem_snippet(Some(highlighted), "ignored*fallback"),
            "«<mark>يا ليلُ،</mark>*<mark>الصَبُّ</mark> متى غدُه؟"
        );
    }

    #[test]
    fn picks_the_verse_holding_the_longer_part_of_a_mark_that_spans_two_verses() {
        let highlighted = "a*b <mark>cd*efg</mark>*h";
        assert_eq!(
            poem_snippet(Some(highlighted), "ignored*fallback"),
            "<mark>efg</mark>*h"
        );
    }

    #[test]
    fn keeps_the_first_of_two_equal_spans() {
        let highlighted = "<mark>ab</mark>*x*<mark>cd</mark>*y";
        assert_eq!(
            poem_snippet(Some(highlighted), "ignored"),
            "<mark>ab</mark>*x"
        );
    }

    #[test]
    fn prefers_the_display_field_even_when_it_is_empty() {
        let source = serde_json::json!({ "titleDisplay": "", "title": "plain" });
        assert_eq!(display(&source, "titleDisplay", "title"), "");
        let missing = serde_json::json!({ "title": "plain" });
        assert_eq!(display(&missing, "titleDisplay", "title"), "plain");
    }

    #[test]
    fn reads_the_total_in_either_shape() {
        assert_eq!(total_hits(&serde_json::json!({"hits":{"total":7}})), 7);
        assert_eq!(
            total_hits(&serde_json::json!({"hits":{"total":{"value":9}}})),
            9
        );
        assert_eq!(total_hits(&serde_json::json!({"hits":{}})), 0);
    }

    fn poem_hit(source: Value, highlight: Option<&str>, score: f64) -> Value {
        let mut hit = serde_json::json!({ "_source": source, "_score": score });
        if let Some(highlight) = highlight {
            hit["highlight"] = serde_json::json!({ "content": [highlight] });
        }
        hit
    }

    #[test]
    fn a_poem_hit_maps_display_fields_snippets_and_scores() {
        let response = serde_json::json!({ "hits": { "total": { "value": 1 }, "hits": [poem_hit(
            serde_json::json!({ "slug": "TnKK", "title": "plain", "titleDisplay": "vocalized", "content": "a*b*c*d",
                                "poetName": "p", "poetNameDisplay": "P", "poetSlug": "yoFB", "meterName": "m",
                                "meterSlug": "altawil", "eraName": "e", "eraSlug": "abbasi" }),
            Some("a*b <mark>x</mark>*c*d"), 3.5) ] } });
        let hits = hits(&response);
        let hit = hits[0];
        assert_eq!(
            display(&hit["_source"], "titleDisplay", "title"),
            "vocalized"
        );
        assert_eq!(display(&hit["_source"], "poetNameDisplay", "poetName"), "P");
        assert_eq!(
            poem_snippet(highlighted_content(hit), &text(&hit["_source"], "content")),
            "a*b <mark>x</mark>"
        );
        assert_eq!(score(hit).to_bits(), 3.5_f64.to_bits());
        assert_eq!(total_hits(&response), 1);
    }

    #[test]
    fn a_hit_with_missing_fields_maps_to_empty_strings_and_zero() {
        let response = serde_json::json!({ "hits": { "hits": [ { "_source": {} } ] } });
        let hit = hits(&response)[0];
        assert_eq!(text(&hit["_source"], "slug"), "");
        assert_eq!(score(hit).to_bits(), 0.0_f64.to_bits());
        assert!(highlighted_content(hit).is_none());
        assert_eq!(hit["_source"]["poemsCount"].as_i64().unwrap_or(0), 0);
        assert_eq!(total_hits(&response), 0);
        assert!(hits(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn an_empty_highlight_falls_back_to_the_opening_verse() {
        let hit = poem_hit(serde_json::json!({ "content": "x*y*z" }), Some(""), 1.0);
        assert!(highlighted_content(&hit).is_none());
        assert_eq!(poem_snippet(highlighted_content(&hit), "x*y*z"), "x*y");
    }
}
