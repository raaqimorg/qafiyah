use serde::Deserialize;
use serde_json::Value;

const SCHEMA_JSON: &str = include_str!("../schema.json");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub poems_alias: String,
    pub poets_alias: String,
    pub poems_prefix: String,
    pub poets_prefix: String,
    pub api_username: String,
    pub api_role: String,
    pub bulk_batch_size: usize,
}

#[derive(Deserialize)]
pub struct Schema {
    pub poems: Value,
    pub poets: Value,
    pub identity: Identity,
}

#[expect(
    clippy::expect_used,
    reason = "schema.json is embedded at compile time, so a parse failure is a programmer error"
)]
pub fn load() -> Schema {
    serde_json::from_str(SCHEMA_JSON).expect("invariant: schema.json is valid JSON")
}

#[expect(
    clippy::expect_used,
    reason = "schema.json is embedded at compile time, so missing keys are a programmer error"
)]
pub fn folding_mappings(body: &Value) -> Vec<String> {
    body.get("settings")
        .and_then(|value| value.get("analysis"))
        .and_then(|value| value.get("char_filter"))
        .and_then(|value| value.get("arabic_letter_folding"))
        .and_then(|value| value.get("mappings"))
        .and_then(Value::as_array)
        .expect("invariant: folding mappings present in schema.json")
        .iter()
        .map(|value| value.as_str().unwrap_or_default().to_string())
        .collect()
}

fn parse_rule(rule: &str) -> Option<(String, String)> {
    let (from, to) = rule.split_once(" => ")?;
    (!from.is_empty()).then(|| (from.to_string(), to.to_string()))
}

#[expect(
    clippy::expect_used,
    reason = "schema.json is embedded at compile time, so every rule parses or the data is wrong"
)]
pub fn folding_rules(body: &Value) -> Vec<(String, String)> {
    folding_mappings(body)
        .iter()
        .map(|rule| parse_rule(rule).expect("invariant: every folding rule parses"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn index_bodies_use_strict_mappings() {
        let schema = load();
        assert_eq!(schema.poems["mappings"]["dynamic"], "strict");
        assert_eq!(schema.poets["mappings"]["dynamic"], "strict");
    }

    #[test]
    fn poem_title_exposes_exact_normalized_and_stemmed() {
        let schema = load();
        let title = &schema.poems["mappings"]["properties"]["title"];
        assert_eq!(title["analyzer"], "arabic_normalized");
        assert_eq!(title["fields"]["exact"]["type"], "keyword");
        assert_eq!(title["fields"]["stemmed"]["analyzer"], "arabic_stemmed");
    }

    #[test]
    fn every_exact_field_folds_letters_like_the_normalized_analyzer_but_keeps_the_standalone_hamza()
    {
        let schema = load();
        for (body, fields) in [
            (&schema.poems, vec!["title", "poetName"]),
            (&schema.poets, vec!["name"]),
        ] {
            let analysis = &body["settings"]["analysis"];
            let normalizer = &analysis["normalizer"]["arabic_exact"];
            let analyzer = &analysis["analyzer"]["arabic_normalized"];
            assert_eq!(normalizer["type"], "custom");
            assert_eq!(
                normalizer["char_filter"],
                json!(["arabic_letter_folding_keep_hamza", "invisible_marks"])
            );
            assert_eq!(normalizer["filter"], analyzer["filter"]);
            for field in fields {
                let exact = &body["mappings"]["properties"][field]["fields"]["exact"];
                assert_eq!(exact["type"], "keyword", "{field}");
                assert_eq!(exact["normalizer"], "arabic_exact", "{field}");
            }
        }
    }

    #[test]
    fn the_char_filter_folds_ta_marbuta_ya_and_hamza_carriers() {
        let mappings = folding_mappings(&load().poems);
        for rule in ["ة => ه", "ى => ي", "ؤ => و", "ئ => ي", "ء => "] {
            assert!(
                mappings.iter().any(|m| m == rule),
                "folding rule missing: {rule}"
            );
        }
    }

    #[test]
    fn arabic_normalized_applies_the_folding_char_filter() {
        let schema = load();
        let analyzer = &schema.poems["settings"]["analysis"]["analyzer"]["arabic_normalized"];
        assert!(
            analyzer["char_filter"]
                .as_array()
                .expect("char_filter list")
                .iter()
                .any(|f| f == "arabic_letter_folding")
        );
    }

    #[test]
    fn autocomplete_is_edge_ngram_and_attached_to_name_like_fields_only() {
        let schema = load();
        let analysis = &schema.poems["settings"]["analysis"];
        assert_eq!(analysis["filter"]["edge_ngram_2"]["type"], "edge_ngram");
        assert_eq!(analysis["filter"]["edge_ngram_2"]["min_gram"], 2);
        assert_eq!(analysis["filter"]["edge_ngram_2"]["max_gram"], 20);

        let poems = &schema.poems["mappings"]["properties"];
        assert_eq!(
            poems["title"]["fields"]["autocomplete"]["analyzer"],
            "autocomplete_2"
        );
        assert_eq!(
            poems["title"]["fields"]["autocomplete"]["search_analyzer"],
            "arabic_normalized"
        );
        assert_eq!(
            poems["poetName"]["fields"]["autocomplete"]["analyzer"],
            "autocomplete_2"
        );
        assert_eq!(
            schema.poets["mappings"]["properties"]["name"]["fields"]["autocomplete"]["analyzer"],
            "autocomplete_name"
        );
    }

    #[test]
    fn the_poem_body_carries_a_stemmed_subfield_only() {
        let schema = load();
        let content = &schema.poems["mappings"]["properties"]["content"];
        assert_eq!(content["fields"]["stemmed"]["analyzer"], "arabic_stemmed");
        assert!(content["fields"]["exact"].is_null());
        assert!(content["fields"]["autocomplete"].is_null());
    }

    #[test]
    fn poems_carry_the_id_of_their_primary_and_a_primary_flag_for_grouping_readings() {
        let poems = &load().poems["mappings"]["properties"];
        assert_eq!(poems["primaryId"]["type"], "integer");
        assert_eq!(poems["isPrimary"]["type"], "boolean");
    }

    #[test]
    fn the_hamza_keeping_folding_is_the_main_folding_without_the_hamza_deletion() {
        let schema = load();
        for body in [&schema.poems, &schema.poets] {
            let filters = &body["settings"]["analysis"]["char_filter"];
            let main: Vec<&str> = filters["arabic_letter_folding"]["mappings"]
                .as_array()
                .expect("main folding")
                .iter()
                .filter_map(Value::as_str)
                .filter(|rule| *rule != "ء => ")
                .collect();
            let kept: Vec<&str> = filters["arabic_letter_folding_keep_hamza"]["mappings"]
                .as_array()
                .expect("hamza-keeping folding")
                .iter()
                .filter_map(Value::as_str)
                .collect();
            assert_eq!(kept, main);
            assert_eq!(
                filters["arabic_letter_folding_keep_hamza"]["type"],
                "mapping"
            );
        }
    }

    #[test]
    fn exact_keywords_and_hamza_subfields_keep_the_standalone_hamza() {
        let schema = load();
        for body in [&schema.poems, &schema.poets] {
            let analysis = &body["settings"]["analysis"];
            assert_eq!(
                analysis["normalizer"]["arabic_exact"]["char_filter"],
                json!(["arabic_letter_folding_keep_hamza", "invisible_marks"])
            );
            let kept = &analysis["analyzer"]["arabic_hamza_kept"];
            assert_eq!(
                kept["char_filter"],
                json!(["arabic_letter_folding_keep_hamza", "invisible_marks"])
            );
            assert_eq!(
                kept["filter"],
                analysis["analyzer"]["arabic_normalized"]["filter"]
            );
        }
        let poems = &schema.poems["mappings"]["properties"];
        for field in ["title", "content"] {
            assert_eq!(
                poems[field]["fields"]["hamza"]["analyzer"], "arabic_hamza_kept",
                "{field}"
            );
        }
    }

    #[test]
    fn poets_store_a_count_and_a_sortable_name() {
        let poets = &load().poets["mappings"]["properties"];
        assert_eq!(poets["poemsCount"]["type"], "integer");
        assert_eq!(poets["nameSort"]["type"], "keyword");
    }

    #[test]
    fn index_identity_pairs_aliases_with_versioned_prefixes() {
        let identity = load().identity;
        assert_eq!(identity.poems_alias, "poems");
        assert_eq!(identity.poets_alias, "poets");
        assert!(identity.poems_prefix.starts_with(&identity.poems_alias));
        assert!(identity.poets_prefix.starts_with(&identity.poets_alias));
    }

    #[test]
    fn every_folding_rule_splits_into_a_source_and_a_target() {
        let rules = folding_rules(&load().poems);
        assert!(rules.len() >= 10);
        for (from, to) in &rules {
            assert!(
                !from.is_empty(),
                "a rule with an empty source would delete nothing"
            );
            assert!(
                to.chars().count() <= 1,
                "targets are one letter or empty: {to:?}"
            );
        }
        assert!(rules.iter().any(|(from, to)| from == "أ" && to == "ا"));
        assert!(rules.iter().any(|(from, to)| from == "ء" && to.is_empty()));
    }

    #[test]
    fn a_mapping_without_the_arrow_is_refused_rather_than_read_as_a_delete() {
        assert_eq!(
            parse_rule("ة => ه"),
            Some(("ة".to_string(), "ه".to_string()))
        );
        assert_eq!(parse_rule("ء => "), Some(("ء".to_string(), String::new())));
        assert_eq!(parse_rule("ة"), None);
        assert_eq!(parse_rule(" => ه"), None);
    }

    const BUILT_IN_FILTERS: [&str; 3] = ["lowercase", "decimal_digit", "arabic_normalization"];

    fn names(value: &Value) -> Vec<String> {
        value
            .as_object()
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn analyzers_named_by_fields(properties: &Value) -> Vec<String> {
        let mut found = Vec::new();
        for (_, field) in properties.as_object().expect("properties") {
            for key in ["analyzer", "search_analyzer"] {
                if let Some(name) = field[key].as_str() {
                    found.push(name.to_string());
                }
            }
            if let Some(subfields) = field["fields"].as_object() {
                for (_, sub) in subfields {
                    for key in ["analyzer", "search_analyzer"] {
                        if let Some(name) = sub[key].as_str() {
                            found.push(name.to_string());
                        }
                    }
                }
            }
        }
        found
    }

    #[test]
    fn every_analyzer_a_field_names_is_defined() {
        for body in [&load().poems, &load().poets] {
            let defined = names(&body["settings"]["analysis"]["analyzer"]);
            let named = analyzers_named_by_fields(&body["mappings"]["properties"]);
            assert!(!named.is_empty());
            for name in named {
                assert!(defined.contains(&name), "undefined analyzer {name}");
            }
        }
    }

    #[test]
    fn every_filter_and_char_filter_an_analyzer_names_is_defined_or_built_in() {
        let body = load().poems;
        let analysis = &body["settings"]["analysis"];
        let filters = names(&analysis["filter"]);
        let char_filters = names(&analysis["char_filter"]);
        for (name, analyzer) in analysis["analyzer"].as_object().expect("analyzers") {
            assert_eq!(analyzer["tokenizer"], "standard", "{name}");
            for filter in analyzer["filter"].as_array().expect("filters") {
                let filter = filter.as_str().expect("a filter name");
                assert!(
                    filters.iter().any(|f| f == filter) || BUILT_IN_FILTERS.contains(&filter),
                    "{name} names unknown filter {filter}"
                );
            }
            for char_filter in analyzer["char_filter"].as_array().expect("char filters") {
                let char_filter = char_filter.as_str().expect("a char filter name");
                assert!(
                    char_filters.iter().any(|f| f == char_filter),
                    "{name} names unknown char filter {char_filter}"
                );
            }
        }
    }

    #[test]
    fn the_poets_index_analyzes_text_exactly_like_the_poems_index() {
        let schema = load();
        assert_eq!(
            schema.poems["settings"]["analysis"],
            schema.poets["settings"]["analysis"]
        );
    }

    #[test]
    fn slug_fields_are_keywords_and_display_fields_are_stored_but_never_indexed() {
        let schema = load();
        for (body, display) in [
            (
                &schema.poems,
                vec!["titleDisplay", "poetNameDisplay", "eraName", "meterName"],
            ),
            (&schema.poets, vec!["nameDisplay", "eraName"]),
        ] {
            let properties = body["mappings"]["properties"]
                .as_object()
                .expect("properties");
            for (name, field) in properties {
                if name.ends_with("Slug") {
                    assert_eq!(field["type"], "keyword", "{name}");
                }
            }
            for name in display {
                assert_eq!(properties[name]["type"], "keyword", "{name}");
                assert_eq!(properties[name]["index"], false, "{name}");
            }
        }
        assert_eq!(
            schema.poems["mappings"]["properties"]["title"]["fields"]["exact"]["ignore_above"],
            256
        );
    }

    #[test]
    fn the_identity_block_is_complete_and_sane() {
        let identity = load().identity;
        assert_ne!(identity.poems_prefix, identity.poems_alias);
        assert_ne!(identity.poets_prefix, identity.poets_alias);
        assert_eq!(identity.api_username, "qafiyah_api");
        assert_eq!(identity.api_role, "qafiyah_reader");
        assert!(identity.bulk_batch_size > 0);
        assert!(
            identity.bulk_batch_size <= 5_000,
            "one bulk request stays well under the Elasticsearch body limit"
        );
    }

    #[test]
    fn the_poets_result_window_matches_the_api_constant_pinned_in_the_constants_check() {
        assert_eq!(load().poets["settings"]["max_result_window"], 50_000);
        assert!(
            load().poems["settings"]["max_result_window"].is_null(),
            "poems rely on the Elasticsearch default of 10000"
        );
    }

    const NAME_ANALYZERS: [&str; 3] = [
        "arabic_name_normalized",
        "arabic_name_stemmed",
        "autocomplete_name",
    ];

    #[test]
    fn poet_names_treat_the_five_nouns_their_case_forms_and_ibn_as_one_word_and_split_joined_compounds()
     {
        let schema = load();
        for body in [&schema.poems, &schema.poets] {
            let analysis = &body["settings"]["analysis"];
            assert_eq!(
                analysis["filter"]["name_equivalents"],
                json!({
                    "type": "synonym",
                    "synonyms": ["ابن, بن", "ابو, ابي, ابا", "اخو, اخي, اخا", "ذو, ذي, ذا", "امرو, امري, امرا, امر"],
                })
            );
            assert_eq!(
                analysis["char_filter"]["name_compound_split"],
                json!({ "type": "pattern_replace", "pattern": "(^|\\s)(عبد(?=ال)|ابو(?=\\S\\S))", "replacement": "$1$2 " })
            );
            for name in NAME_ANALYZERS {
                let analyzer = &analysis["analyzer"][name];
                let char_filters = analyzer["char_filter"].as_array().expect("char filters");
                let filters = analyzer["filter"].as_array().expect("filters");
                assert_eq!(
                    char_filters.first(),
                    Some(&json!("arabic_letter_folding")),
                    "{name}"
                );
                assert!(
                    char_filters.iter().any(|f| f == "name_compound_split"),
                    "{name}"
                );
                assert!(filters.iter().any(|f| f == "name_equivalents"), "{name}");
            }
        }
    }

    #[test]
    fn the_name_analyzers_serve_poet_names_and_nicknames_only() {
        let schema = load();
        let poets = &schema.poets["mappings"]["properties"];
        for field in ["name", "nickname"] {
            assert_eq!(
                poets[field]["analyzer"], "arabic_name_normalized",
                "{field}"
            );
            assert_eq!(
                poets[field]["fields"]["stemmed"]["analyzer"], "arabic_name_stemmed",
                "{field}"
            );
            assert_eq!(
                poets[field]["fields"]["autocomplete"]["analyzer"], "autocomplete_name",
                "{field}"
            );
            assert_eq!(
                poets[field]["fields"]["autocomplete"]["search_analyzer"], "arabic_name_normalized",
                "{field}"
            );
        }
        let poems = analyzers_named_by_fields(&schema.poems["mappings"]["properties"]);
        assert!(
            poems
                .iter()
                .all(|name| !NAME_ANALYZERS.contains(&name.as_str())),
            "{poems:?}"
        );
    }

    #[test]
    fn every_analyzer_and_the_exact_normalizer_delete_the_marks_and_controls_the_folding_leaves() {
        let schema = load();
        for body in [&schema.poems, &schema.poets] {
            let analysis = &body["settings"]["analysis"];
            assert_eq!(
                analysis["char_filter"]["invisible_marks"],
                json!({
                    "type": "pattern_replace",
                    "pattern": "[\u{0610}-\u{061A}\u{0653}-\u{065F}\u{06D6}-\u{06ED}\u{202A}-\u{202E}\u{2066}-\u{2069}]",
                    "replacement": "",
                })
            );
            let mut chains = vec![(
                "arabic_exact".to_string(),
                &analysis["normalizer"]["arabic_exact"],
            )];
            for (name, analyzer) in analysis["analyzer"].as_object().expect("analyzers") {
                chains.push((name.clone(), analyzer));
            }
            for (name, chain) in chains {
                let char_filters = chain["char_filter"].as_array().expect("char filters");
                assert_eq!(
                    char_filters.get(1),
                    Some(&json!("invisible_marks")),
                    "{name}"
                );
            }
        }
    }
}
