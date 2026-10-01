use serde_json::{Map, Value, json};

use crate::constants::{ES_MAX_RESULT_WINDOW, SEARCH_POEMS_PER_PAGE, SEARCH_POETS_PER_PAGE};

const RECALL_FLOOR: &str = "1<75%";

const CLASSICAL_ERA_SLUGS: [&str; 8] = [
    "jahili", "islami", "umawi", "abbasi", "andalusi", "fatimi", "ayyubi", "mamluki",
];
const CLASSICAL_ERA_WEIGHT: f64 = 1.1;
const TYPED_HAMZA_WEIGHT: f64 = 1.5;
const STANDALONE_HAMZA: char = 'ء';

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
    weight: i64,
    supports_exact: bool,
    supports_autocomplete: bool,
}

const POEM_FIELDS: [FieldSpec; 2] = [
    FieldSpec {
        name: "title",
        weight: 4,
        supports_exact: true,
        supports_autocomplete: false,
    },
    FieldSpec {
        name: "content",
        weight: 1,
        supports_exact: false,
        supports_autocomplete: false,
    },
];

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
    should.extend(
        fields
            .iter()
            .filter(|field| field.supports_autocomplete)
            .map(|field| {
                json!({ "match": { format!("{}.autocomplete", field.name): { "query": q } } })
            }),
    );
    json!({ "bool": { "should": should, "minimum_should_match": 1 } })
}

fn ranking_clauses(q: &str, fields: &[FieldSpec]) -> Vec<Value> {
    let mut should = Vec::new();
    for field in fields {
        let name = field.name;
        if field.supports_exact {
            should.push(json!({ "term": { format!("{name}.exact"): {
                "value": q,
                "boost": tier::SURFACE_EXACT.saturating_mul(field.weight),
            } } }));
        }
        should.push(
            json!({ "match_phrase": { name: { "query": q, "boost": tier::SURFACE_PHRASE.saturating_mul(field.weight) } } }),
        );
        should.push(json!({ "match_phrase": { format!("{name}.stemmed"): {
            "query": q,
            "boost": tier::STEM_PHRASE.saturating_mul(field.weight),
        } } }));
        should.push(json!({ "match": { name: {
            "query": q,
            "operator": "and",
            "boost": tier::SURFACE_ALL.saturating_mul(field.weight),
        } } }));
        should.push(json!({ "match": { format!("{name}.stemmed"): {
            "query": q,
            "operator": "and",
            "boost": tier::STEM_ALL.saturating_mul(field.weight),
        } } }));
        should.push(json!({ "match": { format!("{name}.stemmed"): {
            "query": q,
            "boost": tier::STEM_SOME * field.weight,
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

#[derive(Default)]
pub struct PoemSearchParams {
    pub q: String,
    pub page: u32,
    pub poet_slugs: Vec<String>,
    pub era_slugs: Vec<String>,
    pub meter_slugs: Vec<String>,
    pub theme_slugs: Vec<String>,
    pub rhyme_slugs: Vec<String>,
    pub collection_slugs: Vec<String>,
    pub exact: bool,
}

pub fn poem_search_body(params: &PoemSearchParams) -> Value {
    let has_text = !params.q.is_empty();
    let filters = term_filters(&[
        ("poetSlug", &params.poet_slugs),
        ("eraSlug", &params.era_slugs),
        ("meterSlug", &params.meter_slugs),
        ("themeSlug", &params.theme_slugs),
        ("rhymeSlug", &params.rhyme_slugs),
        ("collectionSlug", &params.collection_slugs),
    ]);

    let alternates = json!([{ "term": { "isPrimary": false } }]);
    let favor_classical = params.era_slugs.is_empty();
    let query = if !has_text {
        let browse = json!({ "bool": {
            "must": [{ "match_all": {} }],
            "filter": filters,
            "must_not": alternates,
        } });
        if favor_classical {
            scored(browse, vec![classical_era_function()])
        } else {
            browse
        }
    } else if params.exact {
        json!({ "bool": {
            "must": [{ "match_phrase": { "content": { "query": params.q } } }],
            "filter": filters,
            "must_not": alternates,
        } })
    } else {
        let mut gate = vec![recall_gate(&params.q, &POEM_FIELDS)];
        gate.extend(filters);
        let ranked = json!({ "bool": {
            "filter": gate,
            "should": ranking_clauses(&params.q, &POEM_FIELDS),
            "must_not": alternates,
        } });
        let mut functions = Vec::new();
        if favor_classical {
            functions.push(classical_era_function());
        }
        if params.q.contains(STANDALONE_HAMZA) {
            functions.push(typed_hamza_function(&params.q));
        }
        if functions.is_empty() {
            ranked
        } else {
            scored(ranked, functions)
        }
    };

    let browse_sort = if favor_classical {
        json!([{ "_score": "desc" }, { "id": "desc" }])
    } else {
        json!([{ "id": "desc" }])
    };

    body(
        params
            .page
            .saturating_sub(1)
            .saturating_mul(SEARCH_POEMS_PER_PAGE),
        SEARCH_POEMS_PER_PAGE,
        ES_MAX_RESULT_WINDOW,
        query,
        Some(if has_text {
            json!([{ "_score": "desc" }, { "id": "asc" }])
        } else {
            browse_sort
        }),
        has_text.then(|| poem_highlight(&params.q)),
    )
}

fn poem_highlight(q: &str) -> Value {
    if !q.contains(STANDALONE_HAMZA) {
        return highlight(0, None, "content", &["content", "content.stemmed"]);
    }
    let mut hl = highlight(
        0,
        None,
        "content",
        &["content", "content.stemmed", "content.hamza"],
    );
    if let Some(map) = hl.as_object_mut() {
        map.insert(
            "highlight_query".into(),
            json!({ "match": { "content.hamza": { "query": q } } }),
        );
    }
    hl
}

fn classical_era_function() -> Value {
    json!({
        "filter": { "terms": { "eraSlug": CLASSICAL_ERA_SLUGS } },
        "weight": CLASSICAL_ERA_WEIGHT,
    })
}

fn typed_hamza_function(q: &str) -> Value {
    json!({
        "filter": { "multi_match": {
            "query": q,
            "type": "phrase",
            "fields": ["title.hamza", "content.hamza"],
        } },
        "weight": TYPED_HAMZA_WEIGHT,
    })
}

fn scored(query: Value, functions: Vec<Value>) -> Value {
    json!({ "function_score": {
        "query": query,
        "functions": functions,
        "score_mode": "multiply",
        "boost_mode": "multiply",
    } })
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PoetSort {
    Id,
    PoemsCount,
}

pub struct PoetSearchParams {
    pub q: String,
    pub page: u32,
    pub era_slugs: Vec<String>,
    pub page_size: u32,
    pub sort: PoetSort,
    pub highlight: bool,
    pub exact: bool,
    pub window: u32,
}

impl Default for PoetSearchParams {
    fn default() -> Self {
        Self {
            q: String::new(),
            page: 1,
            era_slugs: Vec::new(),
            page_size: SEARCH_POETS_PER_PAGE,
            sort: PoetSort::Id,
            highlight: true,
            exact: false,
            window: ES_MAX_RESULT_WINDOW,
        }
    }
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
        gate.push(json!({ "multi_match": {
            "query": params.q,
            "type": "cross_fields",
            "operator": "and",
            "fields": ["name.autocomplete", "name.stemmed", "nickname.autocomplete", "nickname.stemmed"],
        } }));
        json!({ "bool": {
            "should": [
                { "term": { "name.exact": { "value": params.q, "boost": poet_boost::EXACT } } },
                { "match_phrase": { "name": { "query": params.q, "boost": poet_boost::PHRASE } } },
                { "match": { "name.autocomplete": { "query": params.q, "boost": poet_boost::PREFIX } } },
                { "match": { "name.stemmed": { "query": params.q, "boost": poet_boost::STEMMED } } },
                { "match": { "name": { "query": params.q, "fuzziness": "AUTO", "boost": poet_boost::FUZZY } } },
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
        params
            .highlight
            .then(|| highlight(1, Some(200), "name", &["name", "name.autocomplete"])),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poems(q: &str, page: u32, exact: bool) -> PoemSearchParams {
        PoemSearchParams {
            q: q.into(),
            page,
            exact,
            ..PoemSearchParams::default()
        }
    }

    #[test]
    fn an_empty_poem_query_with_an_era_filter_browses_by_id_without_highlighting() {
        let body = poem_search_body(&PoemSearchParams {
            page: 1,
            era_slugs: vec!["hadith".into()],
            ..PoemSearchParams::default()
        });
        assert_eq!(body["from"], 0);
        assert_eq!(body["size"], SEARCH_POEMS_PER_PAGE);
        assert_eq!(body["track_total_hits"], ES_MAX_RESULT_WINDOW);
        assert_eq!(body["sort"], json!([{ "id": "desc" }]));
        assert!(body.get("highlight").is_none());
        assert_eq!(body["query"]["bool"]["must"], json!([{ "match_all": {} }]));
        assert_eq!(
            body["query"]["bool"]["filter"],
            json!([{ "terms": { "eraSlug": ["hadith"] } }])
        );
    }

    #[test]
    fn an_empty_poem_query_with_no_era_filter_lists_classical_poems_first_then_by_id() {
        let body = poem_search_body(&PoemSearchParams {
            page: 1,
            meter_slugs: vec!["altawil".into()],
            ..PoemSearchParams::default()
        });
        assert_eq!(
            body["sort"],
            json!([{ "_score": "desc" }, { "id": "desc" }])
        );
        assert!(body.get("highlight").is_none());
        let scored = &body["query"]["function_score"];
        assert_eq!(scored["boost_mode"], "multiply");
        assert_eq!(
            scored["functions"],
            json!([{
                "filter": { "terms": { "eraSlug": [
                    "jahili", "islami", "umawi", "abbasi", "andalusi", "fatimi", "ayyubi", "mamluki",
                ] } },
                "weight": 1.1,
            }])
        );
        assert_eq!(
            scored["query"],
            json!({ "bool": {
                "must": [{ "match_all": {} }],
                "filter": [{ "terms": { "meterSlug": ["altawil"] } }],
                "must_not": [{ "term": { "isPrimary": false } }],
            } })
        );
    }

    #[test]
    fn an_exact_poem_query_is_a_single_phrase_on_content() {
        let body = poem_search_body(&poems("يا رب", 1, true));
        assert_eq!(
            body["query"]["bool"]["must"],
            json!([{ "match_phrase": { "content": { "query": "يا رب" } } }])
        );
        assert!(body["query"]["bool"].get("should").is_none());
        assert_eq!(body["highlight"]["number_of_fragments"], 0);
    }

    #[test]
    fn a_ranked_poem_query_gates_recall_and_ranks_in_should() {
        let body = poem_search_body(&poems("حب", 3, false));
        assert_eq!(body["from"], 40);
        let ranked = &body["query"]["function_score"]["query"]["bool"];
        let filter = ranked["filter"].as_array().expect("filters");
        assert_eq!(filter[0]["bool"]["minimum_should_match"], 1);
        assert_eq!(
            filter[0]["bool"]["should"][0]["match"]["title.stemmed"]["minimum_should_match"],
            RECALL_FLOOR
        );
        let should = ranked["should"].as_array().expect("ranking");
        assert_eq!(should.len(), 11, "six tiers for title, five for content");
        assert_eq!(
            should[0]["term"]["title.exact"]["boost"],
            tier::SURFACE_EXACT * 4
        );
        assert_eq!(
            should[10]["match"]["content.stemmed"]["boost"],
            tier::STEM_SOME
        );
    }

    #[test]
    fn the_recall_gate_also_matches_every_term_on_the_surface_fields() {
        let body = poem_search_body(&poems("هذا", 1, false));
        let gate = &body["query"]["function_score"]["query"]["bool"]["filter"][0]["bool"]["should"];
        for field in ["title", "content"] {
            assert!(
                gate.as_array().expect("gate").iter().any(|clause| clause
                    == &json!({ "match": { field: { "query": "هذا", "operator": "and" } } })),
                "{field} has no surface clause in the recall gate"
            );
        }
    }

    #[test]
    fn a_ranked_poem_query_multiplies_the_score_of_classical_era_poems() {
        let body = poem_search_body(&poems("حب", 1, false));
        let scored = &body["query"]["function_score"];
        assert_eq!(scored["boost_mode"], "multiply");
        assert_eq!(
            scored["functions"],
            json!([{
                "filter": { "terms": { "eraSlug": [
                    "jahili", "islami", "umawi", "abbasi", "andalusi", "fatimi", "ayyubi", "mamluki",
                ] } },
                "weight": 1.1,
            }])
        );
    }

    #[test]
    fn every_poem_query_leaves_alternate_readings_out() {
        let with_era = |q: &str, exact: bool| PoemSearchParams {
            q: q.into(),
            page: 1,
            exact,
            era_slugs: vec!["abbasi".into()],
            ..PoemSearchParams::default()
        };
        let alternates_out = json!([{ "term": { "isPrimary": false } }]);
        for body in [
            poem_search_body(&poems("حب", 1, false)),
            poem_search_body(&poems("حب", 1, true)),
            poem_search_body(&poems("", 1, false)),
            poem_search_body(&with_era("حب", false)),
            poem_search_body(&with_era("حب", true)),
            poem_search_body(&with_era("", false)),
        ] {
            let query = &body["query"];
            let bool_query = if query["function_score"].is_object() {
                &query["function_score"]["query"]["bool"]
            } else {
                &query["bool"]
            };
            assert_eq!(bool_query["must_not"], alternates_out, "{query}");
        }
    }

    #[test]
    fn a_ranked_poem_query_with_an_era_filter_is_not_boosted() {
        let body = poem_search_body(&PoemSearchParams {
            q: "حب".into(),
            page: 1,
            era_slugs: vec!["hadith".into()],
            ..PoemSearchParams::default()
        });
        assert!(body["query"].get("function_score").is_none());
        let filter = body["query"]["bool"]["filter"].as_array().expect("filters");
        assert_eq!(filter[1], json!({ "terms": { "eraSlug": ["hadith"] } }));
        assert_eq!(
            body["query"]["bool"]["should"]
                .as_array()
                .expect("ranking")
                .len(),
            11
        );
    }

    fn typed_hamza(q: &str) -> Value {
        json!({
            "filter": { "multi_match": {
                "query": q,
                "type": "phrase",
                "fields": ["title.hamza", "content.hamza"],
            } },
            "weight": 1.5,
        })
    }

    #[test]
    fn a_ranked_poem_query_with_a_standalone_hamza_favors_poems_that_spell_it_that_way() {
        let body = poem_search_body(&poems("ماء", 1, false));
        let scored = &body["query"]["function_score"];
        let functions = scored["functions"].as_array().expect("functions");
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[1], typed_hamza("ماء"));
        assert_eq!(scored["score_mode"], "multiply");
    }

    #[test]
    fn the_hamza_preference_applies_with_an_era_filter_too() {
        let body = poem_search_body(&PoemSearchParams {
            q: "السماء".into(),
            page: 1,
            era_slugs: vec!["abbasi".into()],
            ..PoemSearchParams::default()
        });
        assert_eq!(
            body["query"]["function_score"]["functions"],
            json!([typed_hamza("السماء")])
        );
    }

    #[test]
    fn a_query_with_a_standalone_hamza_highlights_only_the_word_as_typed() {
        let body = poem_search_body(&poems("ماء", 1, false));
        assert_eq!(
            body["highlight"]["highlight_query"],
            json!({ "match": { "content.hamza": { "query": "ماء" } } })
        );
        assert_eq!(
            body["highlight"]["fields"]["content"]["matched_fields"],
            json!(["content", "content.stemmed", "content.hamza"])
        );
        let plain = poem_search_body(&poems("حب", 1, false));
        assert!(plain["highlight"].get("highlight_query").is_none());
        assert_eq!(
            plain["highlight"]["fields"]["content"]["matched_fields"],
            json!(["content", "content.stemmed"])
        );
    }

    #[test]
    fn a_query_without_a_standalone_hamza_gets_no_hamza_preference() {
        for q in ["حب", "أمي", "مسؤول", "شئ"] {
            let body = poem_search_body(&poems(q, 1, false));
            let functions = body["query"]["function_score"]["functions"]
                .as_array()
                .expect("functions");
            assert_eq!(functions.len(), 1, "{q}");
        }
    }

    #[test]
    fn every_non_empty_facet_becomes_a_terms_filter_in_declaration_order() {
        let params = PoemSearchParams {
            q: "x".into(),
            page: 1,
            poet_slugs: vec!["yoFB".into()],
            era_slugs: vec![],
            meter_slugs: vec!["altawil".into()],
            theme_slugs: vec![],
            rhyme_slugs: vec!["meem".into()],
            collection_slugs: vec!["almuallaqat".into()],
            exact: false,
        };
        let body = poem_search_body(&params);
        let filter = body["query"]["function_score"]["query"]["bool"]["filter"]
            .as_array()
            .expect("filters");
        let fields: Vec<&str> = filter[1..]
            .iter()
            .map(|f| {
                f["terms"]
                    .as_object()
                    .expect("terms")
                    .keys()
                    .next()
                    .expect("field")
                    .as_str()
            })
            .collect();
        assert_eq!(
            fields,
            ["poetSlug", "meterSlug", "rhymeSlug", "collectionSlug"]
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

    #[test]
    fn poets_browse_by_count_then_name_then_id_and_search_by_the_flat_ladder() {
        let browse = poet_search_body(&PoetSearchParams {
            sort: PoetSort::PoemsCount,
            highlight: false,
            ..PoetSearchParams::default()
        });
        assert_eq!(
            browse["sort"],
            json!([{ "poemsCount": "desc" }, { "nameSort": "asc" }, { "id": "asc" }])
        );
        assert!(browse.get("highlight").is_none());
        let by_id = poet_search_body(&PoetSearchParams::default());
        assert_eq!(by_id["sort"], json!([{ "id": "desc" }]));
        assert_eq!(by_id["highlight"]["fragment_size"], 200);
        let ranked = poet_search_body(&PoetSearchParams {
            q: "المتنبي".into(),
            ..PoetSearchParams::default()
        });
        let should = ranked["query"]["bool"]["should"]
            .as_array()
            .expect("ladder");
        assert_eq!(
            should[0],
            json!({ "term": { "name.exact": { "value": "المتنبي", "boost": 12 } } })
        );
        assert_eq!(
            should[1],
            json!({ "match_phrase": { "name": { "query": "المتنبي", "boost": 6 } } })
        );
        assert_eq!(
            should[2],
            json!({ "match": { "name.autocomplete": { "query": "المتنبي", "boost": 2 } } })
        );
        assert_eq!(
            should[3],
            json!({ "match": { "name.stemmed": { "query": "المتنبي", "boost": 3 } } })
        );
        assert_eq!(
            should[4],
            json!({ "match": { "name": { "query": "المتنبي", "fuzziness": "AUTO", "boost": 1 } } })
        );
        assert_eq!(
            should[5],
            json!({ "match_phrase": { "nickname": { "query": "المتنبي", "boost": 6 } } })
        );
        assert_eq!(
            should[6],
            json!({ "match": { "nickname.autocomplete": { "query": "المتنبي", "boost": 2 } } })
        );
        assert_eq!(
            should[7],
            json!({ "match": { "nickname.stemmed": { "query": "المتنبي", "boost": 3 } } })
        );
        assert_eq!(ranked["query"]["bool"]["minimum_should_match"], 1);
        assert_eq!(
            ranked["sort"],
            json!([{ "_score": "desc" }, { "poemsCount": "desc" }, { "nameSort": "asc" }, { "id": "asc" }])
        );
        let exact = poet_search_body(&PoetSearchParams {
            q: "x".into(),
            exact: true,
            era_slugs: vec!["abbasi".into()],
            ..PoetSearchParams::default()
        });
        assert_eq!(
            exact["query"]["bool"]["must"][0],
            json!({ "multi_match": { "query": "x", "type": "phrase", "fields": ["name", "nickname"] } })
        );
        assert_eq!(
            exact["query"]["bool"]["filter"][0]["terms"]["eraSlug"],
            json!(["abbasi"])
        );
    }

    #[test]
    fn ranked_and_exact_poem_queries_break_score_ties_by_id() {
        for body in [
            poem_search_body(&poems("حب", 1, false)),
            poem_search_body(&poems("حب", 1, true)),
        ] {
            assert_eq!(body["sort"], json!([{ "_score": "desc" }, { "id": "asc" }]));
        }
    }

    #[test]
    fn an_exact_poet_query_breaks_score_ties_like_the_poets_list() {
        let body = poet_search_body(&PoetSearchParams {
            q: "المتنبي".into(),
            exact: true,
            ..PoetSearchParams::default()
        });
        assert_eq!(
            body["sort"],
            json!([{ "_score": "desc" }, { "poemsCount": "desc" }, { "nameSort": "asc" }, { "id": "asc" }])
        );
    }

    #[test]
    fn a_poet_is_admitted_only_when_every_query_term_reaches_the_name_or_nickname() {
        let body = poet_search_body(&PoetSearchParams {
            q: "ابو الطيب".into(),
            era_slugs: vec!["abbasi".into()],
            ..PoetSearchParams::default()
        });
        assert_eq!(
            body["query"]["bool"]["filter"],
            json!([
                { "terms": { "eraSlug": ["abbasi"] } },
                { "multi_match": {
                    "query": "ابو الطيب",
                    "type": "cross_fields",
                    "operator": "and",
                    "fields": ["name.autocomplete", "name.stemmed", "nickname.autocomplete", "nickname.stemmed"],
                } },
            ])
        );
    }
}
