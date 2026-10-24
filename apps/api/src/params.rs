use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::json;
use utoipa::ToSchema;
use utoipa::openapi::schema::{ArrayBuilder, ObjectBuilder, Schema, Type};
use utoipa::openapi::{Ref, RefOr};

use crate::constants::{
    ES_MAX_RESULT_WINDOW, MAX_CURSOR_LENGTH, MAX_FILTER_SLUGS, MAX_QUERY_LENGTH,
    MAX_TERM_SLUG_LENGTH,
};
use crate::cursor::{self, Key};
use crate::domain::paging::Cursor;
use crate::domain::search::PoetPosition;

const FOUR_LETTERS: &str = "must be four letters, a to z or A to Z";

#[derive(Debug, Clone, ToSchema)]
#[schema(value_type = String, pattern = "^[a-zA-Z]{4}$")]
pub struct FourLetterSlug(String);

impl FourLetterSlug {
    pub fn parse(raw: &str) -> Result<Self, String> {
        if raw.len() == 4 && raw.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            Ok(Self(raw.to_string()))
        } else {
            Err(FOUR_LETTERS.to_string())
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for FourLetterSlug {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, ToSchema)]
#[schema(value_type = String, pattern = "^[a-z][a-z-]*$", max_length = 64)]
pub struct TransliteratedSlug(String);

impl TransliteratedSlug {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let mut bytes = raw.bytes();
        let starts_lower = bytes.next().is_some_and(|byte| byte.is_ascii_lowercase());
        if starts_lower
            && raw.len() <= MAX_TERM_SLUG_LENGTH
            && bytes.all(|byte| byte.is_ascii_lowercase() || byte == b'-')
        {
            Ok(Self(raw.to_string()))
        } else {
            Err(format!(
                "must be a lowercase slug of letters and hyphens, at most {MAX_TERM_SLUG_LENGTH} characters"
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for TransliteratedSlug {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

pub const NOT_A_CURSOR: &str = "must be the `next` or `prev` value of a page of this list";

fn cursor_schema() -> RefOr<Schema> {
    ObjectBuilder::new()
        .schema_type(Type::String)
        .max_length(Some(MAX_CURSOR_LENGTH))
        .pattern(Some("^[A-Za-z0-9_-]+$"))
        .into()
}

fn cursor_text<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let raw = String::deserialize(deserializer)?;
    if raw.len() > MAX_CURSOR_LENGTH {
        return Err(de::Error::custom(NOT_A_CURSOR));
    }
    Ok(raw)
}

#[derive(Debug)]
pub struct PageCursor<K>(Cursor<K>);

impl<K: Key> PageCursor<K> {
    pub fn parse(raw: &str) -> Result<Self, String> {
        cursor::decode(raw)
            .map(Self)
            .ok_or_else(|| NOT_A_CURSOR.to_string())
    }

    pub fn into_inner(self) -> Cursor<K> {
        self.0
    }
}

impl<'de, K: Key> Deserialize<'de> for PageCursor<K> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(&cursor_text(deserializer)?).map_err(de::Error::custom)
    }
}

impl<K> utoipa::PartialSchema for PageCursor<K> {
    fn schema() -> RefOr<Schema> {
        cursor_schema()
    }
}

pub type IdCursor = PageCursor<i32>;
pub type PoetCursor = PageCursor<PoetPosition>;

#[derive(Debug, Clone, Copy)]
pub struct SearchCursor<const PAGE_SIZE: u32>(u32);

impl<const PAGE_SIZE: u32> SearchCursor<PAGE_SIZE> {
    pub fn parse(raw: &str) -> Result<Self, String> {
        cursor::decode_offset(raw)
            .filter(|offset| offset.saturating_add(PAGE_SIZE) <= ES_MAX_RESULT_WINDOW)
            .map(Self)
            .ok_or_else(|| NOT_A_CURSOR.to_string())
    }

    pub fn offset(self) -> u32 {
        self.0
    }
}

impl<'de, const PAGE_SIZE: u32> Deserialize<'de> for SearchCursor<PAGE_SIZE> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(&cursor_text(deserializer)?).map_err(de::Error::custom)
    }
}

impl<const PAGE_SIZE: u32> utoipa::PartialSchema for SearchCursor<PAGE_SIZE> {
    fn schema() -> RefOr<Schema> {
        cursor_schema()
    }
}

#[derive(Debug, Clone)]
pub struct SearchText(String);

impl SearchText {
    pub fn parse(raw: String) -> Result<Self, String> {
        if raw.chars().count() <= MAX_QUERY_LENGTH {
            Ok(Self(raw))
        } else {
            Err(format!("must be at most {MAX_QUERY_LENGTH} characters"))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SearchText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl utoipa::PartialSchema for SearchText {
    fn schema() -> RefOr<Schema> {
        ObjectBuilder::new()
            .schema_type(Type::String)
            .max_length(Some(MAX_QUERY_LENGTH))
            .into()
    }
}

fn capped<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Vec<T>, D::Error> {
    let values = Vec::<T>::deserialize(deserializer)?;
    if values.len() > MAX_FILTER_SLUGS {
        return Err(de::Error::custom(format!(
            "takes at most {MAX_FILTER_SLUGS} values"
        )));
    }
    Ok(values)
}

fn slug_list_schema(item: &str) -> RefOr<Schema> {
    ArrayBuilder::new()
        .items(RefOr::Ref(Ref::from_schema_name(item)))
        .default(Some(json!([])))
        .max_items(Some(MAX_FILTER_SLUGS))
        .into()
}

#[derive(Debug, Clone, Default)]
pub struct PoetSlugs(Vec<FourLetterSlug>);

impl PoetSlugs {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_strings(self) -> Vec<String> {
        self.0.into_iter().map(FourLetterSlug::into_inner).collect()
    }
}

impl<'de> Deserialize<'de> for PoetSlugs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        capped(deserializer).map(Self)
    }
}

impl utoipa::PartialSchema for PoetSlugs {
    fn schema() -> RefOr<Schema> {
        slug_list_schema(&FourLetterSlug::name())
    }
}

#[derive(Debug, Clone, Default)]
pub struct TermSlugs(Vec<TransliteratedSlug>);

impl TermSlugs {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_strings(self) -> Vec<String> {
        self.0
            .into_iter()
            .map(TransliteratedSlug::into_inner)
            .collect()
    }
}

impl<'de> Deserialize<'de> for TermSlugs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        capped(deserializer).map(Self)
    }
}

impl utoipa::PartialSchema for TermSlugs {
    fn schema() -> RefOr<Schema> {
        slug_list_schema(&TransliteratedSlug::name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SearchTypeParam {
    Poems,
    Poets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum RandomPoemOptionParam {
    Slug,
    Lines,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExactFlag {
    True,
    False,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoParams {}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct SlugsParams {
    /// The `next` or `prev` value of a page of slugs. Leave it out for the first page.
    #[param(inline, example = "eyJhZnRlciI6MX0")]
    pub cursor: Option<IdCursor>,
}

#[cfg(test)]
mod tests {
    use utoipa::PartialSchema;

    use super::*;

    #[derive(Debug, Deserialize)]
    struct Lists {
        #[serde(default)]
        poet: PoetSlugs,
        #[serde(default)]
        era: TermSlugs,
    }

    fn lists(raw: &str) -> Result<Lists, String> {
        serde_html_form::from_str(raw).map_err(|error| error.to_string())
    }

    #[test]
    fn a_four_letter_slug_is_exactly_four_ascii_letters() {
        assert_eq!(FourLetterSlug::parse("TnKK").unwrap().as_str(), "TnKK");
        for raw in ["", "abc", "abcde", "ab1d", "عربي", "ab d"] {
            assert_eq!(
                FourLetterSlug::parse(raw).unwrap_err(),
                FOUR_LETTERS,
                "{raw}"
            );
        }
    }

    #[test]
    fn a_transliterated_slug_is_lowercase_letters_and_hyphens_up_to_sixty_four() {
        for raw in ["abbasi", "al-tawil", "a", "a-", "a--b"] {
            assert!(TransliteratedSlug::parse(raw).is_ok(), "{raw}");
        }
        assert!(TransliteratedSlug::parse(&"a".repeat(64)).is_ok());
        for raw in ["", "-abbasi", "Abbasi", "abbasi1", "abbasi_x", "عربي"] {
            assert!(TransliteratedSlug::parse(raw).is_err(), "{raw}");
        }
        assert!(TransliteratedSlug::parse(&"a".repeat(65)).is_err());
    }

    #[test]
    fn slug_checks_never_accept_a_byte_outside_their_alphabet() {
        let mut rng = crate::test_support::Rng::new(5);
        for _ in 0..5_000 {
            let len = usize::try_from(rng.below(12)).expect("bounded length");
            let raw = String::from_utf8_lossy(&rng.bytes(len)).into_owned();
            if TransliteratedSlug::parse(&raw).is_ok() {
                assert!(raw.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'));
                assert!(raw.as_bytes()[0].is_ascii_lowercase());
            }
            if FourLetterSlug::parse(&raw).is_ok() {
                assert_eq!(raw.len(), 4);
                assert!(raw.bytes().all(|b| b.is_ascii_alphabetic()));
            }
        }
    }

    #[test]
    fn a_cursor_parameter_reads_only_a_cursor_that_a_page_gave_out() {
        let next = cursor::encode(Cursor::After(1264));
        assert!(matches!(
            PageCursor::<i32>::parse(&next).unwrap().into_inner(),
            Cursor::After(1264)
        ));
        assert_eq!(PageCursor::<i32>::parse("2").unwrap_err(), NOT_A_CURSOR);
        let long = "A".repeat(MAX_CURSOR_LENGTH + 1);
        assert!(serde_html_form::from_str::<SlugsParams>(&format!("cursor={long}")).is_err());
    }

    #[test]
    fn a_search_cursor_stays_inside_the_result_window() {
        let last = cursor::encode_offset(ES_MAX_RESULT_WINDOW - 20);
        assert_eq!(SearchCursor::<20>::parse(&last).unwrap().offset(), 9980);
        let past = cursor::encode_offset(ES_MAX_RESULT_WINDOW - 19);
        assert_eq!(SearchCursor::<20>::parse(&past).unwrap_err(), NOT_A_CURSOR);
        let poems = cursor::encode(Cursor::After(1));
        assert!(SearchCursor::<20>::parse(&poems).is_err());
    }

    #[test]
    fn search_text_is_capped_at_a_hundred_characters() {
        assert!(SearchText::parse("ا".repeat(100)).is_ok());
        assert!(SearchText::parse("ا".repeat(101)).is_err());
        assert!(SearchText::parse("😀".repeat(100)).is_ok());
        assert!(SearchText::parse("😀".repeat(101)).is_err());
        assert!(SearchText::parse(format!("ح{}", "\u{0651}".repeat(99))).is_ok());
        assert_eq!(
            SearchText::parse(format!("ح{}", "\u{0651}".repeat(100))).unwrap_err(),
            "must be at most 100 characters"
        );
    }

    #[test]
    fn slug_lists_read_repeated_keys_and_stop_at_one_hundred_values() {
        let read = lists("era=jahili&era=abbasi&poet=PAKT").unwrap();
        assert_eq!(read.era.into_strings(), ["jahili", "abbasi"]);
        assert_eq!(read.poet.into_strings(), ["PAKT"]);
        assert!(lists("").unwrap().era.is_empty());
        let hundred = vec!["era=jahili"; 100].join("&");
        assert_eq!(lists(&hundred).unwrap().era.into_strings().len(), 100);
        let error = lists(&format!("{hundred}&era=jahili")).unwrap_err();
        assert!(error.contains("takes at most 100 values"), "{error}");
        assert!(lists("era=").is_err());
        assert!(lists("era=jahili&era=JAHILI").is_err());
    }

    #[test]
    fn the_search_types_are_the_shared_search_type_values() {
        let names: Vec<String> = [SearchTypeParam::Poems, SearchTypeParam::Poets]
            .iter()
            .map(|kind| {
                serde_json::to_value(kind)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(names, crate::constants::SEARCH_TYPE_VALUES);
    }

    #[test]
    fn each_value_type_documents_the_rule_it_checks() {
        assert_eq!(
            serde_json::to_value(PageCursor::<i32>::schema()).unwrap(),
            json!({ "type": "string", "maxLength": 512, "pattern": "^[A-Za-z0-9_-]+$" })
        );
        assert_eq!(
            serde_json::to_value(SearchText::schema()).unwrap(),
            json!({ "type": "string", "maxLength": 100 })
        );
        assert_eq!(
            serde_json::to_value(TermSlugs::schema()).unwrap(),
            json!({
                "type": "array",
                "items": { "$ref": "#/components/schemas/TransliteratedSlug" },
                "default": [],
                "maxItems": 100
            })
        );
        assert_eq!(
            serde_json::to_value(TransliteratedSlug::schema()).unwrap(),
            json!({ "type": "string", "maxLength": 64, "pattern": "^[a-z][a-z-]*$" })
        );
    }

    #[test]
    fn no_params_accepts_only_an_empty_query() {
        assert!(serde_html_form::from_str::<NoParams>("").is_ok());
        assert!(serde_html_form::from_str::<NoParams>("cursor=x").is_err());
    }
}
