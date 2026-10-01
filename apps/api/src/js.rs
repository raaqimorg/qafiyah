use serde::Serializer;

fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

pub fn trim(text: &str) -> &str {
    text.trim_matches(is_whitespace)
}

pub fn collapse_whitespace(text: &str) -> String {
    text.split(is_whitespace)
        .filter(|part| !part.is_empty())
        .collect::<Vec<&str>>()
        .join(" ")
}

#[expect(
    clippy::as_conversions,
    reason = "the guard above ensures the value is a whole number within the integer limit"
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "a whole f64 below the integer limit fits i64 exactly"
)]
pub fn serialize_number<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
    const INTEGER_LIMIT: f64 = 9007199254740992.0;
    if value.is_finite() && value.fract() == 0.0 && value.abs() < INTEGER_LIMIT {
        serializer.serialize_i64(*value as i64)
    } else {
        serializer.serialize_f64(*value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_the_ecmascript_set_not_the_unicode_one() {
        assert_eq!(trim("\u{FEFF}abc\u{FEFF}"), "abc");
        assert_eq!(trim("\u{0085}abc\u{0085}"), "\u{0085}abc\u{0085}");
        assert_eq!(trim("   "), "");
        assert_eq!(trim(" \t\n abc \r\n "), "abc");
    }

    #[derive(serde::Serialize)]
    struct Scored {
        #[serde(serialize_with = "serialize_number")]
        relevance: f64,
    }

    #[test]
    fn writes_whole_scores_without_a_fractional_part() {
        let json = |relevance| serde_json::to_string(&Scored { relevance }).unwrap();
        assert_eq!(json(0.0), r#"{"relevance":0}"#);
        assert_eq!(json(12.0), r#"{"relevance":12}"#);
        assert_eq!(json(12.5), r#"{"relevance":12.5}"#);
    }

    #[test]
    fn non_finite_and_unsafe_integers_never_pretend_to_be_javascript_integers() {
        let json = |relevance| serde_json::to_string(&Scored { relevance }).unwrap();
        assert_eq!(json(f64::NAN), r#"{"relevance":null}"#);
        assert_eq!(json(f64::INFINITY), r#"{"relevance":null}"#);
        assert_eq!(json(f64::NEG_INFINITY), r#"{"relevance":null}"#);
        assert_eq!(
            json(9_007_199_254_740_991.0),
            r#"{"relevance":9007199254740991}"#
        );
        assert_eq!(
            json(9_007_199_254_740_992.0),
            r#"{"relevance":9007199254740992.0}"#
        );
        assert_eq!(json(-3.0), r#"{"relevance":-3}"#);
    }

    #[test]
    fn collapses_ecmascript_whitespace_runs_to_one_space_and_trims_the_ends() {
        assert_eq!(
            collapse_whitespace("  قفا \u{00A0}\t نبك \u{FEFF}"),
            "قفا نبك"
        );
        assert_eq!(collapse_whitespace("   "), "");
    }
}
