use serde_json::{Map, Value, json};

use crate::constants::ES_MAX_RESULT_WINDOW;
use crate::domain::search::{PoemSearchParams, PoetSearchParams, PoetSort};

const RECALL_FLOOR: &str = "2<75%";

const CLASSICAL_ERA_SLUGS: [&str; 8] = [
    "jahili", "islami", "umawi", "abbasi", "andalusi", "fatimi", "ayyubi", "mamluki",
];
const CLASSICAL_ERA_WEIGHT: f64 = 1.1;
const ALTERNATE_READING_WEIGHT: f64 = 0.5;
const TYPED_HAMZA_WEIGHT: f64 = 1.5;
const STANDALONE_HAMZA: char = 'ء';
const VERBATIM_MIN_WORDS: usize = 3;
const VERBATIM_FLOOR: f64 = 100_000_000.0;
const VERBATIM_ERA_STEP: f64 = 1_000_000.0;
const VERBATIM_TIE_BREAKER: f64 = 0.01;
const NAME_FUZZINESS: &str = "AUTO:4,7";
const SIMILARITY_ONLY_REWRITE: &str = "top_terms_boost_50";
const TEXT_FIELDS: [&str; 2] = ["title", "content"];
const TYPED_FIELDS: [&str; 2] = ["title.hamza", "content.hamza"];

mod tier {
    pub(super) const SURFACE_EXACT: i64 = 32768;
    pub(super) const SURFACE_PHRASE: i64 = 4096;
    pub(super) const STEM_PHRASE: i64 = 512;
    pub(super) const SURFACE_ALL: i64 = 64;
    pub(super) const STEM_ALL: i64 = 8;
    pub(super) const STEM_SOME: i64 = 1;
}

mod poet_boost {
    pub(super) const EXACT: i64 = 12;
    pub(super) const PHRASE: i64 = 6;
    pub(super) const STEMMED: i64 = 3;
    pub(super) const PREFIX: i64 = 2;
    pub(super) const FUZZY: i64 = 1;
}

struct FieldSpec {
    name: &'static str,
    surface_weight: i64,
    supports_exact: bool,
}

const TITLE: FieldSpec = FieldSpec {
    name: "title",
    surface_weight: 4,
    supports_exact: true,
};

const CONTENT: FieldSpec = FieldSpec {
    name: "content",
    surface_weight: 1,
    supports_exact: false,
};

const POEM_FIELDS: [FieldSpec; 2] = [TITLE, CONTENT];

fn term_filters(facets: &[(&str, &[String])]) -> Vec<Value> {
    facets
        .iter()
        .filter(|(_, values)| !values.is_empty())
        .map(|(field, values)| json!({ "terms": { *field: values } }))
        .collect()
}

fn recall_gate(q: &str, fields: &[FieldSpec]) -> Value {
    let mut should: Vec<Value> = fields
        .iter()
        .map(|field| {
            json!({ "match": { format!("{}.stemmed", field.name): {
                "query": q,
                "minimum_should_match": RECALL_FLOOR,
            } } })
        })
        .collect();
    should.extend(
        fields
            .iter()
            .map(|field| json!({ "match": { field.name: { "query": q, "operator": "and" } } })),
    );
    json!({ "bool": { "should": should, "minimum_should_match": 1 } })
}

fn ranking_clauses(q: &str, fields: &[FieldSpec]) -> Vec<Value> {
    let mut should = Vec::new();
    for field in fields {
        let name = field.name;
        let weight = field.surface_weight;
        if field.supports_exact {
            should.push(json!({ "term": { format!("{name}.exact"): {
                "value": q,
                "boost": tier::SURFACE_EXACT.saturating_mul(weight),
            } } }));
        }
        should.push(
            json!({ "match_phrase": { name: { "query": q, "boost": tier::SURFACE_PHRASE.saturating_mul(weight) } } }),
        );
        should.push(json!({ "match_phrase": { format!("{name}.stemmed"): {
            "query": q,
            "boost": tier::STEM_PHRASE,
        } } }));
        should.push(json!({ "match": { name: {
            "query": q,
            "operator": "and",
            "boost": tier::SURFACE_ALL.saturating_mul(weight),
        } } }));
        should.push(json!({ "match": { format!("{name}.stemmed"): {
            "query": q,
            "operator": "and",
            "boost": tier::STEM_ALL,
        } } }));
        should.push(json!({ "match": { format!("{name}.stemmed"): {
            "query": q,
            "boost": tier::STEM_SOME,
        } } }));
    }
    should
}

fn highlight(
    number_of_fragments: u32,
    fragment_size: Option<u32>,
    field: &str,
    matched: &[&str],
) -> Value {
    let mut body = json!({
        "pre_tags": ["<mark>"],
        "post_tags": ["</mark>"],
        "number_of_fragments": number_of_fragments,
        "fields": { field: { "matched_fields": matched } },
    });
    if let (Some(size), Some(object)) = (fragment_size, body.as_object_mut()) {
        object.insert("fragment_size".into(), json!(size));
    }
    body
}

fn body(
    from: u32,
    size: u32,
    window: u32,
    query: Value,
    sort: Option<Value>,
    hl: Option<Value>,
) -> Value {
    let mut map = Map::new();
    map.insert("from".into(), json!(from));
    map.insert("size".into(), json!(size));
    map.insert("track_total_hits".into(), json!(window));
    map.insert("query".into(), query);
    if let Some(sort) = sort {
        map.insert("sort".into(), sort);
    }
    if let Some(hl) = hl {
        map.insert("highlight".into(), hl);
    }
    Value::Object(map)
}

pub fn poem_search_body(params: &PoemSearchParams) -> Value {
    let has_text = !params.q.is_empty();
    let filters = term_filters(&[
        ("poetSlug", &params.poet_slugs),
        ("eraSlug", &params.era_slugs),
        ("meterSlug", &params.meter_slugs),
        ("themeSlug", &params.theme_slugs),
        ("rhymeSlug", &params.rhyme_slugs),
        ("poemTypeSlug", &params.poem_type_slugs),
        ("collectionSlug", &params.collection_slugs),
    ]);

    let favor_classical = params.era_slugs.is_empty();
    let verbatim = params
        .q
        .split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .count()
        >= VERBATIM_MIN_WORDS;
    let query = if !has_text {
        let mut primaries = filters;
        primaries.push(json!({ "term": { "isPrimary": true } }));
        let browse = json!({ "bool": { "must": [{ "match_all": {} }], "filter": primaries } });
        if favor_classical {
            scored(browse, vec![classical_era_function()])
        } else {
            browse
        }
    } else if params.exact {
        let exact = phrase(&params.q, TYPED_FIELDS);
        if verbatim {
            json!({ "bool": {
                "must": [verbatim_first(exact.clone(), exact)],
                "filter": filters,
            } })
        } else {
            scored(
                json!({ "bool": { "must": [exact], "filter": filters } }),
                vec![alternate_reading_function()],
            )
        }
    } else {
        let mut gate = vec![recall_gate(&params.q, &POEM_FIELDS)];
        gate.extend(filters);
        let ranking = ranking_clauses(&params.q, &POEM_FIELDS);
        let should = if verbatim {
            vec![verbatim_first(
                phrase(&params.q, TEXT_FIELDS),
                json!({ "bool": { "should": ranking } }),
            )]
        } else {
            ranking
        };
        let ranked = json!({ "bool": {
            "filter": gate,
            "should": should,
        } });
        let mut functions = Vec::new();
        if favor_classical {
            functions.push(classical_era_function());
        }
        if params.q.contains(STANDALONE_HAMZA) {
            functions.push(typed_hamza_function(&params.q));
        }
        if !verbatim {
            functions.push(alternate_reading_function());
        }
        scored(ranked, functions)
    };

    let browse_sort = if favor_classical {
        json!([{ "_score": "desc" }, { "id": "desc" }])
    } else {
        json!([{ "id": "desc" }])
    };

    let mut request = body(
        params
            .page
            .saturating_sub(1)
            .saturating_mul(params.page_size),
        params.page_size,
        ES_MAX_RESULT_WINDOW,
        query,
        Some(if has_text {
            json!([{ "_score": "desc" }, { "id": "asc" }])
        } else {
            browse_sort
        }),
        has_text.then(|| poem_highlight(&params.q, params.exact)),
    );
    if let (true, Some(map)) = (has_text, request.as_object_mut()) {
        map.insert("collapse".into(), json!({ "field": "primaryId" }));
        map.insert(
            "aggs".into(),
            json!({ "poems": { "cardinality": {
                "field": "primaryId",
                "precision_threshold": ES_MAX_RESULT_WINDOW,
            } } }),
        );
    }
    request
}

fn poem_highlight(q: &str, exact: bool) -> Value {
    const TYPED: [&str; 3] = ["content", "content.stemmed", "content.hamza"];
    let (matched, marked): (&[&str], Value) = if exact {
        (
            &TYPED,
            json!({ "match_phrase": { "content.hamza": { "query": q } } }),
        )
    } else if q.contains(STANDALONE_HAMZA) {
        (
            &TYPED,
            json!({ "match": { "content.hamza": { "query": q } } }),
        )
    } else {
        (
            &["content", "content.stemmed"],
            json!({ "bool": { "should": ranking_clauses(q, &[CONTENT]) } }),
        )
    };
    let mut hl = highlight(0, None, "content", matched);
    if let Some(map) = hl.as_object_mut() {
        map.insert("highlight_query".into(), marked);
    }
    hl
}

fn phrase(q: &str, fields: [&str; 2]) -> Value {
    json!({ "multi_match": { "query": q, "type": "phrase", "fields": fields } })
}

fn verbatim_first(verbatim: Value, ladder: Value) -> Value {
    json!({ "dis_max": {
        "queries": [
            verbatim_by_era(verbatim),
            scored(ladder, vec![alternate_reading_function()]),
        ],
        "tie_breaker": VERBATIM_TIE_BREAKER,
    } })
}

fn verbatim_by_era(verbatim: Value) -> Value {
    let later_eras = VERBATIM_FLOOR + VERBATIM_ERA_STEP;
    let mut weight = later_eras;
    let mut functions: Vec<Value> = CLASSICAL_ERA_SLUGS
        .iter()
        .rev()
        .map(|slug| {
            weight += VERBATIM_ERA_STEP;
            json!({ "filter": { "term": { "eraSlug": slug } }, "weight": weight })
        })
        .collect();
    functions.reverse();
    functions.push(json!({ "filter": { "match_all": {} }, "weight": later_eras }));
    json!({ "function_score": {
        "query": { "constant_score": { "filter": verbatim } },
        "functions": functions,
        "score_mode": "first",
        "boost_mode": "replace",
    } })
}

fn classical_era_function() -> Value {
    json!({
        "filter": { "terms": { "eraSlug": CLASSICAL_ERA_SLUGS } },
        "weight": CLASSICAL_ERA_WEIGHT,
    })
}

fn alternate_reading_function() -> Value {
    json!({
        "filter": { "term": { "isPrimary": false } },
        "weight": ALTERNATE_READING_WEIGHT,
    })
}

fn typed_hamza_function(q: &str) -> Value {
    json!({ "filter": phrase(q, TYPED_FIELDS), "weight": TYPED_HAMZA_WEIGHT })
}

fn scored(query: Value, functions: Vec<Value>) -> Value {
    json!({ "function_score": {
        "query": query,
        "functions": functions,
        "score_mode": "multiply",
        "boost_mode": "multiply",
    } })
}

pub fn poet_search_body(params: &PoetSearchParams) -> Value {
    let has_text = !params.q.is_empty();
    let filters = term_filters(&[("eraSlug", &params.era_slugs)]);

    let query = if !has_text {
        json!({ "bool": { "must": [{ "match_all": {} }], "filter": filters } })
    } else if params.exact {
        json!({ "bool": {
            "must": [{ "multi_match": { "query": params.q, "type": "phrase", "fields": ["name", "nickname"] } }],
            "filter": filters,
        } })
    } else {
        let mut gate = filters;
        gate.push(json!({ "bool": {
            "should": [
                { "multi_match": {
                    "query": params.q,
                    "type": "cross_fields",
                    "operator": "and",
                    "fields": ["name.autocomplete", "name.stemmed", "nickname.autocomplete", "nickname.stemmed"],
                } },
                { "match": { "name": {
                    "query": params.q,
                    "operator": "and",
                    "fuzziness": NAME_FUZZINESS,
                    "prefix_length": 1,
                } } },
            ],
            "minimum_should_match": 1,
        } }));
        json!({ "bool": {
            "should": [
                { "term": { "name.exact": { "value": params.q, "boost": poet_boost::EXACT } } },
                { "match_phrase": { "name": { "query": params.q, "boost": poet_boost::PHRASE } } },
                { "match": { "name.autocomplete": { "query": params.q, "boost": poet_boost::PREFIX } } },
                { "match": { "name.stemmed": { "query": params.q, "boost": poet_boost::STEMMED } } },
                { "match": { "name": {
                    "query": params.q,
                    "fuzziness": NAME_FUZZINESS,
                    "fuzzy_rewrite": SIMILARITY_ONLY_REWRITE,
                    "boost": poet_boost::FUZZY,
                } } },
                { "match_phrase": { "nickname": { "query": params.q, "boost": poet_boost::PHRASE } } },
                { "match": { "nickname.autocomplete": { "query": params.q, "boost": poet_boost::PREFIX } } },
                { "match": { "nickname.stemmed": { "query": params.q, "boost": poet_boost::STEMMED } } },
            ],
            "minimum_should_match": 1,
            "filter": gate,
        } })
    };

    let browse_sort = match params.sort {
        PoetSort::PoemsCount => {
            json!([{ "poemsCount": "desc" }, { "nameSort": "asc" }, { "id": "asc" }])
        }
        PoetSort::Id => json!([{ "id": "desc" }]),
    };

    body(
        params
            .page
            .saturating_sub(1)
            .saturating_mul(params.page_size),
        params.page_size,
        params.window,
        query,
        Some(if has_text {
            json!([{ "_score": "desc" }, { "poemsCount": "desc" }, { "nameSort": "asc" }, { "id": "asc" }])
        } else {
            browse_sort
        }),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::SEARCH_POEMS_PER_PAGE;

    fn poems(q: &str, page: u32, exact: bool) -> PoemSearchParams {
        PoemSearchParams {
            q: q.into(),
            page,
            exact,
            ..PoemSearchParams::default()
        }
    }

    fn every_name_in(value: &Value, names: &mut std::collections::BTreeSet<String>) {
        match value {
            Value::Object(map) => {
                for (key, inner) in map {
                    names.insert(key.clone());
                    every_name_in(inner, names);
                }
            }
            Value::Array(items) => {
                for item in items {
                    every_name_in(item, names);
                }
            }
            Value::String(text) => {
                names.insert(text.clone());
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }

    fn searchable_or_sortable(properties: &Value) -> Vec<String> {
        let mut found = Vec::new();
        for (name, field) in properties.as_object().expect("properties") {
            let mut paths = vec![(name.clone(), field)];
            if let Some(subfields) = field["fields"].as_object() {
                paths.extend(
                    subfields
                        .iter()
                        .map(|(sub, spec)| (format!("{name}.{sub}"), spec)),
                );
            }
            for (path, spec) in paths {
                let searchable = spec["index"] != false;
                let sortable = spec["type"] != "text" && spec["doc_values"] != false;
                if searchable || sortable {
                    found.push(path);
                }
            }
        }
        found
    }

    #[test]
    fn every_field_the_indices_can_search_or_sort_is_one_a_search_reads() {
        let every_facet = |q: &str, era: bool| PoemSearchParams {
            q: q.into(),
            page: 1,
            page_size: SEARCH_POEMS_PER_PAGE,
            poet_slugs: vec!["yoFB".into()],
            era_slugs: if era { vec!["abbasi".into()] } else { vec![] },
            meter_slugs: vec!["altawil".into()],
            theme_slugs: vec!["alnasib".into()],
            rhyme_slugs: vec!["meem".into()],
            poem_type_slugs: vec!["amudi".into()],
            collection_slugs: vec!["almuallaqat".into()],
            exact: false,
        };
        let mut bodies = Vec::new();
        for q in ["حب", "قفا نبك من", "ماء"] {
            bodies.push(poem_search_body(&poems(q, 1, false)));
            bodies.push(poem_search_body(&poems(q, 1, true)));
        }
        for (q, era) in [("", false), ("", true), ("حب", false)] {
            bodies.push(poem_search_body(&every_facet(q, era)));
        }
        for (q, exact, sort) in [
            ("", false, PoetSort::Id),
            ("", false, PoetSort::PoemsCount),
            ("المتنبي", false, PoetSort::Id),
            ("المتنبي", true, PoetSort::Id),
        ] {
            bodies.push(poet_search_body(&PoetSearchParams {
                q: q.into(),
                exact,
                sort,
                era_slugs: vec!["abbasi".into()],
                ..PoetSearchParams::default()
            }));
        }
        let mut read = std::collections::BTreeSet::new();
        for body in &bodies {
            every_name_in(body, &mut read);
        }

        let schema = qafiyah_elasticsearch::load();
        let unread: Vec<String> = [("poems", &schema.poems), ("poets", &schema.poets)]
            .into_iter()
            .flat_map(|(index, definition)| {
                searchable_or_sortable(&definition["mappings"]["properties"])
                    .into_iter()
                    .filter(|path| !read.contains(path))
                    .map(move |path| format!("{index}: {path}"))
            })
            .collect();

        assert!(
            unread.is_empty(),
            "indexed or given doc values but read by no search: {unread:?}"
        );
    }

    #[test]
    fn the_last_page_offsets_to_the_edge_of_the_window() {
        let body = poem_search_body(&poems("x", 500, false));
        assert_eq!(body["from"], 9980);
        let poets = poet_search_body(&PoetSearchParams {
            page: 1666,
            page_size: 30,
            window: 50_000,
            ..PoetSearchParams::default()
        });
        assert_eq!(poets["from"], 49_950);
        assert_eq!(poets["track_total_hits"], 50_000);
    }
}
