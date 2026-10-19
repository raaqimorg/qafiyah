#![expect(
    clippy::exit,
    reason = "a failed boot exits non-zero before the server starts"
)]

use std::sync::Arc;
use std::time::Duration;

use qafiyah_mcp::api::Api;
use qafiyah_mcp::config::Config;
use qafiyah_mcp::constants::{API_CALL_TIMEOUT_SECONDS, MAX_POEM_CHARS};
use qafiyah_mcp::metrics::Metrics;
use qafiyah_mcp::tools::Catalog;

fn main() {
    if qafiyah_mcp::log::init(&std::env::var("ENVIRONMENT").unwrap_or_default()).is_err() {
        std::process::exit(1);
    }
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            tracing::error!(stage = "env", error = %e);
            std::process::exit(1);
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            tracing::error!(stage = "runtime", error = %e);
            std::process::exit(1);
        }
    };
    runtime.block_on(serve(config));
}

async fn serve(config: Config) {
    let metrics = Metrics::default();
    let api = match Api::new(
        &config.api_url,
        config.api_key.clone(),
        Duration::from_secs(API_CALL_TIMEOUT_SECONDS),
    ) {
        Ok(api) => api,
        Err(e) => {
            tracing::error!(stage = "boot_api", error = %e);
            std::process::exit(1);
        }
    };
    let catalog = Catalog::new(
        api,
        Arc::from(config.trusted_proxies.clone()),
        metrics.clone(),
        MAX_POEM_CHARS,
    );
    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(stage = "bind", error = %e);
            std::process::exit(1);
        }
    };

    if let Some(port) = config.metrics_port {
        let metrics_listener = match tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
            Ok(listener) => listener,
            Err(e) => {
                tracing::error!(stage = "bind_metrics", error = %e);
                std::process::exit(1);
            }
        };
        tokio::spawn(
            axum::serve(metrics_listener, qafiyah_mcp::metrics::router(metrics)).into_future(),
        );
    }

    tracing::info!(stage = "ready", port = config.port);

    let app = qafiyah_mcp::app(catalog, &config.allowed_hosts);
    match qafiyah_mcp::serve(listener, app, qafiyah_mcp::shutdown_signal()).await {
        Ok(()) => tracing::info!(stage = "stopped"),
        Err(e) => {
            tracing::error!(stage = "serve", error = %e);
            std::process::exit(1);
        }
    }
}
