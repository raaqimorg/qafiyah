use axum::extract::{Extension, RawQuery, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use rand::RngExt;

use crate::constants::{
    API_V1_PREFIX, MAX_FILTER_SLUGS, NO_STORE_CACHE_CONTROL, POEMS_PER_PAGE, READ_CACHE_CONTROL,
    SITEMAP_POEMS_PER_SHARD,
};
use crate::contract::poems::{PoemDetail, PoemFacets, PoemListItem, Total};
use crate::domain::poems::{self, Facets, RandomPoemOption};
use crate::envelope::{ItemEnvelope, ListEnvelope, build_pagination};
use crate::error::{AppError, Resource, RouteProblem};
use crate::extract::SafePath;
use crate::log::LogHandle;
use crate::openapi::{
    FilteredListErrors, FourLetterSlug, ListErrors, LookupErrors, RandomPoemOptionParam,
    TransliteratedSlug,
};
use crate::query::Query;
use crate::routes::permanent_redirect;
use crate::slug;
use crate::state::AppState;

fn facets(query: &Query) -> Result<Facets, AppError> {
    Ok(Facets {
        poet: query.facet("poet", slug::four_letters, MAX_FILTER_SLUGS)?,
        era: query.facet("era", slug::transliterated, MAX_FILTER_SLUGS)?,
        theme: query.facet("theme", slug::transliterated, MAX_FILTER_SLUGS)?,
        meter: query.facet("meter", slug::transliterated, MAX_FILTER_SLUGS)?,
        rhyme: query.facet("rhyme", slug::transliterated, MAX_FILTER_SLUGS)?,
        collection: query.facet("collection", slug::transliterated, MAX_FILTER_SLUGS)?,
    })
}

#[utoipa::path(
    get,
    path = "/poems",
    tag = "poems",
    operation_id = "poems.list",
    description = "A page of 30 poems in catalog order, oldest entries first, holding primary readings only (a poem's alternate readings are listed on the poem). Filter by poet, era, theme, meter, rhyme, and collection: values of one filter combine with OR and different filters with AND, e.g. `?poet=PAKT&meter=altawil&meter=alkamil`. A slug that matches nothing gives an empty page rather than an error, and unknown query params are ignored.",
    params(
        ("page" = Option<String>, Query, description = "Page number as a 1-based integer string, 30 poems a page. Minimum 1.", pattern = "^[1-9][0-9]*$", example = "1"),
        ("poet" = Option<Vec<FourLetterSlug>>, Query, description = "Filter by poet. Repeatable, e.g. `?poet=PAKT`. Values are `slug` from GET /poets.", example = json!(["PAKT"])),
        ("era" = Option<Vec<TransliteratedSlug>>, Query, description = "Filter by era. Repeatable, e.g. `?era=jahili`. Values are `slug` from GET /eras.", example = json!(["jahili"])),
        ("theme" = Option<Vec<TransliteratedSlug>>, Query, description = "Filter by theme. Repeatable, e.g. `?theme=alhikma`. Values are `slug` from GET /themes.", example = json!(["alhikma"])),
        ("meter" = Option<Vec<TransliteratedSlug>>, Query, description = "Filter by meter. Repeatable, e.g. `?meter=altawil`. Values are `slug` from GET /meters.", example = json!(["altawil"])),
        ("rhyme" = Option<Vec<TransliteratedSlug>>, Query, description = "Filter by rhyme. Repeatable, e.g. `?rhyme=meem`. Values are `slug` from GET /rhymes.", example = json!(["meem"])),
        ("collection" = Option<Vec<TransliteratedSlug>>, Query, description = "Filter by collection. Repeatable, e.g. `?collection=almuallaqat`. Values are `slug` from GET /collections.", example = json!(["almuallaqat"])),
    ),
    responses(
        (status = 200, description = "A page of poems with pagination metadata.", body = ListEnvelope<PoemListItem>),
        FilteredListErrors,
    ),
)]
pub(crate) async fn list(
    State(state): State<AppState>,
    Extension(log): Extension<LogHandle>,
    RawQuery(raw): RawQuery,
) -> Result<Json<ListEnvelope<PoemListItem>>, AppError> {
    let query = Query::parse(raw.as_deref());
    let page = query.unbounded_page()?;
    let facets = facets(&query)?;
    let (poems, total) = state.poems.list(&facets, page, POEMS_PER_PAGE).await?;
    let envelope = ListEnvelope {
        data: poems.into_iter().map(PoemListItem::from).collect(),
        pagination: build_pagination(page, POEMS_PER_PAGE, total.cast_unsigned()),
    };
    log.set("result_count", total);
    log.set("page", page);
    log.set("page_size", POEMS_PER_PAGE);
    log.set("total_pages", envelope.pagination.total_pages);
    Ok(Json(envelope))
}

#[utoipa::path(
    get,
    path = "/poems/slugs",
    tag = "poems",
    operation_id = "poems.listSlugs",
    description = "The slug of every primary poem, 45,000 a page in slug order, for sitemaps and incremental crawling.",
    params(
        ("page" = Option<String>, Query, description = "Page number as a 1-based integer string, 45,000 slugs a page. Minimum 1.", pattern = "^[1-9][0-9]*$", example = "1"),
    ),
    responses(
        (status = 200, description = "A page of poem slugs.", body = ListEnvelope<FourLetterSlug>),
        FilteredListErrors,
    ),
)]
pub(crate) async fn list_slugs(
    State(state): State<AppState>,
    Extension(log): Extension<LogHandle>,
    RawQuery(raw): RawQuery,
) -> Result<Json<ListEnvelope<String>>, AppError> {
    let page = Query::parse(raw.as_deref()).unbounded_page()?;
    let (data, total) = tokio::try_join!(
        state.poems.list_slugs(page, SITEMAP_POEMS_PER_SHARD),
        state.poems.count()
    )?;
    log.set(
        "result_count",
        u64::try_from(data.len()).unwrap_or(u64::MAX),
    );
    log.set("page", page);
    let envelope = ListEnvelope {
        data,
        pagination: build_pagination(page, SITEMAP_POEMS_PER_SHARD, total.cast_unsigned()),
    };
    Ok(Json(envelope))
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
    Extension(log): Extension<LogHandle>,
) -> Result<Json<ItemEnvelope<Total>>, AppError> {
    let total = state.poems.count().await?;
    log.set("result_count", total);
    Ok(Json(ItemEnvelope {
        data: Total { total },
    }))
}

#[utoipa::path(
    get,
    path = "/poems/facets",
    tag = "poems",
    operation_id = "poems.facets",
    description = "The meters, rhymes, and themes of one poet's poems, each with a poem count, for building filters over `GET /poems?poet=`. Narrow with the same `meter`, `rhyme`, and `theme` params as `GET /poems`: each list is counted under the other two filters but not its own, so it keeps every value that can still be added, and a selected value stays listed even at a count of zero. Values with no matching poem are left out. Lists are ordered by poem count descending, then by name.",
    params(
        ("poet" = String, Query, description = "The poet whose poems are counted. A single `slug` from GET /poets.", pattern = "^[a-zA-Z]{4}$", example = "PAKT"),
        ("meter" = Option<Vec<TransliteratedSlug>>, Query, description = "Narrow the rhyme and theme counts to poems of these meters. Repeatable, e.g. `?meter=altawil`. Values are `slug` from GET /meters.", example = json!(["altawil"])),
        ("rhyme" = Option<Vec<TransliteratedSlug>>, Query, description = "Narrow the meter and theme counts to poems of these rhymes. Repeatable, e.g. `?rhyme=meem`. Values are `slug` from GET /rhymes.", example = json!(["meem"])),
        ("theme" = Option<Vec<TransliteratedSlug>>, Query, description = "Narrow the meter and rhyme counts to poems of these themes. Repeatable, e.g. `?theme=alhikma`. Values are `slug` from GET /themes.", example = json!(["alhikma"])),
    ),
    responses(
        (status = 200, description = "The poet's meters, rhymes, and themes with poem counts under the given filters.", body = ItemEnvelope<PoemFacets>),
        LookupErrors,
    ),
)]
pub(crate) async fn facet_counts(
    State(state): State<AppState>,
    Extension(log): Extension<LogHandle>,
    RawQuery(raw): RawQuery,
) -> Result<Json<ItemEnvelope<PoemFacets>>, AppError> {
    let query = Query::parse(raw.as_deref());
    let poet = query
        .scalar_slug("poet", slug::four_letters)?
        .ok_or(AppError::BadRequest)?;
    log.set("poet_id", poet.clone());
    let facets = Facets {
        poet: vec![poet],
        meter: query.facet("meter", slug::transliterated, MAX_FILTER_SLUGS)?,
        rhyme: query.facet("rhyme", slug::transliterated, MAX_FILTER_SLUGS)?,
        theme: query.facet("theme", slug::transliterated, MAX_FILTER_SLUGS)?,
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
    Extension(log): Extension<LogHandle>,
    SafePath(raw): SafePath<String>,
) -> Result<Response, AppError> {
    let slug = slug::four_letters(&raw)?;
    let Some(poem) = poems::get(state.poems.as_ref(), slug).await? else {
        let Some(survivor) = state.poems.alias_target(slug).await? else {
            return Err(AppError::NotFound(Resource::Poem));
        };
        log.set("poem_id", slug);
        log.set("alias_of", survivor.clone());
        return Ok(permanent_redirect(
            &format!("{API_V1_PREFIX}/poems/{survivor}"),
            READ_CACHE_CONTROL,
        ));
    };
    log.set("poem_id", slug);
    log.set("poet_id", poem.poet.slug.clone());
    log.set("era", poem.era.slug.clone());
    log.set("meter", poem.meter.slug.clone());
    log.set("theme", poem.theme.slug.clone());
    Ok(Json(ItemEnvelope {
        data: PoemDetail::from(poem),
    })
    .into_response())
}

fn random_option(raw: Option<&str>) -> Result<RandomPoemOption, AppError> {
    match raw {
        None | Some("slug") => Ok(RandomPoemOption::Slug),
        Some("lines") => Ok(RandomPoemOption::Lines),
        Some(_) => Err(RouteProblem::bad_request(
            "Invalid ?option value (expected 'slug' or 'lines')",
        )
        .into()),
    }
}

#[utoipa::path(
    get,
    path = "/poems/random",
    tag = "poems",
    operation_id = "poems.random",
    description = "A random poem, as plain text that is never cached. By default, or with `option=slug`, the body is the poem's slug, for `GET /poems/{slug}`. With `option=lines` it is one verse of the poem, its two half-lines on two lines, then a blank line and the poet's name, at most 280 characters. A poet is picked at random first and then one of their poems, so every poet is equally likely. A poem is eligible when it is a primary reading by a named poet of the jahili, islami, umawi, or abbasi era, in the amudi form, at least four verses long, and of a known meter.",
    params(
        ("option" = Option<RandomPoemOptionParam>, Query, description = "What the body holds: `slug` (the default) or `lines`.", example = "slug"),
    ),
    responses(
        (status = 200, description = "The slug, or with `option=lines` one verse and the poet's name.", content_type = "text/plain", body = String, example = "gnNg"),
        FilteredListErrors,
    ),
)]
pub(crate) async fn random(
    State(state): State<AppState>,
    RawQuery(raw): RawQuery,
) -> Result<Response, AppError> {
    let option = random_option(Query::parse(raw.as_deref()).first("option").as_deref())?;
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
