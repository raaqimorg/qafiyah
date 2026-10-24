#![expect(
    clippy::print_stdout,
    reason = "the query vectors are this command's output"
)]
#![expect(clippy::expect_used, reason = "built bodies are always serializable")]

use serde::Serialize;
use serde_json::{Value, json};

use qafiyah_api::constants::SEARCH_POEMS_PER_PAGE;
use qafiyah_api::domain::paging::Cursor;
use qafiyah_api::domain::search::{
    PoemSearchParams, PoetListParams, PoetPosition, PoetSearchParams,
};
use qafiyah_api::es::query::{poem_search_body, poet_list_body, poet_search_body};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PoemCase {
    q: &'static str,
    offset: u32,
    poet_slugs: &'static [&'static str],
    era_slugs: &'static [&'static str],
    meter_slugs: &'static [&'static str],
    theme_slugs: &'static [&'static str],
    rhyme_slugs: &'static [&'static str],
    poem_type_slugs: &'static [&'static str],
    collection_slugs: &'static [&'static str],
    #[serde(skip_serializing_if = "Option::is_none")]
    exact: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PoetCase {
    q: &'static str,
    offset: u32,
    era_slugs: &'static [&'static str],
    #[serde(skip_serializing_if = "Option::is_none")]
    exact: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Position {
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<f64>,
    poems_count: i64,
    name_sort: &'static str,
    id: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListCase {
    q: &'static str,
    era_slugs: &'static [&'static str],
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<Position>,
}

const NO_SLUGS: PoemCase = PoemCase {
    q: "",
    offset: 0,
    poet_slugs: &[],
    era_slugs: &[],
    meter_slugs: &[],
    theme_slugs: &[],
    rhyme_slugs: &[],
    poem_type_slugs: &[],
    collection_slugs: &[],
    exact: None,
};

const POEMS: &[(&str, PoemCase)] = &[
    (
        "text query, default ranking",
        PoemCase {
            q: "حب",
            ..NO_SLUGS
        },
    ),
    (
        "a quoted line of three words or more",
        PoemCase {
            q: "قفا نبك من",
            ..NO_SLUGS
        },
    ),
    (
        "stopword-only query",
        PoemCase {
            q: "هذا",
            ..NO_SLUGS
        },
    ),
    (
        "query with a standalone hamza",
        PoemCase {
            q: "ماء",
            ..NO_SLUGS
        },
    ),
    ("browse, no text", PoemCase { ..NO_SLUGS }),
    (
        "browse, later page",
        PoemCase {
            offset: 60,
            ..NO_SLUGS
        },
    ),
    (
        "exact phrase",
        PoemCase {
            q: "يا رب",
            exact: Some(true),
            ..NO_SLUGS
        },
    ),
    (
        "an exact quoted line of three words or more",
        PoemCase {
            q: "قفا نبك من",
            exact: Some(true),
            ..NO_SLUGS
        },
    ),
    (
        "text plus one filter",
        PoemCase {
            q: "دمع",
            offset: 20,
            era_slugs: &["jahili"],
            ..NO_SLUGS
        },
    ),
    (
        "every filter at once, no text",
        PoemCase {
            poet_slugs: &["yoFB"],
            era_slugs: &["abbasi", "jahili"],
            meter_slugs: &["altawil"],
            theme_slugs: &["alnasib"],
            rhyme_slugs: &["meem"],
            poem_type_slugs: &["amudi"],
            collection_slugs: &["almuallaqat"],
            ..NO_SLUGS
        },
    ),
    (
        "exact with a filter",
        PoemCase {
            q: "صبر",
            meter_slugs: &["alkamil"],
            exact: Some(true),
            ..NO_SLUGS
        },
    ),
];

const BROWSE: PoetCase = PoetCase {
    q: "",
    offset: 0,
    era_slugs: &[],
    exact: None,
};

const POETS: &[(&str, PoetCase)] = &[
    (
        "text query, default ranking",
        PoetCase {
            q: "أحمد",
            ..BROWSE
        },
    ),
    ("browse by id, no text", PoetCase { ..BROWSE }),
    (
        "exact name",
        PoetCase {
            q: "المتنبي",
            exact: Some(true),
            ..BROWSE
        },
    ),
    (
        "search section, no highlight",
        PoetCase {
            q: "حب",
            offset: 20,
            ..BROWSE
        },
    ),
];

const FIRST: ListCase = ListCase {
    q: "",
    era_slugs: &[],
    after: None,
    before: None,
};

const POET_LIST: &[(&str, ListCase)] = &[
    ("the first page, most poems first", FIRST),
    (
        "the page after a poet",
        ListCase {
            after: Some(Position {
                score: None,
                poems_count: 40,
                name_sort: "زهير",
                id: 120,
            }),
            ..FIRST
        },
    ),
    (
        "a query and an era, the page before a poet",
        ListCase {
            q: "المتنبي",
            era_slugs: &["abbasi"],
            before: Some(Position {
                score: Some(12.5),
                poems_count: 3,
                name_sort: "متنبي",
                id: 9,
            }),
            ..FIRST
        },
    ),
];

fn owned(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn poem_entry(note: &str, case: &PoemCase) -> Value {
    let body = poem_search_body(&PoemSearchParams {
        q: case.q.to_string(),
        offset: case.offset,
        page_size: SEARCH_POEMS_PER_PAGE,
        poet_slugs: owned(case.poet_slugs),
        era_slugs: owned(case.era_slugs),
        meter_slugs: owned(case.meter_slugs),
        theme_slugs: owned(case.theme_slugs),
        rhyme_slugs: owned(case.rhyme_slugs),
        poem_type_slugs: owned(case.poem_type_slugs),
        collection_slugs: owned(case.collection_slugs),
        exact: case.exact.unwrap_or_default(),
    });
    json!({ "note": note, "params": case, "body": body })
}

fn poet_entry(note: &str, case: &PoetCase) -> Value {
    let body = poet_search_body(&PoetSearchParams {
        q: case.q.to_string(),
        offset: case.offset,
        era_slugs: owned(case.era_slugs),
        exact: case.exact.unwrap_or_default(),
        ..PoetSearchParams::default()
    });
    json!({ "note": note, "params": case, "body": body })
}

fn position(at: &Position) -> PoetPosition {
    PoetPosition {
        score: at.score,
        poems_count: at.poems_count,
        name_sort: at.name_sort.to_string(),
        id: at.id,
    }
}

fn poet_list_entry(note: &str, case: &ListCase) -> Value {
    let cursor = match (&case.after, &case.before) {
        (Some(at), _) => Some(Cursor::After(position(at))),
        (None, Some(at)) => Some(Cursor::Before(position(at))),
        (None, None) => None,
    };
    let body = poet_list_body(&PoetListParams {
        q: case.q.to_string(),
        era_slugs: owned(case.era_slugs),
        cursor,
        ..PoetListParams::default()
    });
    json!({ "note": note, "params": case, "body": body })
}

fn query_vectors() -> Value {
    json!({
        "poems": POEMS
            .iter()
            .map(|(note, case)| poem_entry(note, case))
            .collect::<Vec<_>>(),
        "poets": POETS
            .iter()
            .map(|(note, case)| poet_entry(note, case))
            .collect::<Vec<_>>(),
        "poetList": POET_LIST
            .iter()
            .map(|(note, case)| poet_list_entry(note, case))
            .collect::<Vec<_>>(),
    })
}

fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&query_vectors())
            .expect("built bodies are always serializable")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_committed_query_bodies_are_current() {
        const COMMITTED: &str = include_str!("../../generated/es/query.vectors.json");
        let committed: Value =
            serde_json::from_str(COMMITTED).expect("the committed query bodies are malformed");
        assert_eq!(
            committed,
            query_vectors(),
            "apps/api/generated/es/query.vectors.json is stale; run: bun run es:query:snapshot"
        );
    }
}
