use serde_json::{Map, Value, json};

use crate::constants::{ES_MAX_RESULT_WINDOW, SEARCH_POEMS_PER_PAGE, SEARCH_POETS_PER_PAGE};

const RECALL_FLOOR: &str = "1<75%";

const CLASSICAL_ERA_SLUGS: [&str; 8] = [
    "jahili", "islami", "umawi", "abbasi", "andalusi", "fatimi", "ayyubi", "mamluki",
];
const CLASSICAL_ERA_WEIGHT: f64 = 1.1;

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

    let query = if !has_text {
        json!({ "bool": { "must": [{ "match_all": {} }], "filter": filters } })
    } else if params.exact {
        json!({ "bool": {
            "must": [{ "match_phrase": { "content": { "query": params.q } } }],
            "filter": filters,
        } })
    } else {
        let mut gate = vec![recall_gate(&params.q, &POEM_FIELDS)];
        gate.extend(filters);
        let ranked = json!({ "bool": {
            "filter": gate,
            "should": ranking_clauses(&params.q, &POEM_FIELDS),
        } });
        if params.era_slugs.is_empty() {
            json!({ "function_score": {
                "query": ranked,
                "functions": [{
                    "filter": { "terms": { "eraSlug": CLASSICAL_ERA_SLUGS } },
                    "weight": CLASSICAL_ERA_WEIGHT,
                }],
                "boost_mode": "multiply",
            } })
        } else {
            ranked
        }
    };

    body(
        params
            .page
            .saturating_sub(1)
            .saturating_mul(SEARCH_POEMS_PER_PAGE),
        SEARCH_POEMS_PER_PAGE,
        ES_MAX_RESULT_WINDOW,
        query,
        (!has_text).then(|| json!([{ "id": "desc" }])),
        has_text.then(|| highlight(0, None, "content", &["content", "content.stemmed"])),
    )
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
            "must": [{ "match_phrase": { "name": { "query": params.q } } }],
            "filter": filters,
        } })
    } else {
        json!({ "bool": {
            "should": [
                { "term": { "name.exact": { "value": params.q, "boost": poet_boost::EXACT } } },
                { "match_phrase": { "name": { "query": params.q, "boost": poet_boost::PHRASE } } },
                { "match": { "name.autocomplete": { "query": params.q, "boost": poet_boost::PREFIX } } },
                { "match": { "name.stemmed": { "query": params.q, "boost": poet_boost::STEMMED } } },
                { "match": { "name": { "query": params.q, "fuzziness": "AUTO", "boost": poet_boost::FUZZY } } },
            ],
            "minimum_should_match": 1,
            "filter": filters,
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
        (!has_text).then_some(browse_sort),
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
    fn an_empty_poem_query_browses_by_id_without_highlighting() {
        let body = poem_search_body(&poems("", 1, false));
        assert_eq!(body["from"], 0);
        assert_eq!(body["size"], SEARCH_POEMS_PER_PAGE);
        assert_eq!(body["track_total_hits"], ES_MAX_RESULT_WINDOW);
        assert_eq!(body["sort"], json!([{ "id": "desc" }]));
        assert!(body.get("highlight").is_none());
        assert_eq!(body["query"]["bool"]["must"], json!([{ "match_all": {} }]));
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
        assert_eq!(ranked["query"]["bool"]["minimum_should_match"], 1);
        assert!(ranked.get("sort").is_none());
        let exact = poet_search_body(&PoetSearchParams {
            q: "x".into(),
            exact: true,
            era_slugs: vec!["abbasi".into()],
            ..PoetSearchParams::default()
        });
        assert_eq!(
            exact["query"]["bool"]["must"][0]["match_phrase"]["name"]["query"],
            "x"
        );
        assert_eq!(
            exact["query"]["bool"]["filter"][0]["terms"]["eraSlug"],
            json!(["abbasi"])
        );
    }
}
