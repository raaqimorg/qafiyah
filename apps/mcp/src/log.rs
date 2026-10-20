use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::EnvFilter;

const PRODUCTION: &str = "production";

pub fn default_directives(environment: &str) -> &'static str {
    if environment == PRODUCTION {
        "warn,rmcp=error"
    } else {
        "warn,qafiyah_mcp=debug"
    }
}

pub fn init(environment: &str) -> Result<(), SetGlobalDefaultError> {
    let directives = std::env::var("RUST_LOG")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_directives(environment).to_string());
    let subscriber = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_target(false)
        .with_env_filter(EnvFilter::new(directives))
        .with_writer(std::io::stdout)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_logs_only_the_apps_warnings_and_every_error() {
        assert_eq!(default_directives("production"), "warn,rmcp=error");
        assert_eq!(default_directives("development"), "warn,qafiyah_mcp=debug");
        assert_eq!(default_directives(""), "warn,qafiyah_mcp=debug");
    }
}
