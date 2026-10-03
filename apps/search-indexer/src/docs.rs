use serde::Serialize;

use crate::arabic::{fold_for_sort, strip_tashkeel};

pub(crate) struct PoemSource {
    pub id: i32,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub poet_name: String,
    pub poet_slug: String,
    pub poet_has_avatar: bool,
    pub poet_is_anonymous: bool,
    pub era_name: String,
    pub era_slug: String,
    pub meter_name: String,
    pub meter_slug: String,
    pub theme_slug: String,
    pub rhyme_slug: String,
    pub poem_type_slug: String,
    pub collection_slug: String,
    pub primary_id: i32,
    pub is_primary: bool,
}

pub(crate) struct PoetSource {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub nickname: String,
    pub era_name: String,
    pub era_slug: String,
    pub poems_count: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PoemDoc {
    pub id: i32,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub title_display: String,
    pub poet_name_display: String,
    pub poet_slug: String,
    pub poet_has_avatar: bool,
    pub poet_is_anonymous: bool,
    pub era_slug: String,
    pub era_name: String,
    pub meter_slug: String,
    pub meter_name: String,
    pub theme_slug: String,
    pub rhyme_slug: String,
    pub poem_type_slug: String,
    pub collection_slug: String,
    pub primary_id: i32,
    pub is_primary: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PoetDoc {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub nickname: String,
    pub name_display: String,
    pub name_sort: String,
    pub poems_count: i32,
    pub era_slug: String,
    pub era_name: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Document {
    Poem(PoemDoc),
    Poet(PoetDoc),
}

impl Document {
    pub(crate) fn slug(&self) -> &str {
        match self {
            Document::Poem(poem) => &poem.slug,
            Document::Poet(poet) => &poet.slug,
        }
    }
}

pub(crate) fn to_poem_doc(src: PoemSource) -> PoemDoc {
    PoemDoc {
        id: src.id,
        slug: src.slug,
        title: strip_tashkeel(&src.title),
        content: src.content,
        title_display: src.title,
        poet_name_display: src.poet_name,
        poet_slug: src.poet_slug,
        poet_has_avatar: src.poet_has_avatar,
        poet_is_anonymous: src.poet_is_anonymous,
        era_slug: src.era_slug,
        era_name: src.era_name,
        meter_slug: src.meter_slug,
        meter_name: src.meter_name,
        theme_slug: src.theme_slug,
        rhyme_slug: src.rhyme_slug,
        poem_type_slug: src.poem_type_slug,
        collection_slug: src.collection_slug,
        primary_id: src.primary_id,
        is_primary: src.is_primary,
    }
}

pub(crate) fn to_poet_doc(src: PoetSource, rules: &[(String, String)]) -> PoetDoc {
    PoetDoc {
        id: src.id,
        slug: src.slug,
        name: strip_tashkeel(&src.name),
        nickname: strip_tashkeel(&src.nickname),
        name_sort: fold_for_sort(&src.name, rules),
        name_display: src.name,
        poems_count: src.poems_count,
        era_slug: src.era_slug,
        era_name: src.era_name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn poem() -> PoemSource {
        PoemSource {
            id: 7,
            slug: "TnKK".into(),
            title: "قَصِيدَة".into(),
            content: "أ*ب".into(),
            poet_name: "المُتَنَبِّي".into(),
            poet_slug: "yoFB".into(),
            poet_has_avatar: true,
            poet_is_anonymous: false,
            era_name: "عباسي".into(),
            era_slug: "abbasi".into(),
            meter_name: "الطويل".into(),
            meter_slug: "altawil".into(),
            theme_slug: "alnasib".into(),
            rhyme_slug: "meem".into(),
            poem_type_slug: "amudi".into(),
            collection_slug: String::new(),
            primary_id: 7,
            is_primary: true,
        }
    }

    fn poet() -> PoetSource {
        PoetSource {
            id: 3,
            slug: "yoFB".into(),
            name: " أَحْمَد ".into(),
            nickname: "أَمِيرُ الشُّعَرَاء".into(),
            era_name: "عباسي".into(),
            era_slug: "abbasi".into(),
            poems_count: 42,
        }
    }

    fn rules() -> Vec<(String, String)> {
        qafiyah_elasticsearch::folding_rules(&qafiyah_elasticsearch::load().poems)
    }

    #[test]
    fn a_poem_document_strips_searchable_text_and_keeps_the_originals_for_display() {
        let doc = to_poem_doc(poem());
        assert_eq!(doc.title, "قصيدة");
        assert_eq!(doc.title_display, "قَصِيدَة");
        assert_eq!(doc.poet_name_display, "المُتَنَبِّي");
        assert_eq!(doc.content, "أ*ب", "content is never stripped in Rust");
        assert_eq!(
            (doc.id, doc.slug.as_str(), doc.collection_slug.as_str()),
            (7, "TnKK", "")
        );
        assert!(doc.poet_has_avatar);
        assert!(!doc.poet_is_anonymous);
    }

    #[test]
    fn a_poem_document_carries_the_id_of_its_primary_and_whether_it_is_the_primary() {
        let primary = to_poem_doc(poem());
        assert_eq!((primary.primary_id, primary.is_primary), (7, true));
        let mut reading = poem();
        reading.id = 9;
        reading.primary_id = 7;
        reading.is_primary = false;
        let alternate = to_poem_doc(reading);
        assert_eq!(
            (alternate.id, alternate.primary_id, alternate.is_primary),
            (9, 7, false)
        );
    }

    #[test]
    fn a_poem_document_carries_its_verse_form_slug() {
        let mut source = poem();
        source.poem_type_slug = "hurr".into();
        assert_eq!(to_poem_doc(source).poem_type_slug, "hurr");
    }

    #[test]
    fn a_poet_document_folds_the_sort_key_and_trims_it() {
        let doc = to_poet_doc(poet(), &rules());
        assert_eq!(doc.name, " أحمد ");
        assert_eq!(doc.name_sort, "احمد");
        assert_eq!(doc.name_display, " أَحْمَد ");
        assert_eq!(doc.nickname, "أمير الشعراء");
        assert_eq!(doc.poems_count, 42);
    }

    #[test]
    fn a_poem_with_empty_content_still_maps() {
        let mut source = poem();
        source.content = String::new();
        assert_eq!(to_poem_doc(source).content, "");
    }

    #[test]
    fn the_document_keys_match_the_strict_mappings_exactly() {
        let schema = qafiyah_elasticsearch::load();
        let keys = |value: serde_json::Value| {
            value
                .as_object()
                .expect("object")
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
        };
        let mapped = |body: &serde_json::Value| {
            body["mappings"]["properties"]
                .as_object()
                .expect("properties")
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(
            keys(serde_json::to_value(to_poem_doc(poem())).expect("json")),
            mapped(&schema.poems)
        );
        assert_eq!(
            keys(serde_json::to_value(to_poet_doc(poet(), &rules())).expect("json")),
            mapped(&schema.poets)
        );
    }
}
