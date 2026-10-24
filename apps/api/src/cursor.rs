use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::domain::paging::Cursor;
use crate::domain::search::PoetPosition;

pub trait Key: Sized {
    type Wire: Serialize + DeserializeOwned;

    fn into_wire(self) -> Self::Wire;
    fn from_wire(wire: Self::Wire) -> Self;
}

impl Key for i32 {
    type Wire = Self;

    fn into_wire(self) -> Self {
        self
    }

    fn from_wire(wire: Self) -> Self {
        wire
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PoetKey {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    score: Option<f64>,
    poems_count: i64,
    name_sort: String,
    id: i32,
}

impl Key for PoetPosition {
    type Wire = PoetKey;

    fn into_wire(self) -> PoetKey {
        PoetKey {
            score: self.score,
            poems_count: self.poems_count,
            name_sort: self.name_sort,
            id: self.id,
        }
    }

    fn from_wire(wire: PoetKey) -> Self {
        Self {
            score: wire.score,
            poems_count: wire.poems_count,
            name_sort: wire.name_sort,
            id: wire.id,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
enum Wire<W> {
    After(W),
    Before(W),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Offset {
    offset: u32,
}

fn to_text<T: Serialize>(value: &T) -> String {
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap_or_default())
}

fn from_text<T: DeserializeOwned>(raw: &str) -> Option<T> {
    let bytes = URL_SAFE_NO_PAD.decode(raw).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn encode<K: Key>(cursor: Cursor<K>) -> String {
    to_text(&match cursor {
        Cursor::After(key) => Wire::After(key.into_wire()),
        Cursor::Before(key) => Wire::Before(key.into_wire()),
    })
}

pub fn decode<K: Key>(raw: &str) -> Option<Cursor<K>> {
    Some(match from_text::<Wire<K::Wire>>(raw)? {
        Wire::After(wire) => Cursor::After(K::from_wire(wire)),
        Wire::Before(wire) => Cursor::Before(K::from_wire(wire)),
    })
}

pub fn encode_offset(offset: u32) -> String {
    to_text(&Offset { offset })
}

pub fn decode_offset(raw: &str) -> Option<u32> {
    from_text::<Offset>(raw).map(|wire| wire.offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cursor_is_url_safe_json_that_reads_back_as_itself() {
        assert_eq!(encode(Cursor::After(1264)), "eyJhZnRlciI6MTI2NH0");
        assert_eq!(
            decode::<i32>("eyJhZnRlciI6MTI2NH0"),
            Some(Cursor::After(1264))
        );
        assert_eq!(decode(&encode(Cursor::Before(7))), Some(Cursor::Before(7)));
        assert_eq!(decode_offset(&encode_offset(40)), Some(40));
        let poet = PoetPosition {
            score: Some(2.5),
            poems_count: 12,
            name_sort: "احمد".into(),
            id: 7,
        };
        let raw = encode(Cursor::Before(poet.clone()));
        assert!(
            raw.bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
        );
        assert_eq!(decode(&raw), Some(Cursor::Before(poet)));
    }

    #[test]
    fn anything_but_a_cursor_of_the_right_kind_is_refused() {
        for raw in ["", "1264", "eyJhZnRlciI6MTI2NH0=", "not base64!", "e30"] {
            assert_eq!(decode::<i32>(raw), None, "{raw}");
        }
        assert_eq!(
            decode::<i32>(&to_text(&serde_json::json!({ "after": "x" }))),
            None
        );
        assert_eq!(
            decode::<i32>(&to_text(&serde_json::json!({ "after": 1, "x": 2 }))),
            None
        );
        assert_eq!(decode::<PoetPosition>(&encode(Cursor::After(5))), None);
        assert_eq!(decode_offset(&encode(Cursor::After(5))), None);
        assert_eq!(decode::<i32>(&encode_offset(5)), None);
    }
}
