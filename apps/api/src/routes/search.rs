use axum::Json;
use axum::extract::{Extension, State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::constants::{
    SEARCH_POEMS_MAX_PAGE, SEARCH_POEMS_PER_PAGE, SEARCH_POETS_MAX_PAGE, SEARCH_POETS_PER_PAGE,
};
use crate::contract::search::{PoemResult, PoetResult};
use crate::domain::search::{self, PoemSearchParams, PoetSearchParams};
use crate::envelope::{ListEnvelope, build_pagination};
use crate::error::{AppError, RouteProblem};
use crate::extract::SafeQuery;
use crate::log::LogHandle;
use crate::openapi::ListErrors;
use crate::params::{ExactFlag, Page, PoetSlugs, SearchText, SearchTypeParam, TermSlugs};
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

#[derive(Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(crate) struct SearchParams {
    /// Search query in Arabic, up to 100 characters. It is normalized (Unicode NFKC, whitespace collapsed and trimmed) and echoed back as `q`. A query of three words or more puts poems holding the words as a phrase above every other result, the oldest classical era first. When empty, the sections are browsed (see above).
    #[param(inline, example = "أمن أم أوفى")]
    q: Option<SearchText>,
    /// Sections to include: `poems`, `poets`, or both, which is the default. Repeat it to send both. Send `types=poems` alone to use any poem-only filter.
    #[param(example = json!(["poems"]))]
    types: Option<Vec<SearchTypeParam>>,
    /// Page of the poems section as a 1-based integer string, 20 results a page. Maximum 500: Elasticsearch stops paging after 10,000 results.
    #[param(inline, example = "1")]
    poems_page: Option<Page<{ SEARCH_POEMS_MAX_PAGE }>>,
    /// Page of the poets section as a 1-based integer string, 20 results a page. Maximum 500: Elasticsearch stops paging after 10,000 results.
    #[param(inline, example = "1")]
    poets_page: Option<Page<{ SEARCH_POETS_MAX_PAGE }>>,
    /// Narrow the poems section to these poets. The poets section ignores it. Repeatable. Values are `slug` from GET /poets.
    #[serde(default)]
    #[param(inline, example = json!(["PAKT"]))]
    poet_slugs: PoetSlugs,
    /// Narrow both sections to these eras. Repeatable. Values are `slug` from GET /eras.
    #[serde(default)]
    #[param(inline, example = json!(["jahili"]))]
    era_slugs: TermSlugs,
    /// Narrow the poems section to these meters. Poems only: needs `types=poems`. Repeatable. Values are `slug` from GET /meters.
    #[serde(default)]
    #[param(inline, example = json!(["altawil"]))]
    meter_slugs: TermSlugs,
    /// Narrow the poems section to these rhymes. Poems only: needs `types=poems`. Repeatable. Values are `slug` from GET /rhymes.
    #[serde(default)]
    #[param(inline, example = json!(["meem"]))]
    rhyme_slugs: TermSlugs,
    /// Narrow the poems section to these themes. Poems only: needs `types=poems`. Repeatable. Values are `slug` from GET /themes.
    #[serde(default)]
    #[param(inline, example = json!(["alhikma"]))]
    theme_slugs: TermSlugs,
    /// Narrow the poems section to these verse forms (amudi, hurr, and the rest). Poems only: needs `types=poems`. Repeatable. Values are `slug` from GET /poem-types.
    #[serde(default)]
    #[param(inline, example = json!(["amudi"]))]
    poem_type_slugs: TermSlugs,
    /// Narrow the poems section to these collections. Poems only: needs `types=poems`. Repeatable. Values are `slug` from GET /collections.
    #[serde(default)]
    #[param(inline, example = json!(["almuallaqat"]))]
    collection_slugs: TermSlugs,
    /// When true, match the literal phrase only (poem title or text, and poet name or nickname), with no stemming, fuzzy, or autocomplete expansion (Arabic letter normalization still applies, except that a standalone hamza in a poem phrase must match as typed). Applies to both result sets.
    #[param(example = "false")]
    exact: Option<ExactFlag>,
}

#[utoipa::path(
    get,
    path = "/search",
    tag = "search",
    operation_id = "search.search",
    description = "Full-text search over poems and poets, in two sections paged on their own. Poems match by title and verse text, poets by name and nickname. Without `q` the sections are browsed instead: poems newest first, with the classical eras first unless an era is chosen, and poets newest first, all narrowed by the filters. A poem found in several readings appears once, as its best-matching reading. Each section's `totalItems` stops at 10,000, so 10,000 means 10,000 or more, and the poems total counts poems, not readings. `relevance` is the raw search score, comparable only within one section. Repeat a filter to match any of its values. The meter, rhyme, theme, verse form, and collection filters apply to poems only and need `types=poems`: with the default `types`, which includes poets, a request using them is refused with 400.",
    params(SearchParams),
    responses(
        (status = 200, description = "The normalized query and the requested sections, each with its own results and pagination.", body = SearchResponse),
        ListErrors,
    ),
)]
pub(crate) async fn search(
    State(state): State<AppState>,
    Extension(log): Extension<LogHandle>,
    SafeQuery(params): SafeQuery<SearchParams>,
) -> Result<Json<SearchResponse>, AppError> {
    let q = params
        .q
        .map(|text| search::normalize_query(text.as_str()))
        .unwrap_or_default();
    let (want_poems, want_poets) = params.types.as_deref().map_or((true, true), |types| {
        (
            types.contains(&SearchTypeParam::Poems),
            types.contains(&SearchTypeParam::Poets),
        )
    });
    let poems_page = params.poems_page.map_or(1, Page::get);
    let poets_page = params.poets_page.map_or(1, Page::get);
    let exact = params.exact == Some(ExactFlag::True);

    let poem_only = [
        ("meterSlugs", params.meter_slugs.is_empty()),
        ("rhymeSlugs", params.rhyme_slugs.is_empty()),
        ("themeSlugs", params.theme_slugs.is_empty()),
        ("poemTypeSlugs", params.poem_type_slugs.is_empty()),
        ("collectionSlugs", params.collection_slugs.is_empty()),
    ];
    if want_poets && let Some((name, _)) = poem_only.iter().find(|(_, empty)| !empty) {
        return Err(RouteProblem::bad_request(&format!(
            "Invalid query parameter `{name}`: filters poems only, send `types=poems`"
        ))
        .into());
    }

    let era_slugs = params.era_slugs.into_strings();
    let poem_params = PoemSearchParams {
        q: q.clone(),
        page: poems_page,
        page_size: SEARCH_POEMS_PER_PAGE,
        poet_slugs: params.poet_slugs.into_strings(),
        era_slugs: era_slugs.clone(),
        meter_slugs: params.meter_slugs.into_strings(),
        theme_slugs: params.theme_slugs.into_strings(),
        rhyme_slugs: params.rhyme_slugs.into_strings(),
        poem_type_slugs: params.poem_type_slugs.into_strings(),
        collection_slugs: params.collection_slugs.into_strings(),
        exact,
    };
    let poet_params = PoetSearchParams {
        q: q.clone(),
        page: poets_page,
        page_size: SEARCH_POETS_PER_PAGE,
        era_slugs,
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
