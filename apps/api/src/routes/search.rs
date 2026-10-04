use axum::Json;
use axum::extract::{Extension, RawQuery, State};
use serde::Serialize;
use utoipa::ToSchema;

use crate::constants::{
    MAX_FILTER_SLUGS, MAX_QUERY_LENGTH, SEARCH_POEMS_MAX_PAGE, SEARCH_POEMS_PER_PAGE,
    SEARCH_POETS_MAX_PAGE, SEARCH_POETS_PER_PAGE,
};
use crate::contract::search::{PoemResult, PoetResult};
use crate::domain::search::{self, PoemSearchParams, PoetSearchParams};
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
#[schema(
    description = "The query as searched and the requested sections: `poems` is present when `types` includes poems, `poets` when it includes poets."
)]
pub(crate) struct SearchResponse {
    /// The query as searched: Unicode NFKC, whitespace collapsed and trimmed. Empty when browsing.
    #[schema(example = "أمن أم أوفى")]
    q: String,
    /// The poems section. Left out unless `types` includes poems.
    poems: Option<ListEnvelope<PoemResult>>,
    /// The poets section. Left out unless `types` includes poets.
    poets: Option<ListEnvelope<PoetResult>>,
}

#[utoipa::path(
    get,
    path = "/search",
    tag = "search",
    operation_id = "search.search",
    description = "Full-text search over poems and poets, in two sections paged on their own. Poems match by title and verse text, poets by name and nickname. Without `q` the sections are browsed instead: poems newest first, with the classical eras first unless an era is chosen, and poets newest first, all narrowed by the filters. A poem found in several readings appears once, as its best-matching reading. Each section's `totalItems` stops at 10,000, so 10,000 means 10,000 or more, and the poems total counts poems, not readings. `relevance` is the raw search score, comparable only within one section. Repeat a filter to match any of its values: `?types=poems&eraSlugs=andalusi&meterSlugs=altawil`. The meter, rhyme, theme, verse form, and collection filters apply to poems only and need `types=poems`: with the default `types`, which includes poets, a request using them is refused with 400. Unknown query params are ignored.",
    params(
        ("q" = Option<String>, Query, description = "Search query in Arabic, up to 100 characters. It is normalized (Unicode NFKC, whitespace collapsed and trimmed) and echoed back as `q`. A query of three words or more puts poems holding the words as a phrase above every other result, the oldest classical era first. When empty, the sections are browsed (see above).", max_length = 100, example = "أمن أم أوفى"),
        ("types" = Option<Vec<SearchTypeParam>>, Query, description = "Sections to include: `poems`, `poets`, or both, which is the default. Repeat to send both, e.g. `?types=poems&types=poets`. Send `types=poems` alone to use any poem-only filter.", max_items = 2, example = json!(["poems"])),
        ("poemsPage" = Option<String>, Query, description = "Page of the poems section as a 1-based integer string, 20 results a page. Maximum 500: Elasticsearch stops paging after 10,000 results.", pattern = "^[1-9][0-9]*$", example = "1"),
        ("poetsPage" = Option<String>, Query, description = "Page of the poets section as a 1-based integer string, 20 results a page. Maximum 500: Elasticsearch stops paging after 10,000 results.", pattern = "^[1-9][0-9]*$", example = "1"),
        ("poetSlugs" = Option<Vec<FourLetterSlug>>, Query, description = "Narrow the poems section to these poets. The poets section ignores it. Repeatable, e.g. `?poetSlugs=PAKT`. Values are `slug` from GET /poets.", example = json!(["PAKT"])),
        ("eraSlugs" = Option<Vec<TransliteratedSlug>>, Query, description = "Narrow both sections to these eras. Repeatable, e.g. `?eraSlugs=jahili`. Values are `slug` from GET /eras.", example = json!(["jahili"])),
        ("meterSlugs" = Option<Vec<TransliteratedSlug>>, Query, description = "Narrow the poems section to these meters. Poems only: needs `types=poems`. Repeatable, e.g. `?types=poems&meterSlugs=altawil`. Values are `slug` from GET /meters.", example = json!(["altawil"])),
        ("rhymeSlugs" = Option<Vec<TransliteratedSlug>>, Query, description = "Narrow the poems section to these rhymes. Poems only: needs `types=poems`. Repeatable, e.g. `?types=poems&rhymeSlugs=meem`. Values are `slug` from GET /rhymes.", example = json!(["meem"])),
        ("themeSlugs" = Option<Vec<TransliteratedSlug>>, Query, description = "Narrow the poems section to these themes. Poems only: needs `types=poems`. Repeatable, e.g. `?types=poems&themeSlugs=alhikma`. Values are `slug` from GET /themes.", example = json!(["alhikma"])),
        ("poemTypeSlugs" = Option<Vec<TransliteratedSlug>>, Query, description = "Narrow the poems section to these verse forms (amudi, hurr, and the rest). Poems only: needs `types=poems`. Repeatable, e.g. `?types=poems&poemTypeSlugs=amudi`. Values are `slug` from GET /poem-types.", example = json!(["amudi"])),
        ("collectionSlugs" = Option<Vec<TransliteratedSlug>>, Query, description = "Narrow the poems section to these collections. Poems only: needs `types=poems`. Repeatable, e.g. `?types=poems&collectionSlugs=almuallaqat`. Values are `slug` from GET /collections.", example = json!(["almuallaqat"])),
        ("exact" = Option<ExactFlag>, Query, description = "When true, match the literal phrase only (poem title or text, and poet name or nickname), with no stemming, fuzzy, or autocomplete expansion (Arabic letter normalization still applies, except that a standalone hamza in a poem phrase must match as typed). Applies to both result sets.", example = "false"),
    ),
    responses(
        (status = 200, description = "The normalized query and the requested sections, each with its own results and pagination.", body = SearchResponse),
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
            data: page.hits.into_iter().map(PoemResult::from).collect(),
            pagination: build_pagination(poems_page, poem_params.page_size, page.total),
        }),
        poets: poets.map(|page| ListEnvelope {
            data: page.hits.into_iter().map(PoetResult::from).collect(),
            pagination: build_pagination(poets_page, poet_params.page_size, page.total),
        }),
    }))
}
