use std::sync::Arc;

use async_trait::async_trait;
use axum::http::StatusCode;

use crate::domain::StoreError;
use crate::domain::feelings::{Feeling, FeelingRepository, FeelingVerse};
use crate::domain::poems::PoemLink;
use crate::domain::{PoetBrief, Term};
use crate::http_tests::empty_hits;
use crate::state::AppState;
use crate::test_support::{FakeEs, request, send, state};

struct Feelings {
    with_verses: bool,
}

fn feeling(id: i32, parent_id: Option<i32>, slug: &str) -> Feeling {
    Feeling {
        id,
        parent_id,
        slug: slug.into(),
        name: format!("ال{slug}"),
        wheel_term: Some(slug.to_uppercase()),
        sort_order: id,
    }
}

fn verse(feeling: &str) -> FeelingVerse {
    FeelingVerse {
        parts: vec!["a".into(), "b".into()],
        position: 1,
        feeling: Term {
            name: feeling.into(),
            slug: feeling.into(),
        },
        poem: PoemLink {
            title: "Title".into(),
            slug: "TnKK".into(),
        },
        poet: PoetBrief {
            name: "Poet".into(),
            slug: "PAKT".into(),
            has_avatar: false,
            is_anonymous: false,
        },
        era: Term {
            name: "Era".into(),
            slug: "jahili".into(),
        },
    }
}

#[async_trait]
impl FeelingRepository for Feelings {
    async fn all(&self) -> Result<Vec<Feeling>, StoreError> {
        Ok(vec![
            feeling(1, None, "sad"),
            feeling(2, Some(1), "lonely"),
            feeling(3, None, "happy"),
            feeling(4, Some(3), "free"),
        ])
    }

    async fn branch_counts(&self) -> Result<Vec<(i32, i32)>, StoreError> {
        Ok(if self.with_verses {
            vec![(1, 3), (2, 2), (3, 1), (4, 0)]
        } else {
            Vec::new()
        })
    }

    async fn verses(
        &self,
        feeling_id: i32,
        _: u32,
        _: u32,
    ) -> Result<Vec<FeelingVerse>, StoreError> {
        Ok(match (self.with_verses, feeling_id) {
            (true, 1) => vec![verse("sad"), verse("lonely"), verse("lonely")],
            (true, 2) => vec![verse("lonely"), verse("lonely")],
            _ => Vec::new(),
        })
    }
}

async fn with_feelings(with_verses: bool) -> (FakeEs, AppState) {
    let es = FakeEs::serving(StatusCode::OK, empty_hits()).await;
    let mut state = state(&es);
    state.feelings = Arc::new(Feelings { with_verses });
    (es, state)
}

#[tokio::test]
async fn the_list_is_in_tree_order_with_branch_counts() {
    let (_es, state) = with_feelings(true).await;
    let body = send(crate::app(state), request("GET", "/v1/feelings"))
        .await
        .json();
    let rows = body["data"].as_array().expect("data");
    let slugs: Vec<&str> = rows.iter().filter_map(|r| r["slug"].as_str()).collect();
    assert_eq!(slugs, vec!["sad", "lonely", "happy"]);
    assert_eq!(rows[0]["verseCount"], 3);
    assert_eq!(rows[1]["parentSlug"], "sad");
    assert_eq!(rows[1]["wheelTerm"], "LONELY");
    assert!(rows[0].get("parentSlug").is_none());
}

#[tokio::test]
async fn a_core_lists_the_verses_of_its_whole_branch() {
    let (_es, state) = with_feelings(true).await;
    let body = send(crate::app(state), request("GET", "/v1/feelings/sad/verses"))
        .await
        .json();
    assert_eq!(body["pagination"]["totalItems"], 3);
    assert_eq!(body["data"][1]["feeling"]["slug"], "lonely");
    assert_eq!(body["data"][0]["parts"], serde_json::json!(["a", "b"]));
    assert_eq!(body["data"][0]["poem"]["slug"], "TnKK");
}

#[tokio::test]
async fn a_feeling_without_verses_is_left_out_and_not_found() {
    let (_es, state) = with_feelings(true).await;
    let one = send(
        crate::app(state.clone()),
        request("GET", "/v1/feelings/free"),
    )
    .await;
    assert_eq!(one.status, StatusCode::NOT_FOUND);
    let page = send(
        crate::app(state),
        request("GET", "/v1/feelings/free/verses"),
    )
    .await;
    assert_eq!(page.status, StatusCode::NOT_FOUND);
    let (_es, empty) = with_feelings(false).await;
    let list = send(crate::app(empty), request("GET", "/v1/feelings")).await;
    assert_eq!(list.status, StatusCode::OK);
    assert_eq!(list.json()["data"], serde_json::json!([]));
}

#[tokio::test]
async fn an_unknown_feeling_is_a_404_and_a_malformed_slug_is_a_400() {
    let (_es, state) = with_feelings(true).await;
    let unknown = send(
        crate::app(state.clone()),
        request("GET", "/v1/feelings/nothing"),
    )
    .await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(unknown.json()["detail"], "Feeling not found");
    let verses = send(
        crate::app(state.clone()),
        request("GET", "/v1/feelings/nothing/verses"),
    )
    .await;
    assert_eq!(verses.status, StatusCode::NOT_FOUND);
    let malformed = send(crate::app(state), request("GET", "/v1/feelings/ZZ")).await;
    assert_eq!(malformed.status, StatusCode::BAD_REQUEST);
}
