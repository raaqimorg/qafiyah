use async_trait::async_trait;
use serde_json::Value;

use crate::constants::ES_MAX_RESULT_WINDOW;
use crate::domain::search::{
    Page, PoemResult, PoemSearchParams, PoetListItem, PoetResult, PoetSearchParams, SearchIndex,
    poem_snippet,
};
use crate::domain::{EraRef, MeterRef, PoetRef};
use crate::error::StoreError;
use crate::es::client::Es;
use crate::es::query::{poem_search_body, poet_search_body};

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

fn poem_total(response: &Value) -> u32 {
    response
        .get("aggregations")
        .and_then(|aggregations| aggregations.get("poems"))
        .and_then(|poems| poems.get("value"))
        .and_then(Value::as_u64)
        .map_or_else(
            || total_hits(response),
            |poems| {
                u32::try_from(poems)
                    .unwrap_or(u32::MAX)
                    .min(ES_MAX_RESULT_WINDOW)
            },
        )
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

fn highlighted_content(hit: &Value) -> Option<&str> {
    hit.get("highlight")
        .and_then(|highlight| highlight.get("content"))
        .and_then(Value::as_array)?
        .first()?
        .as_str()
        .filter(|value| !value.is_empty())
}

#[async_trait]
impl SearchIndex for Es {
    async fn search_poems(
        &self,
        params: &PoemSearchParams,
    ) -> Result<Page<PoemResult>, StoreError> {
        let response = self
            .search(&self.poems_alias, &poem_search_body(params))
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
                        name: text(source, "poetNameDisplay"),
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
            total: poem_total(&response),
        })
    }

    async fn search_poets(
        &self,
        params: &PoetSearchParams,
    ) -> Result<Page<PoetResult>, StoreError> {
        let response = self
            .search(&self.poets_alias, &poet_search_body(params))
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

    async fn list_poets(
        &self,
        params: &PoetSearchParams,
    ) -> Result<Page<PoetListItem>, StoreError> {
        let response = self
            .search(&self.poets_alias, &poet_search_body(params))
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
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn counts_poems_from_the_cardinality_aggregation_capped_at_the_result_window() {
        let grouped = serde_json::json!({
            "hits": { "total": { "value": 12 } },
            "aggregations": { "poems": { "value": 9 } },
        });
        assert_eq!(poem_total(&grouped), 9);
        let many = serde_json::json!({
            "hits": { "total": { "value": 10000 } },
            "aggregations": { "poems": { "value": 51234 } },
        });
        assert_eq!(poem_total(&many), ES_MAX_RESULT_WINDOW);
        let browse = serde_json::json!({ "hits": { "total": { "value": 7 } } });
        assert_eq!(poem_total(&browse), 7);
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
                                "poetNameDisplay": "P", "poetSlug": "yoFB", "meterName": "m",
                                "meterSlug": "altawil", "eraName": "e", "eraSlug": "abbasi" }),
            Some("a*b <mark>x</mark>*c*d"), 3.5) ] } });
        let hits = hits(&response);
        let hit = hits[0];
        assert_eq!(
            display(&hit["_source"], "titleDisplay", "title"),
            "vocalized"
        );
        assert_eq!(text(&hit["_source"], "poetNameDisplay"), "P");
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
