use axum::Json;
use axum::extract::{Extension, RawQuery, State};
use axum::response::{IntoResponse, Response};

use crate::constants::{
    API_V1_PREFIX, LIST_POETS_MAX_PAGE, MAX_QUERY_LENGTH, POEMS_PER_PAGE,
    POETS_LIST_MAX_RESULT_WINDOW, READ_CACHE_CONTROL, SITEMAP_POETS_PER_SHARD,
};
use crate::domain::poets::{self, PoetSlugEntry, PoetStats};
use crate::domain::search::{self, PoetListItem};
use crate::envelope::{ItemEnvelope, ListEnvelope, build_pagination};
use crate::error::{AppError, Resource};
use crate::es::query::{PoetSearchParams, PoetSort};
use crate::extract::SafePath;
use crate::js;
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
    description = "Paginated, Elasticsearch-backed list of poets with poem counts, ordered by poem count descending. Narrow by era or by a full-text name query `q`, which orders by relevance instead.",
    params(
        ("page" = Option<String>, Query, description = "Page number as a 1-based integer string. Minimum 1, maximum 1666 (offsets past this exceed the Elasticsearch result window).", pattern = "^[1-9][0-9]*$", example = "1"),
        ("era" = Option<String>, Query, description = "Filter to poets of a single era. Value is a `slug` from GET /eras.", pattern = "^[a-z][a-z-]*$", example = "abbasi"),
        ("q" = Option<String>, Query, description = "Full-text query matched against poet names.", max_length = 50, example = "المتنبي"),
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
        .map(|raw| js::trim(&raw).to_string())
        .unwrap_or_default();
    let era = query.scalar_slug("era", slug::transliterated)?;

    let found = search::list_poets(
        &state.es,
        &PoetSearchParams {
            q,
            page,
            era_slugs: era.into_iter().collect(),
            page_size: POEMS_PER_PAGE,
            sort: PoetSort::PoemsCount,
            highlight: false,
            exact: false,
            window: POETS_LIST_MAX_RESULT_WINDOW,
        },
    )
    .await?;

    log.set("result_count", found.total);
    log.set("page", page);
    log.set("page_size", POEMS_PER_PAGE);
    let envelope = ListEnvelope {
        data: found.hits,
        pagination: build_pagination(page, POEMS_PER_PAGE, found.total),
    };
    Ok(Json(envelope))
}

#[utoipa::path(
    get,
    path = "/poets/slugs",
    tag = "poets",
    operation_id = "poets.listSlugs",
    description = "Paginated stream of poet slugs with an avatar flag, intended for sitemap generation and incremental crawling.",
    params(
        ("page" = Option<String>, Query, description = "Page number as a 1-based integer string. Minimum 1.", pattern = "^[1-9][0-9]*$", example = "1"),
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
        poets::list_slugs(&state.pg, page, SITEMAP_POETS_PER_SHARD),
        poets::count(&state.pg)
    )?;
    log.set(
        "result_count",
        u64::try_from(data.len()).unwrap_or(u64::MAX),
    );
    log.set("page", page);
    let envelope = ListEnvelope {
        data,
        pagination: build_pagination(page, SITEMAP_POETS_PER_SHARD, total.cast_unsigned()),
    };
    Ok(Json(envelope))
}

#[utoipa::path(
    get,
    path = "/poets/{slug}",
    tag = "poets",
    operation_id = "poets.get",
    description = "A single poet with nickname, bio, era, avatar flag, and poem count, by slug.",
    params(
        ("slug" = String, Path, description = "Resource identifier taken from the `slug` field of the matching list endpoint.", pattern = "^[a-zA-Z]{4}$", example = "yoFB"),
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
    let poet = match poets::get(&state.pg, slug).await {
        Ok(poet) => poet,
        Err(AppError::NotFound(Resource::Poet)) => {
            let Some(survivor) = poets::alias_target(&state.pg, slug).await? else {
                return Err(AppError::NotFound(Resource::Poet));
            };
            log.set("alias_of", survivor.clone());
            return Ok(permanent_redirect(
                &format!("{API_V1_PREFIX}/poets/{survivor}"),
                READ_CACHE_CONTROL,
            ));
        }
        Err(error) => return Err(error),
    };
    Ok(Json(ItemEnvelope { data: poet }).into_response())
}
