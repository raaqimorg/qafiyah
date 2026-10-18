use ipnet::IpNet;

use crate::constants::DEV_MCP_PORT;

const PRODUCTION: &str = "production";

pub struct Config {
    pub environment: String,
    pub api_url: String,
    pub api_key: Option<String>,
    pub trusted_proxies: Vec<IpNet>,
    pub allowed_hosts: Vec<String>,
    pub port: u16,
    pub metrics_port: Option<u16>,
}

fn required_in_production(name: &str, missing: bool, environment: &str) -> Result<(), String> {
    if environment == PRODUCTION && missing {
        return Err(format!("{name} is required when ENVIRONMENT=production"));
    }
    Ok(())
}

fn items(value: Option<&str>) -> impl Iterator<Item = &str> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

fn port(name: &str, value: Option<String>) -> Result<Option<u16>, String> {
    value
        .map(|value| {
            value
                .parse::<u16>()
                .map_err(|_| format!("{name} must be a port number"))
        })
        .transpose()
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    pub fn from_vars(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let environment = lookup("ENVIRONMENT").unwrap_or_default();
        let api_url = lookup("INTERNAL_API_URL")
            .filter(|url| !url.is_empty())
            .ok_or_else(|| "INTERNAL_API_URL is required".to_string())?;
        let api_key = lookup("API_KEY_MCP").filter(|key| !key.is_empty());
        let trusted_proxies = items(lookup("TRUSTED_PROXIES").as_deref())
            .map(|range| {
                range.parse::<IpNet>().map_err(|_| {
                    format!(
                        "TRUSTED_PROXIES must be a comma-separated list of CIDR ranges, not {range}"
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let allowed_hosts: Vec<String> = items(lookup("MCP_ALLOWED_HOSTS").as_deref())
            .map(str::to_string)
            .collect();
        required_in_production("API_KEY_MCP", api_key.is_none(), &environment)?;
        required_in_production("TRUSTED_PROXIES", trusted_proxies.is_empty(), &environment)?;
        required_in_production("MCP_ALLOWED_HOSTS", allowed_hosts.is_empty(), &environment)?;
        Ok(Self {
            port: port("MCP_PORT", lookup("MCP_PORT"))?.unwrap_or(DEV_MCP_PORT),
            metrics_port: port("METRICS_PORT", lookup("METRICS_PORT"))?,
            environment,
            api_url,
            api_key,
            trusted_proxies,
            allowed_hosts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::DEV_MCP_PORT;

    fn vars<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| (*v).to_string())
        }
    }

    const PRODUCTION_VARS: [(&str, &str); 5] = [
        ("ENVIRONMENT", "production"),
        ("INTERNAL_API_URL", "http://api-backend:8787"),
        ("API_KEY_MCP", "m"),
        ("TRUSTED_PROXIES", "172.27.0.0/24"),
        ("MCP_ALLOWED_HOSTS", "mcp.qafiyah.com"),
    ];

    #[test]
    fn production_refuses_each_missing_setting() {
        assert!(Config::from_vars(vars(&PRODUCTION_VARS)).is_ok());
        for missing in ["API_KEY_MCP", "TRUSTED_PROXIES", "MCP_ALLOWED_HOSTS"] {
            let pairs: Vec<(&str, &str)> = PRODUCTION_VARS
                .into_iter()
                .filter(|(name, _)| *name != missing)
                .collect();
            assert_eq!(
                Config::from_vars(vars(&pairs)).err(),
                Some(format!("{missing} is required when ENVIRONMENT=production"))
            );
            let blank: Vec<(&str, &str)> = PRODUCTION_VARS
                .into_iter()
                .map(|(name, value)| match name {
                    _ if name != missing => (name, value),
                    "API_KEY_MCP" => (name, ""),
                    _ => (name, " , "),
                })
                .collect();
            assert!(Config::from_vars(vars(&blank)).is_err(), "{missing} blank");
        }
    }

    #[test]
    fn outside_production_only_the_api_url_is_required() {
        let config = Config::from_vars(vars(&[("INTERNAL_API_URL", "http://localhost:8787")]))
            .expect("a dev config");
        assert_eq!(config.port, DEV_MCP_PORT);
        assert_eq!(config.metrics_port, None);
        assert!(config.trusted_proxies.is_empty());
        assert!(config.allowed_hosts.is_empty());
        assert_eq!(config.api_key, None);
        assert_eq!(
            Config::from_vars(vars(&[])).err(),
            Some("INTERNAL_API_URL is required".into())
        );
    }

    #[test]
    fn trusted_proxies_are_a_comma_separated_list_of_cidr_ranges() {
        let config = Config::from_vars(vars(&[
            ("INTERNAL_API_URL", "http://localhost:8787"),
            ("TRUSTED_PROXIES", "172.27.0.0/24, 172.26.0.0/24"),
        ]))
        .expect("a config");
        assert_eq!(
            config.trusted_proxies,
            vec![
                "172.27.0.0/24".parse::<IpNet>().expect("a network"),
                "172.26.0.0/24".parse::<IpNet>().expect("a network"),
            ]
        );
        for bad in ["172.27.0.0/33", "nonsense"] {
            let error = Config::from_vars(vars(&[
                ("INTERNAL_API_URL", "http://localhost:8787"),
                ("TRUSTED_PROXIES", bad),
            ]))
            .err()
            .expect("a refusal");
            assert!(error.contains("TRUSTED_PROXIES"), "{error}");
        }
    }

    #[test]
    fn allowed_hosts_split_on_commas_and_drop_blanks() {
        let config = Config::from_vars(vars(&[
            ("INTERNAL_API_URL", "http://localhost:8787"),
            ("MCP_ALLOWED_HOSTS", "mcp.qafiyah.com, ,localhost"),
        ]))
        .expect("a config");
        assert_eq!(config.allowed_hosts, vec!["mcp.qafiyah.com", "localhost"]);
    }

    #[test]
    fn the_ports_are_read_and_a_bad_one_is_refused() {
        let config = Config::from_vars(vars(&[
            ("INTERNAL_API_URL", "http://localhost:8787"),
            ("MCP_PORT", "9000"),
            ("METRICS_PORT", "9464"),
        ]))
        .expect("a config");
        assert_eq!((config.port, config.metrics_port), (9000, Some(9464)));
        for name in ["MCP_PORT", "METRICS_PORT"] {
            assert_eq!(
                Config::from_vars(vars(&[
                    ("INTERNAL_API_URL", "http://localhost:8787"),
                    (name, "nine"),
                ]))
                .err(),
                Some(format!("{name} must be a port number"))
            );
        }
    }
}
