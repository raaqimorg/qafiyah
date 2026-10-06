use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::http::StatusCode;

use crate::constants::RANDOM_POEM_MAX_ATTEMPTS;
use crate::domain::StoreError;
use crate::domain::poems::{
    FacetCounts, Facets, PoemRecord, PoemRepository, PoemSummary, RandomPoem,
};
use crate::domain::poets::{PoetProfile, PoetRepository, PoetSlug};
use crate::domain::{PoetBrief, Term};
use crate::http_tests::empty_hits;
use crate::state::AppState;
use crate::test_support::{FakeEs, request, send, state};

#[derive(Default)]
struct Poems {
    stall: bool,
    survivor: Option<&'static str>,
    lines: Option<Vec<String>>,
    randoms: Mutex<VecDeque<(&'static str, &'static str)>>,
}

fn drawing(randoms: &[(&'static str, &'static str)]) -> Poems {
    Poems {
        randoms: Mutex::new(randoms.iter().copied().collect()),
        ..Poems::default()
    }
}

fn record(lines: Vec<String>) -> PoemRecord {
    let (name, slug) = (String::from("Name"), String::from("name"));
    PoemRecord {
        title: "Title".into(),
        verse_count: 1,
        recension_of_id: None,
        poet: PoetBrief {
            name: name.clone(),
            slug: slug.clone(),
            has_avatar: false,
            is_anonymous: false,
        },
        era: Term {
            name: name.clone(),
            slug: slug.clone(),
        },
        meter: Term {
            name: name.clone(),
            slug: slug.clone(),
        },
        theme: Term {
            name: name.clone(),
            slug: slug.clone(),
        },
        rhyme: Term {
            name: name.clone(),
            slug: slug.clone(),
        },
        poem_type: Term { name, slug },
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
        if self.stall {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
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
    ) -> Result<(Vec<PoemSummary>, i32), StoreError> {
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
        Ok(self
            .randoms
            .lock()
            .expect("the random queue")
            .pop_front()
            .map(|(slug, content)| RandomPoem {
                poet_name: "Poet".into(),
                lines: vec![content.into()],
                slug: slug.into(),
            }))
    }
}

struct Poets {
    survivor: Option<&'static str>,
}

#[async_trait]
impl PoetRepository for Poets {
    async fn get(&self, _: &str) -> Result<Option<PoetProfile>, StoreError> {
        Ok(None)
    }
    async fn alias_target(&self, _: &str) -> Result<Option<String>, StoreError> {
        Ok(self.survivor.map(String::from))
    }
    async fn count_with_poems(&self) -> Result<i32, StoreError> {
        Ok(0)
    }
    async fn list_slugs(&self, _: u32, _: u32) -> Result<Vec<PoetSlug>, StoreError> {
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
async fn a_found_poem_returns_one_entry_per_stored_row() {
    let (_es, state) = with_poems(Poems {
        lines: Some(vec!["a*b".into(), "c".into(), "d*e".into()]),
        ..Poems::default()
    })
    .await;
    let sent = send(crate::app(state), request("GET", "/v1/poems/TnKK")).await;
    assert_eq!(sent.status, StatusCode::OK);
    assert_eq!(
        sent.json()["data"]["verses"],
        serde_json::json!([["a", "b"], ["c"], ["d", "e"]])
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

#[tokio::test]
async fn a_random_poem_is_its_slug_or_its_lines_and_any_other_option_is_refused() {
    let (_es, state) = with_poems(drawing(&[
        ("Abcd", "first*second"),
        ("Efgh", "first*second"),
    ]))
    .await;
    let app = crate::app(state);
    let slug = send(app.clone(), request("GET", "/v1/poems/random")).await;
    assert_eq!((slug.status, slug.body.as_str()), (StatusCode::OK, "Abcd"));
    let lines = send(app.clone(), request("GET", "/v1/poems/random?option=lines")).await;
    assert_eq!(
        (lines.status, lines.body.as_str()),
        (StatusCode::OK, "first\nsecond\n\nPoet")
    );
    let refused = send(app, request("GET", "/v1/poems/random?option=verse")).await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        refused.json()["detail"],
        "Invalid query parameter `option`: unknown variant `verse`, expected `slug` or `lines`"
    );
}

#[tokio::test]
async fn random_lines_skip_a_poem_too_short_for_an_excerpt() {
    let (_es, state) = with_poems(drawing(&[("Abcd", "alone"), ("Efgh", "first*second")])).await;
    let sent = send(
        crate::app(state),
        request("GET", "/v1/poems/random?option=lines"),
    )
    .await;
    assert_eq!(
        (sent.status, sent.body.as_str()),
        (StatusCode::OK, "first\nsecond\n\nPoet")
    );
}

#[tokio::test]
async fn random_lines_give_up_after_the_last_attempt() {
    let unusable: Vec<(&str, &str)> = (0..RANDOM_POEM_MAX_ATTEMPTS)
        .map(|_| ("Abcd", "alone"))
        .collect();
    let (_es, state) = with_poems(drawing(&unusable)).await;
    let sent = send(
        crate::app(state),
        request("GET", "/v1/poems/random?option=lines"),
    )
    .await;
    assert_eq!(sent.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(sent.json()["detail"], "Failed to fetch random poem");
}

#[tokio::test]
async fn no_random_poem_at_all_is_a_500_not_an_empty_answer() {
    let (_es, state) = with_poems(Poems::default()).await;
    let sent = send(crate::app(state), request("GET", "/v1/poems/random")).await;
    assert_eq!(sent.status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test(start_paused = true)]
async fn a_request_past_its_deadline_is_a_temporary_problem_not_a_hang() {
    let (_es, state) = with_poems(Poems {
        stall: true,
        ..Poems::default()
    })
    .await;
    let sent = send(crate::app(state), request("GET", "/v1/poems/count")).await;
    assert_eq!(sent.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(sent.header("retry-after"), Some("2"));
    assert_eq!(sent.json()["code"], "SERVICE_UNAVAILABLE");
}
