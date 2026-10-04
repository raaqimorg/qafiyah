use axum::Json;
use axum::extract::{Extension, RawQuery, State};
use axum::response::{IntoResponse, Response};

use crate::constants::{
    API_V1_PREFIX, LIST_POETS_MAX_PAGE, MAX_QUERY_LENGTH, POEMS_PER_PAGE,
    POETS_LIST_MAX_RESULT_WINDOW, READ_CACHE_CONTROL, SITEMAP_POETS_PER_SHARD,
};
use crate::contract::poets::{PoetSlugEntry, PoetStats};
use crate::contract::search::PoetListItem;
use crate::domain::search::{self, PoetSearchParams, PoetSort};
use crate::envelope::{ItemEnvelope, ListEnvelope, build_pagination};
use crate::error::{AppError, Resource};
use crate::extract::SafePath;
use crate::log::LogHandle;
use crate::openapi::{FilteredListErrors, LookupErrors};
use crate::query::Query;
use crate::routes::permanent_redirect;
use crate::slug;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/poets",
    tag = "poets",
    operation_id = "poets.list",
    description = "A page of 30 poets with their poem counts, most poems first, then by name. Narrow to one era, or search names with `q`, which orders by relevance and keeps the same order for ties.",
    params(
        ("page" = Option<String>, Query, description = "Page number as a 1-based integer string, 30 poets a page. Minimum 1, maximum 1666: Elasticsearch stops paging after 50,000 results.", pattern = "^[1-9][0-9]*$", example = "1"),
        ("era" = Option<String>, Query, description = "Narrow to the poets of one era. A single `slug` from GET /eras: unlike `GET /poems`, this filter takes one value.", pattern = "^[a-z][a-z-]*$", example = "jahili"),
        ("q" = Option<String>, Query, description = "Search poet names and nicknames, up to 100 characters. A poet is listed when every word matches the name or the nickname as a stem, or every word is the start of a word of them (two letters or more), or every word is within a typo of a word of the name. A typo is not allowed in a word of up to three letters; one is allowed from four letters and two from seven, and the first letter must be right.", max_length = 100, example = "زهير بن أبي سلمى"),
    ),
    responses(
        (status = 200, description = "A page of poets with pagination metadata.", body = ListEnvelope<PoetListItem>),
        FilteredListErrors,
    ),
)]
pub(crate) async fn list(
    State(state): State<AppState>,
    Extension(log): Extension<LogHandle>,
    RawQuery(raw): RawQuery,
) -> Result<Json<ListEnvelope<PoetListItem>>, AppError> {
    let query = Query::parse(raw.as_deref());
    let page = query.page(LIST_POETS_MAX_PAGE)?;
    let q = query
        .text("q", MAX_QUERY_LENGTH)?
        .map(|raw| search::normalize_query(&raw))
        .unwrap_or_default();
    let era = query.scalar_slug("era", slug::transliterated)?;

    let found = state
        .search
        .list_poets(&PoetSearchParams {
            q,
            page,
            era_slugs: era.into_iter().collect(),
            page_size: POEMS_PER_PAGE,
            sort: PoetSort::PoemsCount,
            exact: false,
            window: POETS_LIST_MAX_RESULT_WINDOW,
        })
        .await?;

    log.set("result_count", found.total);
    log.set("page", page);
    log.set("page_size", POEMS_PER_PAGE);
    let envelope = ListEnvelope {
        data: found.hits.into_iter().map(PoetListItem::from).collect(),
        pagination: build_pagination(page, POEMS_PER_PAGE, found.total),
    };
    Ok(Json(envelope))
}

#[utoipa::path(
    get,
    path = "/poets/slugs",
    tag = "poets",
    operation_id = "poets.listSlugs",
    description = "The slug of every poet with at least one poem, with an avatar flag, 45,000 a page in slug order, for sitemaps and incremental crawling.",
    params(
        ("page" = Option<String>, Query, description = "Page number as a 1-based integer string, 45,000 slugs a page. Minimum 1.", pattern = "^[1-9][0-9]*$", example = "1"),
    ),
    responses(
        (status = 200, description = "A page of poet slugs, each flagged with whether the poet has an avatar.", body = ListEnvelope<PoetSlugEntry>),
        FilteredListErrors,
    ),
)]
pub(crate) async fn list_slugs(
    State(state): State<AppState>,
    Extension(log): Extension<LogHandle>,
    RawQuery(raw): RawQuery,
) -> Result<Json<ListEnvelope<PoetSlugEntry>>, AppError> {
    let page = Query::parse(raw.as_deref()).unbounded_page()?;
    let (data, total) = tokio::try_join!(
        state.poets.list_slugs(page, SITEMAP_POETS_PER_SHARD),
        state.poets.count_with_poems()
    )?;
    log.set(
        "result_count",
        u64::try_from(data.len()).unwrap_or(u64::MAX),
    );
    log.set("page", page);
    let envelope = ListEnvelope {
        data: data.into_iter().map(PoetSlugEntry::from).collect(),
        pagination: build_pagination(page, SITEMAP_POETS_PER_SHARD, total.cast_unsigned()),
    };
    Ok(Json(envelope))
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
    Extension(log): Extension<LogHandle>,
    SafePath(raw): SafePath<String>,
) -> Result<Response, AppError> {
    let slug = slug::four_letters(&raw)?;
    log.set("poet_id", slug);
    let Some(poet) = state.poets.get(slug).await? else {
        let Some(survivor) = state.poets.alias_target(slug).await? else {
            return Err(AppError::NotFound(Resource::Poet));
        };
        log.set("alias_of", survivor.clone());
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
