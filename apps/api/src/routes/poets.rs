use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::constants::{
    API_V1_PREFIX, POEMS_PER_PAGE, READ_CACHE_CONTROL, SITEMAP_POETS_PER_SHARD,
};
use crate::contract::poets::{PoetSlugEntry, PoetStats};
use crate::contract::search::PoetListItem;
use crate::domain::search::{self, PoetListParams};
use crate::envelope::{ItemEnvelope, PageEnvelope};
use crate::error::{AppError, Resource};
use crate::extract::{SafePath, SafeQuery, invalid_path_slug, invalid_query_parameter};
use crate::openapi::{ListErrors, LookupErrors};
use crate::params::{
    FourLetterSlug, NOT_A_CURSOR, NoParams, PageCursor, PoetCursor, SearchText, SlugsParams,
    TransliteratedSlug,
};
use crate::routes::permanent_redirect;
use crate::state::AppState;

#[derive(Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(crate) struct PoetsParams {
    /// The `next` or `prev` value of a page of this list. Leave it out for the first page.
    #[param(
        inline,
        example = "eyJhZnRlciI6eyJzY29yZSI6MS41LCJwb2Vtc0NvdW50IjoxMDAsIm5hbWVTb3J0Ijoi2LLZh9mK2LEiLCJpZCI6MX19"
    )]
    cursor: Option<PoetCursor>,
    /// Narrow to the poets of one era. A single `slug` from GET /eras: unlike `GET /poems`, this filter takes one value.
    #[param(inline, example = "jahili")]
    era: Option<TransliteratedSlug>,
    /// Search poet names and nicknames, up to 100 characters. A poet is listed when every word matches the name or the nickname as a stem, or every word is the start of a word of them (two letters or more), or every word is within a typo of a word of the name. A typo is not allowed in a word of up to three letters; one is allowed from four letters and two from seven, and the first letter must be right.
    #[param(inline, example = "زهير بن أبي سلمى")]
    q: Option<SearchText>,
}

#[utoipa::path(
    get,
    path = "/poets",
    tag = "poets",
    operation_id = "poets.list",
    description = "A page of 30 poets with their poem counts, most poems first, then by name. Narrow to one era, or search names with `q`, which orders by relevance and keeps the same order for ties. Pass the page's `next` or `prev` as `cursor` for the page after or before it, with the same `era` and `q`.",
    params(PoetsParams),
    responses(
        (status = 200, description = "A page of poets, with the cursors of the pages around it.", body = PageEnvelope<PoetListItem>),
        ListErrors,
    ),
)]
pub(crate) async fn list(
    State(state): State<AppState>,
    SafeQuery(params): SafeQuery<PoetsParams>,
) -> Result<Json<PageEnvelope<PoetListItem>>, AppError> {
    let q = params
        .q
        .map(|text| search::normalize_query(text.as_str()))
        .unwrap_or_default();
    let cursor = params.cursor.map(PageCursor::into_inner);
    if cursor
        .as_ref()
        .is_some_and(|cursor| cursor.key().score.is_some() == q.is_empty())
    {
        return Err(invalid_query_parameter("cursor", NOT_A_CURSOR));
    }
    let (poets, total) = state
        .search
        .list_poets(&PoetListParams {
            q,
            era_slugs: params
                .era
                .map(TransliteratedSlug::into_inner)
                .into_iter()
                .collect(),
            cursor,
            page_size: POEMS_PER_PAGE,
        })
        .await?;
    crate::log::record_results(u64::from(total));
    Ok(Json(PageEnvelope::new(poets, total, PoetListItem::from)))
}

#[utoipa::path(
    get,
    path = "/poets/slugs",
    tag = "poets",
    operation_id = "poets.listSlugs",
    description = "The slug of every poet with at least one poem, with an avatar flag, 45,000 a page in catalog order, oldest entries first, for sitemaps and incremental crawling.",
    params(SlugsParams),
    responses(
        (status = 200, description = "A page of poet slugs, each flagged with whether the poet has an avatar.", body = PageEnvelope<PoetSlugEntry>),
        ListErrors,
    ),
)]
pub(crate) async fn list_slugs(
    State(state): State<AppState>,
    SafeQuery(params): SafeQuery<SlugsParams>,
) -> Result<Json<PageEnvelope<PoetSlugEntry>>, AppError> {
    let cursor = params.cursor.map(PageCursor::into_inner);
    let (slugs, total) = tokio::try_join!(
        state
            .poets
            .list_slugs(cursor.as_ref(), SITEMAP_POETS_PER_SHARD),
        state.poets.count_with_poems()
    )?;
    crate::log::record_results(u64::try_from(slugs.items.len()).unwrap_or(u64::MAX));
    Ok(Json(PageEnvelope::new(
        slugs,
        total.cast_unsigned(),
        PoetSlugEntry::from,
    )))
}

#[utoipa::path(
    get,
    path = "/poets/{slug}",
    tag = "poets",
    operation_id = "poets.get",
    description = "A poet by slug, with nickname, biography, era, poem count, and whether an avatar image exists. A slug that was merged into another poet answers 301 to the surviving poet.",
    params(
        ("slug" = String, Path, description = "The poet's `slug`, from a list or search response.", pattern = "^[a-zA-Z]{4}$", example = "PAKT"),
    ),
    responses(
        (status = 200, description = "The requested poet.", body = ItemEnvelope<PoetStats>),
        (status = 301, description = "The slug belongs to a poet merged into another; `Location` names the surviving poet.", headers(("Location" = String, description = "Path of the surviving poet"))),
        LookupErrors,
    ),
)]
pub(crate) async fn detail(
    State(state): State<AppState>,
    SafePath(raw): SafePath<String>,
    _: SafeQuery<NoParams>,
) -> Result<Response, AppError> {
    let slug = FourLetterSlug::parse(&raw).map_err(|reason| invalid_path_slug(&reason))?;
    let slug = slug.as_str();
    tracing::Span::current().record("poet_id", slug);
    let Some(poet) = state.poets.get(slug).await? else {
        let Some(survivor) = state.poets.alias_target(slug).await? else {
            return Err(AppError::NotFound(Resource::Poet));
        };
        tracing::Span::current().record("alias_of", survivor.as_str());
        return Ok(permanent_redirect(
            &format!("{API_V1_PREFIX}/poets/{survivor}"),
            READ_CACHE_CONTROL,
        ));
    };
    Ok(Json(ItemEnvelope {
        data: PoetStats::from(poet),
    })
    .into_response())
}
