pub const FOUR_LETTER_PATTERN: &str = "^[a-zA-Z]{4}$";
pub const TERM_PATTERN: &str = "^[a-z][a-z-]*$";

pub fn is_poem_slug(slug: &str) -> bool {
    slug.len() == 4 && slug.bytes().all(|byte| byte.is_ascii_alphabetic())
}

pub fn is_term_slug(slug: &str) -> bool {
    let mut bytes = slug.bytes();
    bytes.next().is_some_and(|first| first.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_poem_slug_is_four_ascii_letters() {
        for good in ["Abcd", "abcd", "ZZZZ"] {
            assert!(is_poem_slug(good), "{good}");
        }
        for bad in [
            "abc",
            "abcde",
            "ab1d",
            "../../account/sessions",
            "abc/",
            "",
            "قصيد",
        ] {
            assert!(!is_poem_slug(bad), "{bad}");
        }
    }

    #[test]
    fn a_term_slug_is_lowercase_letters_and_hyphens_starting_with_a_letter() {
        for good in ["tawil", "poem-types", "a"] {
            assert!(is_term_slug(good), "{good}");
        }
        for bad in ["", "-a", "Tawil", "a/b", "..", "a_b", "a b"] {
            assert!(!is_term_slug(bad), "{bad}");
        }
    }
}
