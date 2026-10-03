use async_trait::async_trait;
use serde_json::Value;

use crate::constants::ES_MAX_RESULT_WINDOW;
use crate::domain::search::{
    Page, PoemHit, PoemSearchParams, PoetHit, PoetListing, PoetSearchParams, SearchIndex,
    poem_snippet,
};
use crate::domain::{PoetBrief, Term};
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
    async fn search_poems(&self, params: &PoemSearchParams) -> Result<Page<PoemHit>, StoreError> {
        let response = self
            .search(&self.poems_alias, &poem_search_body(params))
            .await?;
        let hits = hits(&response)
            .into_iter()
            .map(|hit| {
                let source = &hit["_source"];
                PoemHit {
                    title: display(source, "titleDisplay", "title"),
                    slug: text(source, "slug"),
                    snippet: poem_snippet(highlighted_content(hit), &text(source, "content")),
                    poet: PoetBrief {
                        name: text(source, "poetNameDisplay"),
                        slug: text(source, "poetSlug"),
                        has_avatar: source["poetHasAvatar"].as_bool().unwrap_or(false),
                        is_anonymous: source["poetIsAnonymous"].as_bool().unwrap_or(false),
                    },
                    meter: Term {
                        name: text(source, "meterName"),
                        slug: text(source, "meterSlug"),
                    },
                    era: Term {
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

    async fn search_poets(&self, params: &PoetSearchParams) -> Result<Page<PoetHit>, StoreError> {
        let response = self
            .search(&self.poets_alias, &poet_search_body(params))
            .await?;
        let hits = hits(&response)
            .into_iter()
            .map(|hit| {
                let source = &hit["_source"];
                PoetHit {
                    name: display(source, "nameDisplay", "name"),
                    slug: text(source, "slug"),
                    era: Term {
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

    async fn list_poets(&self, params: &PoetSearchParams) -> Result<Page<PoetListing>, StoreError> {
        let response = self
            .search(&self.poets_alias, &poet_search_body(params))
            .await?;
        let hits = hits(&response)
            .into_iter()
            .map(|hit| {
                let source = &hit["_source"];
                PoetListing {
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
    use std::time::Duration;

    use axum::http::StatusCode;
    use serde_json::json;

    use super::*;
    use crate::test_support::FakeEs;

    async fn answering(response: Value) -> (FakeEs, Es) {
        let fake = FakeEs::serving(StatusCode::OK, response).await;
        let es = Es::with_timeout(&fake.url, Duration::from_secs(2)).expect("a fake endpoint");
        (fake, es)
    }

    async fn poems_found(response: Value) -> Page<PoemHit> {
        let (_fake, es) = answering(response).await;
        es.search_poems(&PoemSearchParams::default())
            .await
            .expect("a page of poems")
    }

    async fn poets_found(response: Value) -> Page<PoetHit> {
        let (_fake, es) = answering(response).await;
        es.search_poets(&PoetSearchParams::default())
            .await
            .expect("a page of poets")
    }

    fn poem_hit(source: Value, highlight: Option<&str>, score: f64) -> Value {
        let mut hit = json!({ "_source": source, "_score": score });
        if let Some(highlight) = highlight {
            hit["highlight"] = json!({ "content": [highlight] });
        }
        hit
    }

    #[tokio::test]
    async fn a_poem_hit_shows_its_vocalized_title_its_highlighted_verse_and_its_score() {
        let found = poems_found(json!({ "hits": { "total": { "value": 1 }, "hits": [poem_hit(
            json!({ "slug": "TnKK", "title": "plain", "titleDisplay": "vocalized", "content": "a*b*c*d",
                    "poetNameDisplay": "P", "poetSlug": "yoFB", "meterName": "m",
                    "meterSlug": "altawil", "eraName": "e", "eraSlug": "abbasi" }),
            Some("a*b <mark>x</mark>*c*d"), 3.5) ] } }))
        .await;
        let hit = &found.hits[0];
        assert_eq!(
            (hit.title.as_str(), hit.slug.as_str()),
            ("vocalized", "TnKK")
        );
        assert_eq!(
            (hit.poet.name.as_str(), hit.poet.slug.as_str()),
            ("P", "yoFB")
        );
        assert_eq!(
            (hit.meter.slug.as_str(), hit.era.slug.as_str()),
            ("altawil", "abbasi")
        );
        assert_eq!(hit.snippet, "a*b <mark>x</mark>");
        assert_eq!(hit.relevance.to_bits(), 3.5_f64.to_bits());
        assert_eq!(found.total, 1);
    }

    #[tokio::test]
    async fn an_empty_vocalized_title_stays_empty_and_a_missing_one_falls_back_to_the_plain_title()
    {
        let found = poems_found(json!({ "hits": { "total": { "value": 2 }, "hits": [
            poem_hit(json!({ "titleDisplay": "", "title": "plain" }), None, 1.0),
            poem_hit(json!({ "title": "plain" }), None, 1.0),
        ] } }))
        .await;
        assert_eq!(found.hits[0].title, "");
        assert_eq!(found.hits[1].title, "plain");
    }

    #[tokio::test]
    async fn the_poem_total_counts_poems_not_readings_and_stops_at_the_result_window() {
        let grouped = poems_found(json!({
            "hits": { "total": { "value": 12 }, "hits": [] },
            "aggregations": { "poems": { "value": 9 } },
        }))
        .await;
        assert_eq!(grouped.total, 9);
        let many = poems_found(json!({
            "hits": { "total": { "value": 10000 }, "hits": [] },
            "aggregations": { "poems": { "value": 51234 } },
        }))
        .await;
        assert_eq!(many.total, ES_MAX_RESULT_WINDOW);
        let browse = poems_found(json!({ "hits": { "total": { "value": 7 }, "hits": [] } })).await;
        assert_eq!(browse.total, 7);
    }

    #[tokio::test]
    async fn the_poet_total_is_read_whether_elasticsearch_sends_a_number_or_an_object() {
        assert_eq!(
            poets_found(json!({ "hits": { "total": 7, "hits": [] } }))
                .await
                .total,
            7
        );
        assert_eq!(
            poets_found(json!({ "hits": { "total": { "value": 9 }, "hits": [] } }))
                .await
                .total,
            9
        );
        assert_eq!(poets_found(json!({ "hits": {} })).await.total, 0);
    }

    #[tokio::test]
    async fn a_hit_missing_its_fields_is_shown_with_empty_text_and_zero_counts() {
        let found = poems_found(json!({ "hits": { "hits": [ { "_source": {} } ] } })).await;
        let hit = &found.hits[0];
        assert_eq!((hit.slug.as_str(), hit.snippet.as_str()), ("", ""));
        assert_eq!(hit.relevance.to_bits(), 0.0_f64.to_bits());
        assert_eq!(found.total, 0);
        let (_fake, es) = answering(json!({ "hits": { "hits": [ { "_source": {} } ] } })).await;
        let listed = es
            .list_poets(&PoetSearchParams::default())
            .await
            .expect("a page of poets");
        assert_eq!(listed.hits[0].poems_count, 0);
        assert!(poems_found(json!({})).await.hits.is_empty());
    }

    #[tokio::test]
    async fn an_empty_highlight_shows_the_opening_verse() {
        let found = poems_found(json!({ "hits": { "hits": [
            poem_hit(json!({ "content": "x*y*z" }), Some(""), 1.0)
        ] } }))
        .await;
        assert_eq!(found.hits[0].snippet, "x*y");
    }
}
