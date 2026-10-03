use axum::Json;
use axum::extract::{Extension, RawQuery, State};
use serde::Serialize;
use utoipa::ToSchema;

use crate::constants::{
    MAX_FILTER_SLUGS, MAX_QUERY_LENGTH, SEARCH_POEMS_MAX_PAGE, SEARCH_POEMS_PER_PAGE,
    SEARCH_POETS_MAX_PAGE, SEARCH_POETS_PER_PAGE,
};
use crate::domain::search::{self, PoemResult, PoetResult};
use crate::domain::search::{PoemSearchParams, PoetSearchParams};
use crate::envelope::{ListEnvelope, build_pagination};
use crate::error::AppError;
use crate::log::LogHandle;
use crate::openapi::{
    ExactFlag, FilteredListErrors, FourLetterSlug, SearchTypeParam, TransliteratedSlug,
};
use crate::query::Query;

use crate::slug;
use crate::state::AppState;

#[derive(Serialize, ToSchema)]
pub(crate) struct SearchResponse {
    q: String,
    poems: Option<ListEnvelope<PoemResult>>,
    poets: Option<ListEnvelope<PoetResult>>,
}

#[utoipa::path(
    get,
    path = "/search",
    tag = "search",
    operation_id = "search.search",
    description = "Full-text search over poems and poets with optional facet filters. Array filters are repeatable params, e.g. ?eraSlugs=andalusi&meterSlugs=altawil. Poets are filterable by era only; meter, rhyme, theme, verse form, and collection filters apply to poems and are rejected with a 400 when the `poets` result type is requested. Unknown query params are ignored.",
    params(
        ("q" = Option<String>, Query, description = "Search query in Arabic. When empty, the results are browsed rather than matched: poems most recently added first, classical eras first when no era is chosen, and poets most recently added first, all narrowed by the filters.", max_length = 100, example = "المتنبي"),
        ("types" = Option<Vec<SearchTypeParam>>, Query, description = "Result types to include. Defaults to all types when omitted.", max_items = 2),
        ("poemsPage" = Option<String>, Query, description = "Page number as a 1-based integer string. Minimum 1, maximum 500 (offsets past this exceed the Elasticsearch result window).", pattern = "^[1-9][0-9]*$", example = "1"),
        ("poetsPage" = Option<String>, Query, description = "Page number as a 1-based integer string. Minimum 1, maximum 500 (offsets past this exceed the Elasticsearch result window).", pattern = "^[1-9][0-9]*$", example = "1"),
        ("poetSlugs" = Option<Vec<FourLetterSlug>>, Query, description = "Filter results by poet slug. Repeatable array param, e.g. ?poetSlugs=yoFB. Slugs are the `slug` values from GET /poets.", example = json!(["yoFB"])),
        ("eraSlugs" = Option<Vec<TransliteratedSlug>>, Query, description = "Filter results by era slug. Repeatable array param, e.g. ?eraSlugs=abbasi. Slugs are the `slug` values from GET /eras.", example = json!(["abbasi"])),
        ("meterSlugs" = Option<Vec<TransliteratedSlug>>, Query, description = "Filter poems by meter slug. Applies to the poems result set only and cannot be combined with the `poets` result type. Repeatable array param, e.g. ?meterSlugs=altawil. Slugs are the `slug` values from GET /meters.", example = json!(["altawil"])),
        ("rhymeSlugs" = Option<Vec<TransliteratedSlug>>, Query, description = "Filter poems by rhyme slug. Applies to the poems result set only and cannot be combined with the `poets` result type. Repeatable array param, e.g. ?rhymeSlugs=meem. Slugs are the `slug` values from GET /rhymes.", example = json!(["meem"])),
        ("themeSlugs" = Option<Vec<TransliteratedSlug>>, Query, description = "Filter poems by theme slug. Applies to the poems result set only and cannot be combined with the `poets` result type. Repeatable array param, e.g. ?themeSlugs=alnasib. Slugs are the `slug` values from GET /themes.", example = json!(["alnasib"])),
        ("poemTypeSlugs" = Option<Vec<TransliteratedSlug>>, Query, description = "Filter poems by verse form slug (amudi, hurr, and the rest). Applies to the poems result set only and cannot be combined with the `poets` result type. Repeatable array param, e.g. ?poemTypeSlugs=hurr. Slugs are the `slug` values from GET /poem-types.", example = json!(["hurr"])),
        ("collectionSlugs" = Option<Vec<TransliteratedSlug>>, Query, description = "Filter poems by collection slug. Applies to the poems result set only and cannot be combined with the `poets` result type. Repeatable array param, e.g. ?collectionSlugs=almuallaqat. Slugs are the `slug` values from GET /collections.", example = json!(["almuallaqat"])),
        ("exact" = Option<ExactFlag>, Query, description = "When true, match the literal phrase only (poem title or text, and poet name or nickname), with no stemming, fuzzy, or autocomplete expansion (Arabic letter normalization still applies, except that a standalone hamza in a poem phrase must match as typed). Applies to both result sets.", example = "false"),
    ),
    responses(
        (status = 200, description = "Echoed query plus the requested poem and poet result sections.", body = SearchResponse),
        FilteredListErrors,
    ),
)]
pub(crate) async fn search(
    State(state): State<AppState>,
    Extension(log): Extension<LogHandle>,
    RawQuery(raw): RawQuery,
) -> Result<Json<SearchResponse>, AppError> {
    let query = Query::parse(raw.as_deref());
    let q = query
        .text("q", MAX_QUERY_LENGTH)?
        .map(|raw| search::normalize_query(&raw))
        .unwrap_or_default();
    let types = query.types()?;
    let poems_page = query.named_page("poemsPage", SEARCH_POEMS_MAX_PAGE)?;
    let poets_page = query.named_page("poetsPage", SEARCH_POETS_MAX_PAGE)?;
    let exact = query.boolean("exact")?;

    let facet = |name: &str, validate: fn(&str) -> Result<&str, AppError>| {
        query.facet(name, validate, MAX_FILTER_SLUGS)
    };
    let poet_slugs = facet("poetSlugs", slug::four_letters)?;
    let era_slugs = facet("eraSlugs", slug::transliterated)?;
    let meter_slugs = facet("meterSlugs", slug::transliterated)?;
    let rhyme_slugs = facet("rhymeSlugs", slug::transliterated)?;
    let theme_slugs = facet("themeSlugs", slug::transliterated)?;
    let poem_type_slugs = facet("poemTypeSlugs", slug::transliterated)?;
    let collection_slugs = facet("collectionSlugs", slug::transliterated)?;

    let want_poems = types.iter().any(|t| t == "poems");
    let want_poets = types.iter().any(|t| t == "poets");
    let poem_only = [
        &meter_slugs,
        &rhyme_slugs,
        &theme_slugs,
        &poem_type_slugs,
        &collection_slugs,
    ];
    if want_poets && poem_only.iter().any(|values| !values.is_empty()) {
        return Err(AppError::BadRequest);
    }

    let poem_params = PoemSearchParams {
        q: q.clone(),
        page: poems_page,
        page_size: SEARCH_POEMS_PER_PAGE,
        poet_slugs: poet_slugs.clone(),
        era_slugs: era_slugs.clone(),
        meter_slugs,
        theme_slugs,
        rhyme_slugs,
        poem_type_slugs,
        collection_slugs,
        exact,
    };
    let poet_params = PoetSearchParams {
        q: q.clone(),
        page: poets_page,
        page_size: SEARCH_POETS_PER_PAGE,
        era_slugs: era_slugs.clone(),
        exact,
        ..PoetSearchParams::default()
    };
    let poems = async {
        if !want_poems {
            return Ok(None);
        }
        state.search.search_poems(&poem_params).await.map(Some)
    };
    let poets = async {
        if !want_poets {
            return Ok(None);
        }
        state.search.search_poets(&poet_params).await.map(Some)
    };
    let (poems, poets) = tokio::try_join!(poems, poets)?;

    if !q.is_empty() {
        log.set("query_text", q.clone());
    }
    log.set(
        "poems_count",
        poems
            .as_ref()
            .map_or(0, |page| u64::try_from(page.hits.len()).unwrap_or(u64::MAX)),
    );
    log.set(
        "poets_count",
        poets
            .as_ref()
            .map_or(0, |page| u64::try_from(page.hits.len()).unwrap_or(u64::MAX)),
    );
    let found = u64::from(poems.as_ref().map_or(0, |page| page.total))
        .saturating_add(u64::from(poets.as_ref().map_or(0, |page| page.total)));
    log.set("result_count", found);
    Ok(Json(SearchResponse {
        q,
        poems: poems.map(|page| ListEnvelope {
            data: page.hits,
            pagination: build_pagination(poems_page, poem_params.page_size, page.total),
        }),
        poets: poets.map(|page| ListEnvelope {
            data: page.hits,
            pagination: build_pagination(poets_page, poet_params.page_size, page.total),
        }),
    }))
}
