use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::json;
use utoipa::ToSchema;
use utoipa::openapi::schema::{ArrayBuilder, ObjectBuilder, Schema, Type};
use utoipa::openapi::{Ref, RefOr};

use crate::constants::{MAX_FILTER_SLUGS, MAX_QUERY_LENGTH, MAX_TERM_SLUG_LENGTH};

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

#[derive(Debug, Clone, Copy)]
pub struct Page<const MAX: u32>(u32);

pub type AnyPage = Page<{ u32::MAX }>;

impl<const MAX: u32> Page<MAX> {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let canonical = raw.starts_with(|first: char| ('1'..='9').contains(&first))
            && raw.bytes().all(|byte| byte.is_ascii_digit());
        match raw.parse::<u32>() {
            Ok(page) if canonical && page <= MAX => Ok(Self(page)),
            _ => Err(format!(
                "must be a whole number from 1 to {MAX}, without a sign or leading zeros"
            )),
        }
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl<'de, const MAX: u32> Deserialize<'de> for Page<MAX> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl<const MAX: u32> utoipa::PartialSchema for Page<MAX> {
    fn schema() -> RefOr<Schema> {
        ObjectBuilder::new()
            .schema_type(Type::String)
            .pattern(Some("^[1-9][0-9]*$"))
            .into()
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
    /// Page number as a 1-based integer string, 45,000 slugs a page. Minimum 1.
    #[param(inline, example = "1")]
    pub page: Option<AnyPage>,
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
    fn a_page_is_a_canonical_whole_number_up_to_its_maximum() {
        assert_eq!(Page::<500>::parse("7").unwrap().get(), 7);
        assert_eq!(Page::<500>::parse("500").unwrap().get(), 500);
        assert_eq!(
            Page::<500>::parse("501").unwrap_err(),
            "must be a whole number from 1 to 500, without a sign or leading zeros"
        );
        assert_eq!(AnyPage::parse("4294967295").unwrap().get(), u32::MAX);
        for raw in [
            "",
            "0",
            "01",
            "007",
            "-0",
            "-1",
            "+5",
            "5.0",
            "1e2",
            "0x10",
            " 5 ",
            "٢",
            "x",
            "4294967296",
            "99999999999999999999",
        ] {
            assert!(AnyPage::parse(raw).is_err(), "{raw}");
        }
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
            serde_json::to_value(Page::<500>::schema()).unwrap(),
            json!({ "type": "string", "pattern": "^[1-9][0-9]*$" })
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
        assert!(serde_html_form::from_str::<NoParams>("page=2").is_err());
    }
}
