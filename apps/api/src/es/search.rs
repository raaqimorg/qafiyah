use async_trait::async_trait;
use serde::Deserialize;

use crate::constants::ES_MAX_RESULT_WINDOW;
use crate::domain::StoreError;
use crate::domain::search::{
    Page, PoemHit, PoemSearchParams, PoetHit, PoetListing, PoetSearchParams, SearchIndex,
    poem_snippet,
};
use crate::domain::{PoetBrief, Term};
use crate::es::client::Es;
use crate::es::query::{poem_search_body, poet_search_body};

#[derive(Deserialize)]
struct Response<S> {
    hits: Hits<S>,
    aggregations: Option<Aggregations>,
}

#[derive(Deserialize)]
struct Hits<S> {
    total: Count,
    hits: Vec<Hit<S>>,
}

#[derive(Deserialize)]
struct Aggregations {
    poems: Count,
}

#[derive(Deserialize)]
struct Count {
    value: u32,
}

#[derive(Deserialize)]
struct Hit<S> {
    #[serde(rename = "_score")]
    score: Option<f64>,
    #[serde(rename = "_source")]
    source: S,
    highlight: Option<Highlight>,
}

#[derive(Deserialize)]
struct Highlight {
    content: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PoemSource {
    title_display: String,
    slug: String,
    content: String,
    poet_name_display: String,
    poet_slug: String,
    poet_has_avatar: bool,
    poet_is_anonymous: bool,
    meter_name: String,
    meter_slug: String,
    era_name: String,
    era_slug: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PoetSource {
    name_display: String,
    slug: String,
    era_name: String,
    era_slug: String,
    poems_count: i64,
}

impl<S> Hit<S> {
    fn relevance(&self) -> f64 {
        self.score.unwrap_or_default()
    }

    fn highlighted_content(&self) -> Option<&str> {
        self.highlight
            .as_ref()?
            .content
            .first()
            .map(String::as_str)
            .filter(|value| !value.is_empty())
    }
}

impl Response<PoemSource> {
    fn poem_total(&self) -> u32 {
        self.aggregations
            .as_ref()
            .map_or(self.hits.total.value, |aggregations| {
                aggregations.poems.value.min(ES_MAX_RESULT_WINDOW)
            })
    }
}

#[async_trait]
impl SearchIndex for Es {
    async fn search_poems(&self, params: &PoemSearchParams) -> Result<Page<PoemHit>, StoreError> {
        let response: Response<PoemSource> = self
            .search(&self.poems_alias, &poem_search_body(params))
            .await?;
        let total = response.poem_total();
        let hits = response
            .hits
            .hits
            .into_iter()
            .map(|hit| PoemHit {
                snippet: poem_snippet(hit.highlighted_content(), &hit.source.content),
                relevance: hit.relevance(),
                title: hit.source.title_display,
                slug: hit.source.slug,
                poet: PoetBrief {
                    name: hit.source.poet_name_display,
                    slug: hit.source.poet_slug,
                    has_avatar: hit.source.poet_has_avatar,
                    is_anonymous: hit.source.poet_is_anonymous,
                },
                meter: Term {
                    name: hit.source.meter_name,
                    slug: hit.source.meter_slug,
                },
                era: Term {
                    name: hit.source.era_name,
                    slug: hit.source.era_slug,
                },
            })
            .collect();
        Ok(Page { hits, total })
    }

    async fn search_poets(&self, params: &PoetSearchParams) -> Result<Page<PoetHit>, StoreError> {
        let response: Response<PoetSource> = self
            .search(&self.poets_alias, &poet_search_body(params))
            .await?;
        let hits = response
            .hits
            .hits
            .into_iter()
            .map(|hit| PoetHit {
                relevance: hit.relevance(),
                name: hit.source.name_display,
                slug: hit.source.slug,
                era: Term {
                    name: hit.source.era_name,
                    slug: hit.source.era_slug,
                },
            })
            .collect();
        Ok(Page {
            hits,
            total: response.hits.total.value,
        })
    }

    async fn list_poets(&self, params: &PoetSearchParams) -> Result<Page<PoetListing>, StoreError> {
        let response: Response<PoetSource> = self
            .search(&self.poets_alias, &poet_search_body(params))
            .await?;
        let hits = response
            .hits
            .hits
            .into_iter()
            .map(|hit| PoetListing {
                name: hit.source.name_display,
                slug: hit.source.slug,
                poems_count: hit.source.poems_count,
            })
            .collect();
        Ok(Page {
            hits,
            total: response.hits.total.value,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::http::StatusCode;
    use serde_json::{Value, json};

    use super::*;
    use crate::metrics::Metrics;
    use crate::test_support::FakeEs;

    async fn answering(response: Value) -> (FakeEs, Es) {
        let fake = FakeEs::serving(StatusCode::OK, response).await;
        let es = Es::with_timeout(&fake.url, Duration::from_secs(2), Metrics::default())
            .expect("a fake endpoint");
        (fake, es)
    }

    async fn poems_searched(response: Value) -> Result<Page<PoemHit>, StoreError> {
        let (_fake, es) = answering(response).await;
        es.search_poems(&PoemSearchParams::default()).await
    }

    async fn poems_found(response: Value) -> Page<PoemHit> {
        poems_searched(response).await.expect("a page of poems")
    }

    async fn poets_found(response: Value) -> Page<PoetHit> {
        let (_fake, es) = answering(response).await;
        es.search_poets(&PoetSearchParams::default())
            .await
            .expect("a page of poets")
    }

    fn poem_source(slug: &str, content: &str) -> Value {
        json!({ "slug": slug, "title": "plain", "titleDisplay": "vocalized", "content": content,
                "poetNameDisplay": "P", "poetSlug": "yoFB", "poetHasAvatar": true,
                "poetIsAnonymous": false, "meterName": "m", "meterSlug": "altawil",
                "eraName": "e", "eraSlug": "abbasi" })
    }

    fn poet_source(slug: &str) -> Value {
        json!({ "slug": slug, "name": "plain", "nameDisplay": "vocalized", "eraName": "e",
                "eraSlug": "abbasi", "poemsCount": 12 })
    }

    fn found(total: u32, hits: Vec<Value>) -> Value {
        json!({ "hits": { "total": { "value": total, "relation": "eq" }, "hits": hits } })
    }

    fn poem_hit(source: Value, highlight: Option<&str>, score: Value) -> Value {
        let mut hit = json!({ "_source": source, "_score": score });
        if let Some(highlight) = highlight {
            hit["highlight"] = json!({ "content": [highlight] });
        }
        hit
    }

    #[tokio::test]
    async fn a_poem_hit_shows_its_vocalized_title_its_highlighted_verse_and_its_score() {
        let found = poems_found(found(
            1,
            vec![poem_hit(
                poem_source("TnKK", "a*b\nc*d"),
                Some("a*b <mark>x</mark>\nc*d"),
                json!(3.5),
            )],
        ))
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
        assert_eq!((hit.poet.has_avatar, hit.poet.is_anonymous), (true, false));
        assert_eq!(
            (hit.meter.slug.as_str(), hit.era.slug.as_str()),
            ("altawil", "abbasi")
        );
        assert_eq!(hit.snippet, "a*b <mark>x</mark>");
        assert_eq!(hit.relevance.to_bits(), 3.5_f64.to_bits());
        assert_eq!(found.total, 1);
    }

    #[tokio::test]
    async fn a_poet_hit_and_a_listed_poet_show_the_vocalized_name() {
        let hits = vec![json!({ "_source": poet_source("yoFB"), "_score": 2.0 })];
        let searched = poets_found(found(1, hits.clone())).await;
        assert_eq!(
            (
                searched.hits[0].name.as_str(),
                searched.hits[0].slug.as_str()
            ),
            ("vocalized", "yoFB")
        );
        assert_eq!(searched.hits[0].era.slug, "abbasi");
        let (_fake, es) = answering(found(1, hits)).await;
        let listed = es
            .list_poets(&PoetSearchParams::default())
            .await
            .expect("a page of poets");
        assert_eq!(
            (listed.hits[0].name.as_str(), listed.hits[0].poems_count),
            ("vocalized", 12)
        );
        assert_eq!(listed.total, 1);
    }

    #[tokio::test]
    async fn a_hit_sorted_without_a_score_has_no_relevance() {
        let found = poems_found(found(
            1,
            vec![poem_hit(poem_source("TnKK", "a*b"), None, Value::Null)],
        ))
        .await;
        assert_eq!(found.hits[0].relevance.to_bits(), 0.0_f64.to_bits());
    }

    #[tokio::test]
    async fn the_poem_total_counts_poems_not_readings_and_stops_at_the_result_window() {
        let mut grouped = found(12, vec![]);
        grouped["aggregations"] = json!({ "poems": { "value": 9 } });
        assert_eq!(poems_found(grouped).await.total, 9);
        let mut many = found(10_000, vec![]);
        many["aggregations"] = json!({ "poems": { "value": 51_234 } });
        assert_eq!(poems_found(many).await.total, ES_MAX_RESULT_WINDOW);
        assert_eq!(poems_found(found(7, vec![])).await.total, 7);
    }

    #[tokio::test]
    async fn a_response_missing_a_field_fails_the_search_instead_of_showing_empty_text() {
        let mut source = poem_source("TnKK", "a*b");
        source
            .as_object_mut()
            .expect("an object")
            .remove("titleDisplay");
        assert!(matches!(
            poems_searched(found(1, vec![poem_hit(source, None, json!(1.0))])).await,
            Err(StoreError::Search(_))
        ));
        assert!(matches!(
            poems_searched(json!({ "hits": { "hits": [] } })).await,
            Err(StoreError::Search(_))
        ));
        let (_fake, es) = answering(found(1, vec![json!({ "_source": { "slug": "yoFB" } })])).await;
        assert!(matches!(
            es.list_poets(&PoetSearchParams::default()).await,
            Err(StoreError::Search(_))
        ));
    }

    #[tokio::test]
    async fn an_empty_highlight_shows_the_opening_verse() {
        let found = poems_found(found(
            1,
            vec![poem_hit(
                poem_source("TnKK", "x*y\nz"),
                Some(""),
                json!(1.0),
            )],
        ))
        .await;
        assert_eq!(found.hits[0].snippet, "x*y");
    }
}
