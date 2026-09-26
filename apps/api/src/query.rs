use std::collections::{BTreeMap, HashMap};

use crate::constants::SEARCH_TYPE_VALUES;
use crate::error::AppError;

const PAGE_PARAM: &str = "page";

#[derive(Default)]
struct Entry {
    bare: Vec<String>,
    appended: Vec<String>,
    indexed: BTreeMap<usize, Vec<String>>,
    object: bool,
}

#[derive(Default)]
pub struct Query {
    entries: HashMap<String, Entry>,
}

impl Query {
    pub fn parse(raw: Option<&str>) -> Self {
        let mut query = Query::default();
        let Some(raw) = raw else {
            return query;
        };
        for (key, value) in form_urlencoded::parse(raw.as_bytes()) {
            let value = value.into_owned();
            match split_subscript(&key) {
                None => query.entry(&key).bare.push(value),
                Some((name, Subscript::Appended)) => query.entry(name).appended.push(value),
                Some((name, Subscript::Index(index))) => {
                    query
                        .entry(name)
                        .indexed
                        .entry(index)
                        .or_default()
                        .push(value);
                }
                Some((name, Subscript::Object)) => query.entry(name).object = true,
            }
        }
        query
    }

    fn entry(&mut self, name: &str) -> &mut Entry {
        self.entries.entry(name.to_string()).or_default()
    }

    fn array(&self, name: &str) -> Result<Option<Vec<String>>, AppError> {
        let Some(entry) = self.entries.get(name) else {
            return Ok(None);
        };
        if entry.object {
            return Err(AppError::BadRequest);
        }
        if !entry.indexed.is_empty() {
            if !entry.appended.is_empty() || !entry.bare.is_empty() {
                return Err(AppError::BadRequest);
            }
            let mut values = Vec::with_capacity(entry.indexed.len());
            for (expected, (index, at_index)) in entry.indexed.iter().enumerate() {
                let [only] = at_index.as_slice() else {
                    return Err(AppError::BadRequest);
                };
                if *index != expected {
                    return Err(AppError::BadRequest);
                }
                values.push(only.clone());
            }
            return Ok(Some(values));
        }
        if !entry.appended.is_empty() {
            return Ok(Some(entry.appended.clone()));
        }
        if entry.bare.is_empty() {
            return Ok(None);
        }
        Ok(Some(entry.bare.clone()))
    }

    fn scalar(&self, name: &str) -> Result<Option<&String>, AppError> {
        let Some(entry) = self.entries.get(name) else {
            return Ok(None);
        };
        if entry.object || !entry.appended.is_empty() || !entry.indexed.is_empty() {
            return Err(AppError::BadRequest);
        }
        match entry.bare.as_slice() {
            [] => Ok(None),
            [only] => Ok(Some(only)),
            _ => Err(AppError::BadRequest),
        }
    }

    pub fn first(&self, name: &str) -> Option<String> {
        self.entries.get(name)?.bare.first().cloned()
    }

    pub fn facet(
        &self,
        name: &str,
        validate: fn(&str) -> Result<&str, AppError>,
        max_values: usize,
    ) -> Result<Vec<String>, AppError> {
        let Some(values) = self.array(name)? else {
            return Ok(Vec::new());
        };
        if values.len() > max_values {
            return Err(AppError::BadRequest);
        }
        for value in &values {
            validate(value)?;
        }
        Ok(values)
    }

    pub fn types(&self) -> Result<Vec<String>, AppError> {
        let Some(types) = self.array("types")? else {
            return Ok(SEARCH_TYPE_VALUES
                .iter()
                .map(|v| (*v).to_string())
                .collect());
        };
        if types.len() > SEARCH_TYPE_VALUES.len() {
            return Err(AppError::BadRequest);
        }
        for value in &types {
            if !SEARCH_TYPE_VALUES.contains(&value.as_str()) {
                return Err(AppError::BadRequest);
            }
        }
        Ok(types)
    }

    pub fn scalar_slug(
        &self,
        name: &str,
        validate: fn(&str) -> Result<&str, AppError>,
    ) -> Result<Option<String>, AppError> {
        match self.scalar(name)? {
            None => Ok(None),
            Some(raw) => validate(raw).map(|value| Some(value.to_string())),
        }
    }

    pub fn scalar_parsed<T>(
        &self,
        name: &str,
        parse: fn(&str) -> Option<T>,
    ) -> Result<Option<T>, AppError> {
        self.scalar(name)?
            .map(|raw| parse(raw).ok_or(AppError::BadRequest))
            .transpose()
    }

    pub fn text(&self, name: &str, max_len: usize) -> Result<Option<String>, AppError> {
        match self.scalar(name)? {
            None => Ok(None),
            Some(raw) if raw.encode_utf16().count() > max_len => Err(AppError::BadRequest),
            Some(raw) => Ok(Some(raw.clone())),
        }
    }

    pub fn boolean(&self, name: &str) -> Result<bool, AppError> {
        match self.scalar(name)?.map(String::as_str) {
            None => Ok(false),
            Some("true") => Ok(true),
            Some("false") => Ok(false),
            Some(_) => Err(AppError::BadRequest),
        }
    }

    pub fn page(&self, max: u32) -> Result<u32, AppError> {
        self.named_page(PAGE_PARAM, max)
    }

    pub fn unbounded_page(&self) -> Result<u32, AppError> {
        self.page(u32::MAX)
    }

    pub fn named_page(&self, name: &str, max: u32) -> Result<u32, AppError> {
        let Some(raw) = self.scalar(name)? else {
            return Ok(1);
        };
        let mut digits = raw.chars();
        let leading_nonzero = matches!(digits.next(), Some('1'..='9'));
        if !leading_nonzero || !digits.all(|c| c.is_ascii_digit()) {
            return Err(AppError::BadRequest);
        }
        let page: u32 = raw.parse().map_err(|_| AppError::BadRequest)?;
        if page > max {
            return Err(AppError::BadRequest);
        }
        Ok(page)
    }
}

enum Subscript {
    Appended,
    Index(usize),
    Object,
}

fn split_subscript(key: &str) -> Option<(&str, Subscript)> {
    let open = key.find('[')?;
    if !key.ends_with(']') {
        return None;
    }
    let name = key.get(..open)?;
    let subscript = key.get(open.saturating_add(1)..key.len().saturating_sub(1))?;
    if subscript.is_empty() {
        return Some((name, Subscript::Appended));
    }
    Some((
        name,
        subscript
            .parse::<usize>()
            .map_or(Subscript::Object, Subscript::Index),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slug;

    fn query(raw: &str) -> Query {
        Query::parse(Some(raw))
    }

    #[test]
    fn reads_arrays_from_the_bracket_form_and_from_repeats() {
        let facet = |raw: &str| query(raw).facet("era", slug::transliterated, 100);
        assert_eq!(facet("era[]=jahili").unwrap(), ["jahili"]);
        assert_eq!(
            facet("era[]=jahili&era[]=abbasi").unwrap(),
            ["jahili", "abbasi"]
        );
        assert_eq!(
            facet("era=jahili&era=abbasi").unwrap(),
            ["jahili", "abbasi"]
        );
        assert!(facet("").unwrap().is_empty());
    }

    #[test]
    fn reads_arrays_from_the_indexed_form() {
        let facet = |raw: &str| query(raw).facet("era", slug::transliterated, 100);
        assert_eq!(facet("era[0]=jahili").unwrap(), ["jahili"]);
        assert_eq!(
            facet("era[0]=jahili&era[1]=abbasi").unwrap(),
            ["jahili", "abbasi"]
        );
        assert_eq!(
            facet("era[1]=abbasi&era[0]=jahili").unwrap(),
            ["jahili", "abbasi"]
        );
    }

    #[test]
    fn rejects_an_indexed_array_that_is_not_dense_from_zero() {
        let facet = |raw: &str| query(raw).facet("era", slug::transliterated, 100);
        assert!(facet("era[1]=jahili").is_err());
        assert!(facet("era[0]=jahili&era[2]=abbasi").is_err());
        assert!(facet("era[0]=jahili&era[0]=abbasi").is_err());
        assert!(facet("era[first]=jahili").is_err());
        assert!(facet("era[]=jahili&era[0]=abbasi").is_err());
        assert!(facet("era=jahili&era[0]=abbasi").is_err());
    }

    #[test]
    fn reads_a_single_bare_value_as_a_one_element_array() {
        assert_eq!(
            query("era=jahili")
                .facet("era", slug::transliterated, 100)
                .expect("a bare value is a one element array"),
            ["jahili"]
        );
    }

    #[test]
    fn rejects_values_that_fail_the_brand() {
        assert!(
            query("era[]=NotReal")
                .facet("era", slug::transliterated, 100)
                .is_err()
        );
        assert!(
            query("era[]=")
                .facet("era", slug::transliterated, 100)
                .is_err()
        );
        assert!(
            query("era[]=a&era[]=b")
                .facet("era", slug::transliterated, 1)
                .is_err()
        );
    }

    #[test]
    fn reads_types_and_booleans_the_way_the_contract_declares_them() {
        assert_eq!(query("").types().unwrap(), ["poems", "poets"]);
        assert_eq!(query("types[]=poets").types().unwrap(), ["poets"]);
        assert_eq!(query("types=poems").types().unwrap(), ["poems"]);
        assert_eq!(
            query("types=poems&types=poets").types().unwrap(),
            ["poems", "poets"]
        );
        assert!(
            query("types[]=poems&types[]=poets&types[]=poems")
                .types()
                .is_err()
        );
        assert!(query("types[]=books").types().is_err());
        assert!(!query("").boolean("exact").unwrap());
        assert!(query("exact=true").boolean("exact").unwrap());
        assert!(!query("exact=false").boolean("exact").unwrap());
        assert!(query("exact=1").boolean("exact").is_err());
    }

    #[test]
    fn a_parsed_scalar_is_absent_parsed_or_rejected() {
        let parse = |raw: &str| (raw == "theme").then_some(1);
        assert_eq!(query("").scalar_parsed("by", parse).unwrap(), None);
        assert_eq!(
            query("by=theme").scalar_parsed("by", parse).unwrap(),
            Some(1)
        );
        assert!(query("by=era").scalar_parsed("by", parse).is_err());
        assert!(
            query("by=theme&by=theme")
                .scalar_parsed("by", parse)
                .is_err()
        );
        assert!(query("by[]=theme").scalar_parsed("by", parse).is_err());
    }

    #[test]
    fn caps_text_by_utf16_length() {
        assert!(query("q=abc").text("q", 3).unwrap().is_some());
        assert!(query("q=abcd").text("q", 3).is_err());
        assert!(query("q[]=abc").text("q", 3).is_err());
        assert!(query("q[0]=abc").text("q", 3).is_err());
    }

    #[test]
    fn parses_the_page_parameter_like_the_contract() {
        assert_eq!(query("").page(10000).unwrap(), 1);
        assert_eq!(query("page=7").page(10000).unwrap(), 7);
        for raw in [
            "page=0",
            "page=",
            "page=01",
            "page=-1",
            "page=x",
            "page=999999",
        ] {
            assert!(query(raw).page(10000).is_err(), "should reject {raw}");
        }
        assert!(query("page=1&page=2").page(10000).is_err());
        assert!(query("page[]=2").page(10000).is_err());
    }

    #[test]
    fn parsing_never_panics_on_arbitrary_query_strings() {
        let mut rng = crate::test_support::Rng::new(11);
        let alphabet = [
            "a", "=", "&", "[", "]", "%", "0", "9", "+", "q", "page", "types", "ع", "😀", "\u{0}",
        ];
        for _ in 0..3_000 {
            let len = rng.below(24);
            let raw: String = (0..len).map(|_| rng.pick(&alphabet)).collect();
            let query = Query::parse(Some(&raw));
            let _result = query.facet(
                "era",
                slug::transliterated,
                crate::constants::MAX_FILTER_SLUGS,
            );
            let _result = query.types();
            let _result = query.text("q", crate::constants::MAX_QUERY_LENGTH);
            let _result = query.boolean("exact");
            let _result = query.page(crate::constants::SEARCH_POEMS_MAX_PAGE);
            let _result = query.unbounded_page();
            let _result = query.scalar_slug("era", slug::transliterated);
            let _result = query.first("option");
        }
    }

    #[test]
    fn text_is_capped_at_fifty_utf16_units_with_astral_and_combining_characters_at_the_edge() {
        let max = crate::constants::MAX_QUERY_LENGTH;
        assert!(
            query(&format!("q={}", "ا".repeat(50)))
                .text("q", max)
                .is_ok()
        );
        assert!(
            query(&format!("q={}", "ا".repeat(51)))
                .text("q", max)
                .is_err()
        );
        assert!(
            query(&format!("q={}", "😀".repeat(25)))
                .text("q", max)
                .is_ok()
        );
        assert!(
            query(&format!("q={}", "😀".repeat(26)))
                .text("q", max)
                .is_err()
        );
        assert!(
            query(&format!("q=ح{}", "\u{0651}".repeat(49)))
                .text("q", max)
                .is_ok()
        );
        assert!(
            query(&format!("q=ح{}", "\u{0651}".repeat(50)))
                .text("q", max)
                .is_err()
        );
    }

    #[test]
    fn facets_are_capped_at_one_hundred_values() {
        let hundred: String = (0..100)
            .map(|_| "era[]=jahili")
            .collect::<Vec<_>>()
            .join("&");
        assert_eq!(
            query(&hundred)
                .facet("era", slug::transliterated, 100)
                .expect("one hundred values")
                .len(),
            100
        );
        let over = format!("{hundred}&era[]=jahili");
        assert!(
            query(&over)
                .facet("era", slug::transliterated, 100)
                .is_err()
        );
    }

    #[test]
    fn pages_are_bounded_by_the_real_ceilings_and_reject_overflow() {
        use crate::constants::{LIST_POETS_MAX_PAGE, SEARCH_POEMS_MAX_PAGE};
        assert_eq!(
            query("page=500")
                .page(SEARCH_POEMS_MAX_PAGE)
                .expect("the last page"),
            500
        );
        assert!(query("page=501").page(SEARCH_POEMS_MAX_PAGE).is_err());
        assert_eq!(
            query("page=1666")
                .page(LIST_POETS_MAX_PAGE)
                .expect("the last page"),
            1666
        );
        assert!(query("page=1667").page(LIST_POETS_MAX_PAGE).is_err());
        assert_eq!(
            query("page=4294967295").unbounded_page().expect("u32::MAX"),
            u32::MAX
        );
        assert!(query("page=4294967296").unbounded_page().is_err());
        assert!(query("page=99999999999999999999").unbounded_page().is_err());
        for raw in [
            "page=+5",
            "page=007",
            "page=5.0",
            "page=1e2",
            "page=0x10",
            "page=%205%20",
            "page=٢",
            "page=-0",
        ] {
            assert!(query(raw).unbounded_page().is_err(), "{raw}");
        }
    }
}
