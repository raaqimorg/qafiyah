#![expect(
    clippy::exit,
    reason = "a failed boot exits non-zero before the server starts"
)]
#![expect(
    clippy::print_stdout,
    reason = "the ready and stopped log is one structured line on stdout"
)]
#![expect(
    clippy::print_stderr,
    reason = "fatal boot errors go to stderr before a non-zero exit"
)]

use std::sync::Arc;
use std::time::Duration;

use qafiyah_api::config::Config;
use qafiyah_api::constants::{
    PG_ACCOUNTS_POOL_MAX_CONNECTIONS, PG_ACQUIRE_TIMEOUT_SECONDS, PG_POOL_MAX_CONNECTIONS,
};
use qafiyah_api::es::client::Es;
use qafiyah_api::log::stage_event;
use qafiyah_api::sentry;
use qafiyah_api::state::AppState;

fn main() {
    let config = match Config::from_env(8787) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{}", stage_event("env", Some(("error", e.into()))));
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
            eprintln!(
                "{}",
                stage_event("runtime", Some(("error", e.to_string().into())))
            );
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
            eprintln!("{}", stage_event("boot_db", Some(("error", e.into()))));
            std::process::exit(1);
        }
    };

    if let Err(e) = qafiyah_api::db::migrate(&config.database_url_accounts).await {
        eprintln!("{}", stage_event("migrate", Some(("error", e.into()))));
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
            eprintln!(
                "{}",
                stage_event("boot_accounts", Some(("error", e.into())))
            );
            std::process::exit(1);
        }
    };

    let metrics = qafiyah_api::metrics::Metrics::default();
    let es = match Es::new(&config.elasticsearch_url, metrics.clone()) {
        Ok(es) => es,
        Err(e) => {
            eprintln!("{}", stage_event("boot_es", Some(("error", e.into()))));
            std::process::exit(1);
        }
    };

    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!(
                "{}",
                stage_event("bind", Some(("error", e.to_string().into())))
            );
            std::process::exit(1);
        }
    };

    if let Some(port) = config.metrics_port {
        let metrics_listener = match tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!(
                    "{}",
                    stage_event("bind_metrics", Some(("error", e.to_string().into())))
                );
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

    println!(
        "{}",
        stage_event("ready", Some(("port", config.port.into())))
    );

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
        Ok(()) => println!("{}", stage_event("stopped", None)),
        Err(e) => {
            eprintln!(
                "{}",
                stage_event("serve", Some(("error", e.to_string().into())))
            );
            std::process::exit(1);
        }
    }
}
