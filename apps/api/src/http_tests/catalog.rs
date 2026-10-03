use std::sync::Arc;

use async_trait::async_trait;
use axum::http::StatusCode;

use crate::domain::poems::{
    FacetCounts, Facets, PoemListItem, PoemRecord, PoemRepository, RandomPoem,
};
use crate::domain::poets::{PoetRepository, PoetSlugEntry, PoetStats};
use crate::domain::{EraRef, MeterRef, PoemTypeRef, PoetRef, RhymeRef, ThemeRef};
use crate::error::StoreError;
use crate::http_tests::empty_hits;
use crate::state::AppState;
use crate::test_support::{FakeEs, request, send, state};

#[derive(Default)]
struct Poems {
    survivor: Option<&'static str>,
    lines: Option<Vec<String>>,
}

fn record(lines: Vec<String>) -> PoemRecord {
    let (name, slug) = (String::from("Name"), String::from("name"));
    PoemRecord {
        title: "Title".into(),
        verse_count: 1,
        recension_of_id: None,
        poet: PoetRef {
            name: name.clone(),
            slug: slug.clone(),
            has_avatar: false,
            is_anonymous: false,
        },
        era: EraRef {
            name: name.clone(),
            slug: slug.clone(),
        },
        meter: MeterRef {
            name: name.clone(),
            slug: slug.clone(),
        },
        theme: ThemeRef {
            name: name.clone(),
            slug: slug.clone(),
        },
        rhyme: RhymeRef {
            name: name.clone(),
            slug: slug.clone(),
        },
        poem_type: PoemTypeRef { name, slug },
        lines,
        prev: None,
        next: None,
        family: Vec::new(),
        related: Vec::new(),
    }
}

#[async_trait]
impl PoemRepository for Poems {
    async fn count(&self) -> Result<i32, StoreError> {
        Ok(0)
    }
    async fn list_slugs(&self, _: u32, _: u32) -> Result<Vec<String>, StoreError> {
        Ok(Vec::new())
    }
    async fn list(
        &self,
        _: &Facets,
        _: u32,
        _: u32,
    ) -> Result<(Vec<PoemListItem>, i32), StoreError> {
        Ok((Vec::new(), 0))
    }
    async fn facet_counts(&self, _: &Facets) -> Result<Option<FacetCounts>, StoreError> {
        Ok(None)
    }
    async fn find(&self, _: &str) -> Result<Option<PoemRecord>, StoreError> {
        Ok(self.lines.clone().map(record))
    }
    async fn alias_target(&self, _: &str) -> Result<Option<String>, StoreError> {
        Ok(self.survivor.map(String::from))
    }
    async fn random(&self) -> Result<Option<RandomPoem>, StoreError> {
        Ok(None)
    }
}

struct Poets {
    survivor: Option<&'static str>,
}

#[async_trait]
impl PoetRepository for Poets {
    async fn get(&self, _: &str) -> Result<Option<PoetStats>, StoreError> {
        Ok(None)
    }
    async fn alias_target(&self, _: &str) -> Result<Option<String>, StoreError> {
        Ok(self.survivor.map(String::from))
    }
    async fn count_with_poems(&self) -> Result<i32, StoreError> {
        Ok(0)
    }
    async fn list_slugs(&self, _: u32, _: u32) -> Result<Vec<PoetSlugEntry>, StoreError> {
        Ok(Vec::new())
    }
}

async fn with_poems(poems: Poems) -> (FakeEs, AppState) {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let mut state = state(&es);
    state.poems = Arc::new(poems);
    (es, state)
}

#[tokio::test]
async fn a_merged_poem_slug_redirects_to_the_poem_it_was_merged_into() {
    let (_es, state) = with_poems(Poems {
        survivor: Some("Abcd"),
        ..Poems::default()
    })
    .await;
    let sent = send(crate::app(state), request("GET", "/v1/poems/TnKK")).await;
    assert_eq!(sent.status, StatusCode::MOVED_PERMANENTLY);
    assert_eq!(sent.header("location"), Some("/v1/poems/Abcd"));
}

#[tokio::test]
async fn an_unknown_poem_slug_without_an_alias_is_a_404() {
    let (_es, state) = with_poems(Poems::default()).await;
    let sent = send(crate::app(state), request("GET", "/v1/poems/TnKK")).await;
    assert_eq!(sent.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_poem_whose_verses_are_missing_is_a_parse_error_not_an_empty_poem() {
    let (_es, state) = with_poems(Poems {
        lines: Some(Vec::new()),
        ..Poems::default()
    })
    .await;
    let sent = send(crate::app(state), request("GET", "/v1/poems/TnKK")).await;
    assert_eq!(sent.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(sent.json()["code"], "POEM_PARSE_ERROR");
}

#[tokio::test]
async fn a_found_poem_pairs_its_hemistichs_into_verses() {
    let (_es, state) = with_poems(Poems {
        lines: Some(vec!["a".into(), "b".into(), "c".into()]),
        ..Poems::default()
    })
    .await;
    let sent = send(crate::app(state), request("GET", "/v1/poems/TnKK")).await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(
        sent.json()["data"]["verses"],
        serde_json::json!([["a", "b"], ["c", ""]])
    );
}

#[tokio::test]
async fn facets_for_a_poet_that_is_not_shown_are_a_404() {
    let (_es, state) = with_poems(Poems::default()).await;
    let sent = send(
        crate::app(state),
        request("GET", "/v1/poems/facets?poet=TnKK"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_merged_poet_slug_redirects_to_the_poet_it_was_merged_into() {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let mut state = state(&es);
    state.poets = Arc::new(Poets {
        survivor: Some("Abcd"),
    });
    let sent = send(crate::app(state), request("GET", "/v1/poets/yoFB")).await;
    assert_eq!(sent.status, StatusCode::MOVED_PERMANENTLY);
    assert_eq!(sent.header("location"), Some("/v1/poets/Abcd"));
}
