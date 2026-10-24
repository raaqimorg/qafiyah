use axum::extract::State;
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use rand::RngExt;
use serde::Deserialize;

use crate::constants::{
    API_V1_PREFIX, NO_STORE_CACHE_CONTROL, POEMS_PER_PAGE, READ_CACHE_CONTROL,
    SITEMAP_POEMS_PER_SHARD,
};
use crate::contract::poems::{PoemDetail, PoemFacets, PoemListItem, Total};
use crate::domain::poems::{self, Facets, RandomPoemOption};
use crate::envelope::{ItemEnvelope, PageEnvelope};
use crate::error::{AppError, Resource};
use crate::extract::{SafePath, SafeQuery, invalid_path_slug};
use crate::openapi::{ListErrors, LookupErrors};
use crate::params::{
    FourLetterSlug, IdCursor, NoParams, PageCursor, PoetSlugs, RandomPoemOptionParam, SlugsParams,
    TermSlugs,
};
use crate::routes::permanent_redirect;
use crate::state::AppState;

#[derive(Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(crate) struct PoemsParams {
    /// The `next` or `prev` value of a page of this list. Leave it out for the first page.
    #[param(inline, example = "eyJhZnRlciI6MX0")]
    cursor: Option<IdCursor>,
    /// Filter by poet. Repeatable. Values are `slug` from GET /poets.
    #[serde(default)]
    #[param(inline, example = json!(["PAKT"]))]
    poet: PoetSlugs,
    /// Filter by era. Repeatable. Values are `slug` from GET /eras.
    #[serde(default)]
    #[param(inline, example = json!(["jahili"]))]
    era: TermSlugs,
    /// Filter by theme. Repeatable. Values are `slug` from GET /themes.
    #[serde(default)]
    #[param(inline, example = json!(["alhikma"]))]
    theme: TermSlugs,
    /// Filter by meter. Repeatable. Values are `slug` from GET /meters.
    #[serde(default)]
    #[param(inline, example = json!(["altawil"]))]
    meter: TermSlugs,
    /// Filter by rhyme. Repeatable. Values are `slug` from GET /rhymes.
    #[serde(default)]
    #[param(inline, example = json!(["meem"]))]
    rhyme: TermSlugs,
    /// Filter by collection. Repeatable. Values are `slug` from GET /collections.
    #[serde(default)]
    #[param(inline, example = json!(["almuallaqat"]))]
    collection: TermSlugs,
}

#[utoipa::path(
    get,
    path = "/poems",
    tag = "poems",
    operation_id = "poems.list",
    description = "A page of 30 poems in catalog order, oldest entries first, holding primary readings only (a poem's alternate readings are listed on the poem). Pass the page's `next` or `prev` as `cursor` for the page after or before it, with the same filters. Filter by poet, era, theme, meter, rhyme, and collection: values of one filter combine with OR and different filters with AND. A slug that matches nothing gives an empty page rather than an error.",
    params(PoemsParams),
    responses(
        (status = 200, description = "A page of poems, with the cursors of the pages around it.", body = PageEnvelope<PoemListItem>),
        ListErrors,
    ),
)]
pub(crate) async fn list(
    State(state): State<AppState>,
    SafeQuery(params): SafeQuery<PoemsParams>,
) -> Result<Json<PageEnvelope<PoemListItem>>, AppError> {
    let cursor = params.cursor.map(PageCursor::into_inner);
    let facets = Facets {
        poet: params.poet.into_strings(),
        era: params.era.into_strings(),
        theme: params.theme.into_strings(),
        meter: params.meter.into_strings(),
        rhyme: params.rhyme.into_strings(),
        collection: params.collection.into_strings(),
    };
    let (poems, total) = state
        .poems
        .list(&facets, cursor.as_ref(), POEMS_PER_PAGE)
        .await?;
    crate::log::record_results(u64::try_from(total).unwrap_or(0));
    Ok(Json(PageEnvelope::new(
        poems,
        total.cast_unsigned(),
        PoemListItem::from,
    )))
}

#[utoipa::path(
    get,
    path = "/poems/slugs",
    tag = "poems",
    operation_id = "poems.listSlugs",
    description = "The slug of every primary poem, 45,000 a page in catalog order, oldest entries first, for sitemaps and incremental crawling. A new poem joins the last page, so a crawler that kept the last `next` it saw (or the last page's `prev`) only reads what came after.",
    params(SlugsParams),
    responses(
        (status = 200, description = "A page of poem slugs.", body = PageEnvelope<FourLetterSlug>),
        ListErrors,
    ),
)]
pub(crate) async fn list_slugs(
    State(state): State<AppState>,
    SafeQuery(params): SafeQuery<SlugsParams>,
) -> Result<Json<PageEnvelope<String>>, AppError> {
    let cursor = params.cursor.map(PageCursor::into_inner);
    let (slugs, total) = tokio::try_join!(
        state
            .poems
            .list_slugs(cursor.as_ref(), SITEMAP_POEMS_PER_SHARD),
        state.poems.count()
    )?;
    crate::log::record_results(u64::try_from(slugs.items.len()).unwrap_or(u64::MAX));
    Ok(Json(PageEnvelope::new(
        slugs,
        total.cast_unsigned(),
        |slug| slug,
    )))
}

#[utoipa::path(
    get,
    path = "/poems/count",
    tag = "poems",
    operation_id = "poems.count",
    description = "The number of poems in the catalog, counting primary readings only: the same poems `GET /poems` pages through with no filter.",
    responses(
        (status = 200, description = "The total poem count.", body = ItemEnvelope<Total>),
        ListErrors,
    ),
)]
pub(crate) async fn count(
    State(state): State<AppState>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ItemEnvelope<Total>>, AppError> {
    let total = state.poems.count().await?;
    crate::log::record_results(u64::try_from(total).unwrap_or(0));
    Ok(Json(ItemEnvelope {
        data: Total { total },
    }))
}

#[derive(Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(crate) struct FacetsParams {
    /// The poet whose poems are counted. A single `slug` from GET /poets.
    #[param(inline, example = "PAKT")]
    poet: FourLetterSlug,
    /// Narrow the rhyme and theme counts to poems of these meters. Repeatable. Values are `slug` from GET /meters.
    #[serde(default)]
    #[param(inline, example = json!(["altawil"]))]
    meter: TermSlugs,
    /// Narrow the meter and theme counts to poems of these rhymes. Repeatable. Values are `slug` from GET /rhymes.
    #[serde(default)]
    #[param(inline, example = json!(["meem"]))]
    rhyme: TermSlugs,
    /// Narrow the meter and rhyme counts to poems of these themes. Repeatable. Values are `slug` from GET /themes.
    #[serde(default)]
    #[param(inline, example = json!(["alhikma"]))]
    theme: TermSlugs,
}

#[utoipa::path(
    get,
    path = "/poems/facets",
    tag = "poems",
    operation_id = "poems.facets",
    description = "The meters, rhymes, and themes of one poet's poems, each with a poem count, for building filters over `GET /poems?poet=`. Narrow with the same `meter`, `rhyme`, and `theme` params as `GET /poems`: each list is counted under the other two filters but not its own, so it keeps every value that can still be added, and a selected value stays listed even at a count of zero. Values with no matching poem are left out. Lists are ordered by poem count descending, then by name.",
    params(FacetsParams),
    responses(
        (status = 200, description = "The poet's meters, rhymes, and themes with poem counts under the given filters.", body = ItemEnvelope<PoemFacets>),
        LookupErrors,
    ),
)]
pub(crate) async fn facet_counts(
    State(state): State<AppState>,
    SafeQuery(params): SafeQuery<FacetsParams>,
) -> Result<Json<ItemEnvelope<PoemFacets>>, AppError> {
    let poet = params.poet.into_inner();
    tracing::Span::current().record("poet_id", poet.as_str());
    let facets = Facets {
        poet: vec![poet],
        meter: params.meter.into_strings(),
        rhyme: params.rhyme.into_strings(),
        theme: params.theme.into_strings(),
        ..Facets::default()
    };
    let counts = poems::facets(state.poems.as_ref(), &facets).await?;
    Ok(Json(ItemEnvelope {
        data: PoemFacets::from(counts),
    }))
}

#[utoipa::path(
    get,
    path = "/poems/{slug}",
    tag = "poems",
    operation_id = "poems.get",
    description = "A poem by slug: its full verse text, its classification, its neighbors in the poet's list, its alternate readings, and up to 10 related poems. An alternate reading has its own slug and names its primary in `recensionOf`. A slug that was merged into another poem answers 301 to the surviving poem.",
    params(
        ("slug" = String, Path, description = "The poem's `slug`, from a list or search response.", pattern = "^[a-zA-Z]{4}$", example = "gnNg"),
    ),
    responses(
        (status = 200, description = "The requested poem.", body = ItemEnvelope<PoemDetail>),
        (status = 301, description = "The slug belongs to a poem merged into another; `Location` names the surviving poem.", headers(("Location" = String, description = "Path of the surviving poem"))),
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
    let Some(poem) = poems::get(state.poems.as_ref(), slug).await? else {
        let Some(survivor) = state.poems.alias_target(slug).await? else {
            return Err(AppError::NotFound(Resource::Poem));
        };
        tracing::Span::current().record("poem_id", slug);
        tracing::Span::current().record("alias_of", survivor.as_str());
        return Ok(permanent_redirect(
            &format!("{API_V1_PREFIX}/poems/{survivor}"),
            READ_CACHE_CONTROL,
        ));
    };
    tracing::Span::current().record("poem_id", slug);
    tracing::Span::current().record("poet_id", poem.poet.slug.as_str());
    tracing::Span::current().record("era", poem.era.slug.as_str());
    tracing::Span::current().record("meter", poem.meter.slug.as_str());
    tracing::Span::current().record("theme", poem.theme.slug.as_str());
    Ok(Json(ItemEnvelope {
        data: PoemDetail::from(poem),
    })
    .into_response())
}

#[derive(Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(crate) struct RandomParams {
    /// What the body holds: `slug` (the default) or `lines`.
    #[param(example = "slug")]
    option: Option<RandomPoemOptionParam>,
}

#[utoipa::path(
    get,
    path = "/poems/random",
    tag = "poems",
    operation_id = "poems.random",
    description = "A random poem, as plain text that is never cached. By default, or with `option=slug`, the body is the poem's slug, for `GET /poems/{slug}`. With `option=lines` it is one verse of the poem, its two half-lines on two lines, then a blank line and the poet's name, at most 280 characters. A poet is picked at random first and then one of their poems, so every poet is equally likely. A poem is eligible when it is a primary reading by a named poet of the jahili, islami, umawi, or abbasi era, in the amudi form, at least four verses long, of a known meter, vocalized (at least 0.3 harakat for each letter), and has at least one stored row with two half-lines.",
    params(RandomParams),
    responses(
        (status = 200, description = "The slug, or with `option=lines` one verse and the poet's name.", content_type = "text/plain", body = String, example = "gnNg"),
        ListErrors,
    ),
)]
pub(crate) async fn random(
    State(state): State<AppState>,
    SafeQuery(params): SafeQuery<RandomParams>,
) -> Result<Response, AppError> {
    let option = match params.option {
        None | Some(RandomPoemOptionParam::Slug) => RandomPoemOption::Slug,
        Some(RandomPoemOptionParam::Lines) => RandomPoemOption::Lines,
    };
    let roll: f64 = rand::rng().random();
    let body = poems::random(state.poems.as_ref(), &option, roll).await?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=UTF-8"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static(NO_STORE_CACHE_CONTROL),
            ),
        ],
        body,
    )
        .into_response())
}

pub fn uncached_router() -> Router<AppState> {
    Router::new().route("/poems/random", get(random))
}
