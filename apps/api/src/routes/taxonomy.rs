use axum::Json;
use axum::extract::State;

use crate::contract::taxonomy::{CountedStats, PoemCountStats};
use crate::domain::taxonomy::{Counted, PoemCounted};
use crate::envelope::{ItemEnvelope, ListEnvelope, build_pagination};
use crate::error::{AppError, Resource};
use crate::extract::{SafePath, SafeQuery, invalid_path_slug};
use crate::openapi::{ListErrors, LookupErrors};
use crate::params::{NoParams, TransliteratedSlug};
use crate::state::AppState;

trait Kind: Copy {
    fn resource(self) -> Resource;
    fn log_field(self) -> &'static str;
}

impl Kind for Counted {
    fn resource(self) -> Resource {
        match self {
            Counted::Meters => Resource::Meter,
            Counted::Rhymes => Resource::Rhyme,
            Counted::Eras => Resource::Era,
            Counted::PoemTypes => Resource::PoemType,
        }
    }

    fn log_field(self) -> &'static str {
        match self {
            Counted::Meters => "meter",
            Counted::Rhymes => "rhyme",
            Counted::Eras => "era",
            Counted::PoemTypes => "poem_type",
        }
    }
}

impl Kind for PoemCounted {
    fn resource(self) -> Resource {
        match self {
            PoemCounted::Themes => Resource::Theme,
            PoemCounted::Collections => Resource::Collection,
        }
    }

    fn log_field(self) -> &'static str {
        match self {
            PoemCounted::Themes => "theme",
            PoemCounted::Collections => "collection",
        }
    }
}

fn listed<T>(rows: Vec<T>) -> Json<ListEnvelope<T>> {
    let count = u32::try_from(rows.len()).unwrap_or(u32::MAX);
    crate::log::record_results(u64::from(count));
    Json(ListEnvelope {
        data: rows,
        pagination: build_pagination(1, count.max(1), count),
    })
}

async fn list_counted_kind(
    State(state): State<AppState>,
    kind: Counted,
) -> Result<Json<ListEnvelope<CountedStats>>, AppError> {
    Ok(listed(
        state
            .taxonomy
            .list_counted(kind)
            .await?
            .into_iter()
            .map(CountedStats::from)
            .collect(),
    ))
}

async fn get_counted_kind(
    State(state): State<AppState>,
    SafePath(raw): SafePath<String>,
    kind: Counted,
) -> Result<Json<ItemEnvelope<CountedStats>>, AppError> {
    let slug = TransliteratedSlug::parse(&raw).map_err(|reason| invalid_path_slug(&reason))?;
    let slug = slug.as_str();
    tracing::Span::current().record(kind.log_field(), slug);
    Ok(Json(ItemEnvelope {
        data: state
            .taxonomy
            .get_counted(kind, slug)
            .await?
            .map(CountedStats::from)
            .ok_or(AppError::NotFound(kind.resource()))?,
    }))
}

async fn list_poem_counted_kind(
    State(state): State<AppState>,
    kind: PoemCounted,
) -> Result<Json<ListEnvelope<PoemCountStats>>, AppError> {
    Ok(listed(
        state
            .taxonomy
            .list_by_poem_count(kind)
            .await?
            .into_iter()
            .map(PoemCountStats::from)
            .collect(),
    ))
}

async fn get_poem_counted_kind(
    State(state): State<AppState>,
    SafePath(raw): SafePath<String>,
    kind: PoemCounted,
) -> Result<Json<ItemEnvelope<PoemCountStats>>, AppError> {
    let slug = TransliteratedSlug::parse(&raw).map_err(|reason| invalid_path_slug(&reason))?;
    let slug = slug.as_str();
    tracing::Span::current().record(kind.log_field(), slug);
    Ok(Json(ItemEnvelope {
        data: state
            .taxonomy
            .get_by_poem_count(kind, slug)
            .await?
            .map(PoemCountStats::from)
            .ok_or(AppError::NotFound(kind.resource()))?,
    }))
}

#[utoipa::path(
    get,
    path = "/meters",
    tag = "meters",
    operation_id = "meters.list",
    description = "Every prosodic meter (al-buhur) with its poem and poet counts, in name order. Counts include primary readings only.",
    responses(
        (status = 200, description = "All meters.", body = ListEnvelope<CountedStats>),
        ListErrors,
    ),
)]
pub(crate) async fn list_meters(
    state: State<AppState>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ListEnvelope<CountedStats>>, AppError> {
    list_counted_kind(state, Counted::Meters).await
}

#[utoipa::path(
    get,
    path = "/meters/{slug}",
    tag = "meters",
    operation_id = "meters.get",
    description = "A meter by slug, with its poem and poet counts.",
    params(
        ("slug" = String, Path, description = "The term's `slug`, from its list.", pattern = "^[a-z][a-z-]*$", example = "altawil"),
    ),
    responses(
        (status = 200, description = "The requested meter.", body = ItemEnvelope<CountedStats>),
        LookupErrors,
    ),
)]
pub(crate) async fn get_meter(
    state: State<AppState>,
    path: SafePath<String>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ItemEnvelope<CountedStats>>, AppError> {
    get_counted_kind(state, path, Counted::Meters).await
}

#[utoipa::path(
    get,
    path = "/rhymes",
    tag = "rhymes",
    operation_id = "rhymes.list",
    description = "Every rhyme letter (al-qawafi) with its poem and poet counts, in a fixed letter order with the alif, hamza, and ta marbuta forms first. Counts include primary readings only.",
    responses(
        (status = 200, description = "All rhymes.", body = ListEnvelope<CountedStats>),
        ListErrors,
    ),
)]
pub(crate) async fn list_rhymes(
    state: State<AppState>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ListEnvelope<CountedStats>>, AppError> {
    list_counted_kind(state, Counted::Rhymes).await
}

#[utoipa::path(
    get,
    path = "/rhymes/{slug}",
    tag = "rhymes",
    operation_id = "rhymes.get",
    description = "A rhyme letter by slug, with its poem and poet counts.",
    params(
        ("slug" = String, Path, description = "The term's `slug`, from its list.", pattern = "^[a-z][a-z-]*$", example = "meem"),
    ),
    responses(
        (status = 200, description = "The requested rhyme.", body = ItemEnvelope<CountedStats>),
        LookupErrors,
    ),
)]
pub(crate) async fn get_rhyme(
    state: State<AppState>,
    path: SafePath<String>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ItemEnvelope<CountedStats>>, AppError> {
    get_counted_kind(state, path, Counted::Rhymes).await
}

#[utoipa::path(
    get,
    path = "/eras",
    tag = "eras",
    operation_id = "eras.list",
    description = "Every literary era (al-usur al-adabiyya) with its poem and poet counts, oldest first, with the unknown era last. Counts include primary readings only.",
    responses(
        (status = 200, description = "All eras.", body = ListEnvelope<CountedStats>),
        ListErrors,
    ),
)]
pub(crate) async fn list_eras(
    state: State<AppState>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ListEnvelope<CountedStats>>, AppError> {
    list_counted_kind(state, Counted::Eras).await
}

#[utoipa::path(
    get,
    path = "/eras/{slug}",
    tag = "eras",
    operation_id = "eras.get",
    description = "An era by slug, with its poem and poet counts.",
    params(
        ("slug" = String, Path, description = "The term's `slug`, from its list.", pattern = "^[a-z][a-z-]*$", example = "jahili"),
    ),
    responses(
        (status = 200, description = "The requested era.", body = ItemEnvelope<CountedStats>),
        LookupErrors,
    ),
)]
pub(crate) async fn get_era(
    state: State<AppState>,
    path: SafePath<String>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ItemEnvelope<CountedStats>>, AppError> {
    get_counted_kind(state, path, Counted::Eras).await
}

#[utoipa::path(
    get,
    path = "/themes",
    tag = "themes",
    operation_id = "themes.list",
    description = "Every theme (al-aghrad) with its poem count. Counts include primary readings only.",
    responses(
        (status = 200, description = "All themes.", body = ListEnvelope<PoemCountStats>),
        ListErrors,
    ),
)]
pub(crate) async fn list_themes(
    state: State<AppState>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ListEnvelope<PoemCountStats>>, AppError> {
    list_poem_counted_kind(state, PoemCounted::Themes).await
}

#[utoipa::path(
    get,
    path = "/themes/{slug}",
    tag = "themes",
    operation_id = "themes.get",
    description = "A theme by slug, with its poem count.",
    params(
        ("slug" = String, Path, description = "The term's `slug`, from its list.", pattern = "^[a-z][a-z-]*$", example = "alhikma"),
    ),
    responses(
        (status = 200, description = "The requested theme.", body = ItemEnvelope<PoemCountStats>),
        LookupErrors,
    ),
)]
pub(crate) async fn get_theme(
    state: State<AppState>,
    path: SafePath<String>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ItemEnvelope<PoemCountStats>>, AppError> {
    get_poem_counted_kind(state, path, PoemCounted::Themes).await
}

#[utoipa::path(
    get,
    path = "/collections",
    tag = "collections",
    operation_id = "collections.list",
    description = "Every curated collection (al-dawawin) with its poem count. Counts include primary readings only.",
    responses(
        (status = 200, description = "All collections.", body = ListEnvelope<PoemCountStats>),
        ListErrors,
    ),
)]
pub(crate) async fn list_collections(
    state: State<AppState>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ListEnvelope<PoemCountStats>>, AppError> {
    list_poem_counted_kind(state, PoemCounted::Collections).await
}

#[utoipa::path(
    get,
    path = "/collections/{slug}",
    tag = "collections",
    operation_id = "collections.get",
    description = "A collection by slug, with its poem count.",
    params(
        ("slug" = String, Path, description = "The term's `slug`, from its list.", pattern = "^[a-z][a-z-]*$", example = "almuallaqat"),
    ),
    responses(
        (status = 200, description = "The requested collection.", body = ItemEnvelope<PoemCountStats>),
        LookupErrors,
    ),
)]
pub(crate) async fn get_collection(
    state: State<AppState>,
    path: SafePath<String>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ItemEnvelope<PoemCountStats>>, AppError> {
    get_poem_counted_kind(state, path, PoemCounted::Collections).await
}

#[utoipa::path(
    get,
    path = "/poem-types",
    tag = "poem-types",
    operation_id = "poemTypes.list",
    description = "Every verse form (anwa' al-qasida) with its poem and poet counts, most poems first. Counts include primary readings only.",
    responses(
        (status = 200, description = "All verse forms.", body = ListEnvelope<CountedStats>),
        ListErrors,
    ),
)]
pub(crate) async fn list_poem_types(
    state: State<AppState>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ListEnvelope<CountedStats>>, AppError> {
    list_counted_kind(state, Counted::PoemTypes).await
}

#[utoipa::path(
    get,
    path = "/poem-types/{slug}",
    tag = "poem-types",
    operation_id = "poemTypes.get",
    description = "A verse form by slug, with its poem and poet counts.",
    params(
        ("slug" = String, Path, description = "The term's `slug`, from its list.", pattern = "^[a-z][a-z-]*$", example = "amudi"),
    ),
    responses(
        (status = 200, description = "The requested verse form.", body = ItemEnvelope<CountedStats>),
        LookupErrors,
    ),
)]
pub(crate) async fn get_poem_type(
    state: State<AppState>,
    path: SafePath<String>,
    _: SafeQuery<NoParams>,
) -> Result<Json<ItemEnvelope<CountedStats>>, AppError> {
    get_counted_kind(state, path, Counted::PoemTypes).await
}
