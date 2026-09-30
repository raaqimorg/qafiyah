#![expect(
    clippy::print_stdout,
    reason = "the query vectors are this command's output"
)]
#![expect(clippy::expect_used, reason = "built bodies are always serializable")]

use serde::Serialize;
use serde_json::{Value, json};

use qafiyah_api::constants::{POEMS_PER_PAGE, POETS_LIST_MAX_RESULT_WINDOW};
use qafiyah_api::es::query::{
    PoemSearchParams, PoetSearchParams, PoetSort, poem_search_body, poet_search_body,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PoemCase {
    q: &'static str,
    page: u32,
    poet_slugs: &'static [&'static str],
    era_slugs: &'static [&'static str],
    meter_slugs: &'static [&'static str],
    theme_slugs: &'static [&'static str],
    rhyme_slugs: &'static [&'static str],
    collection_slugs: &'static [&'static str],
    #[serde(skip_serializing_if = "Option::is_none")]
    exact: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PoetCase {
    q: &'static str,
    page: u32,
    era_slugs: &'static [&'static str],
    #[serde(skip_serializing_if = "Option::is_none")]
    page_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    highlight: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    window: Option<u32>,
}

const NO_SLUGS: PoemCase = PoemCase {
    q: "",
    page: 1,
    poet_slugs: &[],
    era_slugs: &[],
    meter_slugs: &[],
    theme_slugs: &[],
    rhyme_slugs: &[],
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
        "stopword-only query",
        PoemCase {
            q: "هذا",
            ..NO_SLUGS
        },
    ),
    ("browse, no text", PoemCase { ..NO_SLUGS }),
    (
        "browse, later page",
        PoemCase {
            page: 4,
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
        "text plus one filter",
        PoemCase {
            q: "دمع",
            page: 2,
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
    page: 1,
    era_slugs: &[],
    page_size: None,
    sort: None,
    highlight: None,
    exact: None,
    window: None,
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
        "the /poets list: sorted by poem count, no highlight, poem page size",
        PoetCase {
            page_size: Some(POEMS_PER_PAGE),
            sort: Some("poemsCount"),
            highlight: Some(false),
            window: Some(POETS_LIST_MAX_RESULT_WINDOW),
            ..BROWSE
        },
    ),
    (
        "the /poets list with a query and an era",
        PoetCase {
            q: "المتنبي",
            page: 3,
            era_slugs: &["abbasi"],
            page_size: Some(POEMS_PER_PAGE),
            sort: Some("poemsCount"),
            highlight: Some(false),
            window: Some(POETS_LIST_MAX_RESULT_WINDOW),
            ..BROWSE
        },
    ),
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
            page: 2,
            highlight: Some(false),
            ..BROWSE
        },
    ),
];

fn owned(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn poem_entry(note: &str, case: &PoemCase) -> Value {
    let body = poem_search_body(&PoemSearchParams {
        q: case.q.to_string(),
        page: case.page,
        poet_slugs: owned(case.poet_slugs),
        era_slugs: owned(case.era_slugs),
        meter_slugs: owned(case.meter_slugs),
        theme_slugs: owned(case.theme_slugs),
        rhyme_slugs: owned(case.rhyme_slugs),
        collection_slugs: owned(case.collection_slugs),
        exact: case.exact.unwrap_or_default(),
    });
    json!({ "note": note, "params": case, "body": body })
}

fn poet_entry(note: &str, case: &PoetCase) -> Value {
    let defaults = PoetSearchParams::default();
    let body = poet_search_body(&PoetSearchParams {
        q: case.q.to_string(),
        page: case.page,
        era_slugs: owned(case.era_slugs),
        page_size: case.page_size.unwrap_or(defaults.page_size),
        sort: match case.sort {
            Some("poemsCount") => PoetSort::PoemsCount,
            _ => PoetSort::Id,
        },
        highlight: case.highlight.unwrap_or(defaults.highlight),
        exact: case.exact.unwrap_or_default(),
        window: case.window.unwrap_or(defaults.window),
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
