use axum::Json;
use axum::extract::State;
use serde::Deserialize;

use crate::constants::FEELING_VERSES_PER_PAGE;
use crate::contract::feelings::{FeelingItem, FeelingVerseItem};
use crate::domain::feelings::{self, FeelingNode};
use crate::envelope::{ItemEnvelope, ListEnvelope, build_pagination};
use crate::error::{AppError, Resource};
use crate::extract::{SafePath, SafeQuery, invalid_path_slug};
use crate::openapi::{ListErrors, LookupErrors};
use crate::params::{AnyPage, NoParams, TransliteratedSlug};
use crate::state::AppState;

#[derive(Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(crate) struct VersesParams {
    /// Page number as a 1-based integer string, 30 verses a page. Minimum 1.
    #[param(inline, example = "1")]
    page: Option<AnyPage>,
}

async fn tree(state: &AppState) -> Result<Vec<FeelingNode>, AppError> {
    let all = state.feelings.all().await?;
    let counts = state.feelings.branch_counts().await?;
    Ok(feelings::nodes(&all, &counts))
}

fn find(nodes: Vec<FeelingNode>, raw: &str) -> Result<FeelingNode, AppError> {
    let slug = TransliteratedSlug::parse(raw).map_err(|reason| invalid_path_slug(&reason))?;
    tracing::Span::current().record("feeling", slug.as_str());
    nodes
        .into_iter()
        .find(|node| node.slug == slug.as_str())
        .ok_or(AppError::NotFound(Resource::Feeling))
}

#[utoipa::path(
    get,
    path = "/feelings",
    tag = "feelings",
    operation_id = "feelings.list",
    description = "The feelings of the Feelings Wheel in Arabic that have verses, as a flat list in tree order: each core feeling, then its children, each followed by its own children. Each item counts the verses of its whole branch.",
    responses(
        (status = 200, description = "Every feeling with verses.", body = ListEnvelope<FeelingItem>),
        ListErrors,
    ),
)]
pub(crate) async fn list(
    State(state): State<AppState>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ListEnvelope<FeelingItem>>, AppError> {
    let nodes = tree(&state).await?;
    let rows: Vec<FeelingItem> = nodes.into_iter().map(FeelingItem::from).collect();
    let count = u32::try_from(rows.len()).unwrap_or(u32::MAX);
    crate::log::record_results(u64::from(count));
    Ok(Json(ListEnvelope {
        data: rows,
        pagination: build_pagination(1, count.max(1), count),
    }))
}

#[utoipa::path(
    get,
    path = "/feelings/{slug}",
    tag = "feelings",
    operation_id = "feelings.get",
    description = "A feeling by slug, with the count of verses in its whole branch. A feeling with no verses is not found.",
    params(
        ("slug" = String, Path, description = "The feeling's `slug`, from its list.", pattern = "^[a-z][a-z-]*$", example = "alhuzn"),
    ),
    responses(
        (status = 200, description = "The requested feeling.", body = ItemEnvelope<FeelingItem>),
        LookupErrors,
    ),
)]
pub(crate) async fn get(
    State(state): State<AppState>,
    SafePath(raw): SafePath<String>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ItemEnvelope<FeelingItem>>, AppError> {
    let nodes = tree(&state).await?;
    Ok(Json(ItemEnvelope {
        data: FeelingItem::from(find(nodes, &raw)?),
    }))
}

#[utoipa::path(
    get,
    path = "/feelings/{slug}/verses",
    tag = "feelings",
    operation_id = "feelings.verses",
    description = "A page of 30 classical verses that express the feeling or a feeling below it, the strongest first. Each verse appears once and names its strongest feeling in the branch.",
    params(
        ("slug" = String, Path, description = "The feeling's `slug`, from its list.", pattern = "^[a-z][a-z-]*$", example = "alhuzn"),
        VersesParams,
    ),
    responses(
        (status = 200, description = "A page of verses with pagination metadata.", body = ListEnvelope<FeelingVerseItem>),
        LookupErrors,
    ),
)]
pub(crate) async fn verses(
    State(state): State<AppState>,
    SafePath(raw): SafePath<String>,
    SafeQuery(params): SafeQuery<VersesParams>,
) -> Result<Json<ListEnvelope<FeelingVerseItem>>, AppError> {
    let node = find(tree(&state).await?, &raw)?;
    let page = params.page.map_or(1, AnyPage::get);
    let found = state
        .feelings
        .verses(node.id, page, FEELING_VERSES_PER_PAGE)
        .await?;
    let total = node.verse_count.cast_unsigned();
    crate::log::record_results(u64::from(total));
    tracing::Span::current().record("page", page);
    Ok(Json(ListEnvelope {
        data: found.into_iter().map(FeelingVerseItem::from).collect(),
        pagination: build_pagination(page, FEELING_VERSES_PER_PAGE, total),
    }))
}
