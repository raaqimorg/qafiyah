use subtle::ConstantTimeEq;

pub struct Keys {
    internal: Option<String>,
    full: Option<String>,
    mcp: Option<String>,
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    a.len() == b.len() && bool::from(a.ct_eq(b))
}

fn matches(key: &str, expected: Option<&String>) -> bool {
    expected.is_some_and(|expected| !expected.is_empty() && constant_time_eq(key, expected))
}

impl Keys {
    pub fn new(internal: Option<String>, full: Option<String>) -> Self {
        Self {
            internal,
            full,
            mcp: None,
        }
    }

    pub fn with_mcp(self, mcp: Option<String>) -> Self {
        Self { mcp, ..self }
    }

    pub fn is_unlimited(&self, key: Option<&str>) -> bool {
        key.is_some_and(|key| {
            matches(key, self.internal.as_ref()) || matches(key, self.full.as_ref())
        })
    }

    pub fn is_internal(&self, key: Option<&str>) -> bool {
        key.is_some_and(|key| matches(key, self.internal.as_ref()))
    }

    pub fn is_mcp(&self, key: Option<&str>) -> bool {
        key.is_some_and(|key| matches(key, self.mcp.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> Keys {
        Keys::new(Some("internal".into()), Some("full".into()))
    }

    #[test]
    fn only_the_internal_and_full_keys_are_unlimited() {
        let keys = keys();
        assert!(keys.is_unlimited(Some("internal")));
        assert!(keys.is_unlimited(Some("full")));
        assert!(!keys.is_unlimited(Some("web")));
        assert!(!keys.is_unlimited(Some("wrong")));
        assert!(!keys.is_unlimited(None));
    }

    #[test]
    fn an_unconfigured_key_never_grants_unlimited_access() {
        let keys = Keys::new(None, Some(String::new()));
        assert!(!keys.is_unlimited(Some("")));
        assert!(!keys.is_unlimited(Some("anything")));
    }

    #[test]
    fn only_the_internal_key_is_internal() {
        let keys = keys();
        assert!(keys.is_internal(Some("internal")));
        assert!(!keys.is_internal(Some("full")));
        assert!(!keys.is_internal(Some("wrong")));
        assert!(!keys.is_internal(None));
    }

    #[test]
    fn only_the_mcp_key_is_mcp_and_it_is_neither_unlimited_nor_internal() {
        let keys = keys().with_mcp(Some("mcp".into()));
        assert!(keys.is_mcp(Some("mcp")));
        assert!(!keys.is_mcp(Some("internal")));
        assert!(!keys.is_mcp(Some("full")));
        assert!(!keys.is_mcp(None));
        assert!(!keys.is_unlimited(Some("mcp")));
        assert!(!keys.is_internal(Some("mcp")));
    }

    #[test]
    fn an_unconfigured_mcp_key_never_matches() {
        assert!(!keys().is_mcp(Some("anything")));
        assert!(!keys().with_mcp(Some(String::new())).is_mcp(Some("")));
    }

    #[test]
    fn an_unconfigured_internal_key_never_grants_internal_access() {
        let keys = Keys::new(None, Some("full".into()));
        assert!(!keys.is_internal(Some("")));
        assert!(!keys.is_internal(Some("full")));
        assert!(!keys.is_internal(Some("anything")));
    }
}
