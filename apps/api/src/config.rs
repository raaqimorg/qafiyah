use crate::auth::Keys;
use crate::constants::{DEFAULT_ANON_REQUESTS, DEV_ANON_REQUESTS, UNKNOWN_ENVIRONMENT};

pub struct Config {
    pub database_url: String,
    pub database_url_accounts: String,
    pub elasticsearch_url: String,
    pub port: u16,
    pub metrics_port: Option<u16>,
    pub environment: String,
    pub keys: Keys,
    pub anon_requests: u32,
}

const PRODUCTION: &str = "production";

fn required_in_production(
    name: &str,
    value: Option<String>,
    environment: &str,
) -> Result<Option<String>, String> {
    if environment == PRODUCTION && value.as_deref().is_none_or(str::is_empty) {
        return Err(format!("{name} is required when ENVIRONMENT=production"));
    }
    Ok(value)
}

fn anon_requests(configured: Option<&str>, environment: &str) -> u32 {
    if let Some(value) = configured.and_then(|v| v.parse().ok()) {
        return value;
    }
    if environment == PRODUCTION {
        DEFAULT_ANON_REQUESTS
    } else {
        DEV_ANON_REQUESTS
    }
}

impl Config {
    pub fn from_env(default_port: u16) -> Result<Self, String> {
        Self::from_vars(|name| std::env::var(name).ok(), default_port)
    }

    pub fn from_vars(
        lookup: impl Fn(&str) -> Option<String>,
        default_port: u16,
    ) -> Result<Self, String> {
        let environment = lookup("ENVIRONMENT").unwrap_or_else(|| UNKNOWN_ENVIRONMENT.to_string());
        let internal_key =
            required_in_production("API_KEY_INTERNAL", lookup("API_KEY_INTERNAL"), &environment)?;
        let full_key =
            required_in_production("API_KEY_FULL", lookup("API_KEY_FULL"), &environment)?;
        let required = |name: &str| lookup(name).ok_or_else(|| format!("{name} is required"));
        let metrics_port = lookup("METRICS_PORT")
            .map(|value| {
                value
                    .parse::<u16>()
                    .map_err(|_| "METRICS_PORT must be a port number".to_string())
            })
            .transpose()?;
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            database_url_accounts: required("DATABASE_URL_ACCOUNTS")?,
            elasticsearch_url: required("ELASTICSEARCH_URL")?,
            port: lookup("PORT")
                .and_then(|v| v.parse().ok())
                .unwrap_or(default_port),
            metrics_port,
            keys: Keys::new(internal_key, full_key),
            anon_requests: anon_requests(lookup("ANON_REQUESTS").as_deref(), &environment),
            environment,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_refuses_a_missing_or_empty_key() {
        assert!(required_in_production("K", None, PRODUCTION).is_err());
        assert!(required_in_production("K", Some(String::new()), PRODUCTION).is_err());
        assert_eq!(
            required_in_production("K", Some("key".into()), PRODUCTION),
            Ok(Some("key".into()))
        );
    }

    #[test]
    fn outside_production_a_missing_key_is_allowed() {
        assert_eq!(required_in_production("K", None, "development"), Ok(None));
    }

    #[test]
    fn production_gets_the_strict_default_and_dev_does_not() {
        assert_eq!(anon_requests(None, PRODUCTION), DEFAULT_ANON_REQUESTS);
        assert_eq!(anon_requests(None, "development"), DEV_ANON_REQUESTS);
        assert_eq!(anon_requests(None, UNKNOWN_ENVIRONMENT), DEV_ANON_REQUESTS);
    }

    #[test]
    fn an_explicit_value_wins_in_every_environment() {
        assert_eq!(anon_requests(Some("7"), PRODUCTION), 7);
        assert_eq!(anon_requests(Some("7"), "development"), 7);
    }

    #[test]
    fn an_unparsable_value_falls_back_to_the_environment_default() {
        assert_eq!(
            anon_requests(Some("many"), PRODUCTION),
            DEFAULT_ANON_REQUESTS
        );
    }

    fn vars<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn every_url_is_required_and_the_port_defaults() {
        let config = Config::from_vars(
            vars(&[
                ("DATABASE_URL", "postgres://a"),
                ("DATABASE_URL_ACCOUNTS", "postgres://b"),
                ("ELASTICSEARCH_URL", "http://c"),
            ]),
            8787,
        )
        .expect("a dev config");
        assert_eq!(config.port, 8787);
        assert_eq!(config.environment, UNKNOWN_ENVIRONMENT);
        assert_eq!(config.anon_requests, DEV_ANON_REQUESTS);
        assert!(!config.keys.is_unlimited(Some("anything")));
        for missing in ["DATABASE_URL", "DATABASE_URL_ACCOUNTS", "ELASTICSEARCH_URL"] {
            let pairs: Vec<(&str, &str)> = [
                ("DATABASE_URL", "x"),
                ("DATABASE_URL_ACCOUNTS", "y"),
                ("ELASTICSEARCH_URL", "z"),
            ]
            .into_iter()
            .filter(|(n, _)| *n != missing)
            .collect();
            let error = Config::from_vars(vars(&pairs), 1).err().expect("an error");
            assert_eq!(error, format!("{missing} is required"));
        }
    }

    #[test]
    fn production_requires_both_keys_and_applies_the_strict_anonymous_default() {
        let base = [
            ("DATABASE_URL", "x"),
            ("DATABASE_URL_ACCOUNTS", "y"),
            ("ELASTICSEARCH_URL", "z"),
            ("ENVIRONMENT", "production"),
        ];
        assert_eq!(
            Config::from_vars(vars(&base), 1).err(),
            Some("API_KEY_INTERNAL is required when ENVIRONMENT=production".into())
        );
        let mut with_internal = base.to_vec();
        with_internal.push(("API_KEY_INTERNAL", "i"));
        assert_eq!(
            Config::from_vars(vars(&with_internal), 1).err(),
            Some("API_KEY_FULL is required when ENVIRONMENT=production".into())
        );
        with_internal.push(("API_KEY_FULL", "f"));
        with_internal.push(("PORT", "not-a-port"));
        let config = Config::from_vars(vars(&with_internal), 8787).expect("a prod config");
        assert_eq!(config.anon_requests, DEFAULT_ANON_REQUESTS);
        assert_eq!(config.port, 8787, "an unparsable PORT falls back");
        assert!(config.keys.is_internal(Some("i")) && config.keys.is_unlimited(Some("f")));
    }

    #[test]
    fn an_explicit_anonymous_ceiling_and_port_are_honored() {
        let config = Config::from_vars(
            vars(&[
                ("DATABASE_URL", "x"),
                ("DATABASE_URL_ACCOUNTS", "y"),
                ("ELASTICSEARCH_URL", "z"),
                ("ANON_REQUESTS", "40"),
                ("PORT", "9000"),
            ]),
            1,
        )
        .expect("a config");
        assert_eq!(config.anon_requests, 40);
        assert_eq!(config.port, 9000);
    }

    #[test]
    fn the_metrics_port_is_optional_but_never_silently_wrong() {
        let base = [
            ("DATABASE_URL", "x"),
            ("DATABASE_URL_ACCOUNTS", "y"),
            ("ELASTICSEARCH_URL", "z"),
        ];
        let unset = Config::from_vars(vars(&base), 1).expect("a config");
        assert_eq!(unset.metrics_port, None);
        let mut set = base.to_vec();
        set.push(("METRICS_PORT", "9464"));
        assert_eq!(
            Config::from_vars(vars(&set), 1)
                .expect("a config")
                .metrics_port,
            Some(9464)
        );
        let mut bad = base.to_vec();
        bad.push(("METRICS_PORT", "nine"));
        assert_eq!(
            Config::from_vars(vars(&bad), 1).err(),
            Some("METRICS_PORT must be a port number".into())
        );
    }
}
