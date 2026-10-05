#![expect(
    clippy::exit,
    reason = "a failed boot exits non-zero before the server starts"
)]

use std::sync::Arc;
use std::time::Duration;

use qafiyah_api::config::Config;
use qafiyah_api::constants::{
    PG_ACCOUNTS_POOL_MAX_CONNECTIONS, PG_ACQUIRE_TIMEOUT_SECONDS, PG_POOL_MAX_CONNECTIONS,
};
use qafiyah_api::es::client::Es;
use qafiyah_api::sentry;
use qafiyah_api::state::AppState;

fn main() {
    if qafiyah_api::log::init(&std::env::var("ENVIRONMENT").unwrap_or_default()).is_err() {
        std::process::exit(1);
    }
    let config = match Config::from_env(8787) {
        Ok(config) => config,
        Err(e) => {
            tracing::error!(stage = "env", error = %e);
            std::process::exit(1);
        }
    };

    let reporting = sentry::Config::from_env(config.environment.clone());
    let _guard = sentry::init(&reporting);

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
    let pg = match qafiyah_api::db::pool(
        &config.database_url,
        PG_POOL_MAX_CONNECTIONS,
        Duration::from_secs(PG_ACQUIRE_TIMEOUT_SECONDS),
        qafiyah_api::db::corpus_setup(),
    ) {
        Ok(pool) => pool,
        Err(e) => {
            tracing::error!(stage = "boot_db", error = %e);
            std::process::exit(1);
        }
    };

    if let Err(e) = qafiyah_api::db::migrate(&config.database_url_accounts).await {
        tracing::error!(stage = "migrate", error = %e);
        std::process::exit(1);
    }

    let accounts = match qafiyah_api::db::pool(
        &config.database_url_accounts,
        PG_ACCOUNTS_POOL_MAX_CONNECTIONS,
        Duration::from_secs(PG_ACQUIRE_TIMEOUT_SECONDS),
        qafiyah_api::db::accounts_setup(),
    ) {
        Ok(pool) => pool,
        Err(e) => {
            tracing::error!(stage = "boot_accounts", error = %e);
            std::process::exit(1);
        }
    };

    let metrics = qafiyah_api::metrics::Metrics::default();
    let es = match Es::new(&config.elasticsearch_url, metrics.clone()) {
        Ok(es) => es,
        Err(e) => {
            tracing::error!(stage = "boot_es", error = %e);
            std::process::exit(1);
        }
    };

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
            axum::serve(
                metrics_listener,
                qafiyah_api::metrics::router(metrics.clone()),
            )
            .into_future(),
        );
    }

    tracing::info!(stage = "ready", port = config.port);

    let state = AppState::new(
        pg,
        accounts.clone(),
        es,
        config.keys,
        config.anon_requests,
        metrics,
    );
    qafiyah_api::rate_limit::sweeper(state.limiter.clone());
    qafiyah_api::accounts::usage::flusher(
        state.usage.clone(),
        Arc::new(qafiyah_api::db::usage::PgUsage::new(accounts)),
    );

    let app = qafiyah_api::app(state);
    match qafiyah_api::serve(listener, app, qafiyah_api::shutdown_signal()).await {
        Ok(()) => tracing::info!(stage = "stopped"),
        Err(e) => {
            tracing::error!(stage = "serve", error = %e);
            std::process::exit(1);
        }
    }
}
