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
    PG_ACCOUNTS_POOL_MAX_CONNECTIONS, PG_ACQUIRE_TIMEOUT_SECONDS, PG_LOCK_TIMEOUT_SECONDS,
    PG_POOL_MAX_CONNECTIONS, PG_STATEMENT_TIMEOUT_SECONDS,
};
use qafiyah_api::es::client::Es;
use qafiyah_api::log::stage_event;
use qafiyah_api::sentry;
use qafiyah_api::state::AppState;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

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

    let accounts_options = match config.database_url_accounts.parse::<PgConnectOptions>() {
        Ok(options) => options.options([
            (
                "statement_timeout",
                format!("{PG_STATEMENT_TIMEOUT_SECONDS}s"),
            ),
            ("lock_timeout", format!("{PG_LOCK_TIMEOUT_SECONDS}s")),
        ]),
        Err(e) => {
            eprintln!(
                "{}",
                stage_event("boot_accounts", Some(("error", e.to_string().into())))
            );
            std::process::exit(1);
        }
    };

    let accounts = match PgPoolOptions::new()
        .max_connections(PG_ACCOUNTS_POOL_MAX_CONNECTIONS)
        .acquire_timeout(Duration::from_secs(PG_ACQUIRE_TIMEOUT_SECONDS))
        .connect_with(accounts_options)
        .await
    {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!(
                "{}",
                stage_event("boot_accounts", Some(("error", e.to_string().into())))
            );
            std::process::exit(1);
        }
    };

    if let Err(e) = sqlx::migrate!("./migrations").run(&accounts).await {
        eprintln!(
            "{}",
            stage_event("migrate", Some(("error", e.to_string().into())))
        );
        std::process::exit(1);
    }

    let es = match Es::new(&config.elasticsearch_url) {
        Ok(es) => Arc::new(es),
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
    println!(
        "{}",
        stage_event("ready", Some(("port", config.port.into())))
    );

    let limiter = Arc::new(qafiyah_api::rate_limit::Limiter::default());
    qafiyah_api::rate_limit::sweeper(limiter.clone());

    let usage = Arc::new(qafiyah_api::accounts::usage::UsageRecorder::default());
    qafiyah_api::accounts::usage::flusher(usage.clone(), accounts.clone());

    let app = qafiyah_api::app(AppState {
        pg,
        accounts,
        es,
        keys: Arc::new(config.keys),
        limiter,
        key_cache: Arc::new(qafiyah_api::accounts::cache::KeyCache::default()),
        usage,
        anon_requests: config.anon_requests,
    });
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
