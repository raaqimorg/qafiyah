use std::time::Duration;

use deadpool::Runtime;
use deadpool::managed::PoolError;
use diesel::pg::Pg;
use diesel::query_builder::{AstPass, Query, QueryFragment, QueryId};
use diesel::sql_types::{Interval, Nullable, Text, Timestamptz};
use diesel::{Connection, ConnectionError, QueryResult};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use diesel_async::pooled_connection::deadpool::{Object, Pool};
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, ManagerConfig};
use diesel_async::{AsyncConnection, AsyncPgConnection, SimpleAsyncConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::constants::{
    PG_CONNECT_RETRY_INTERVAL_MILLISECONDS, PG_LOCK_TIMEOUT_SECONDS, PG_STATEMENT_TIMEOUT_SECONDS,
};
use crate::domain::StoreError;

pub use qafiyah_corpus::schema as corpus;

#[path = "../../generated/diesel/accounts.gen.rs"]
pub mod accounts_schema;
pub mod keys;
pub mod poems;
pub mod poets;
pub mod sessions;
pub mod taxonomy;
pub mod usage;
pub mod users;

#[diesel::declare_sql_function]
extern "SQL" {
    fn lower(value: Text) -> Text;
    fn coalesce(value: Nullable<Text>, fallback: Nullable<Text>) -> Nullable<Text>;
    fn date_trunc(field: Text, value: Timestamptz) -> Timestamptz;
    fn date_add(value: Timestamptz, step: Interval) -> Timestamptz;
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[derive(Clone)]
pub struct PgPool {
    pool: Pool<AsyncPgConnection>,
    connect_retry: Duration,
}

impl PgPool {
    pub async fn get(
        &self,
    ) -> Result<Object<AsyncPgConnection>, PoolError<diesel_async::pooled_connection::PoolError>>
    {
        let deadline = tokio::time::Instant::now().checked_add(self.connect_retry);
        loop {
            match self.pool.get().await {
                Err(PoolError::Backend(_))
                    if deadline.is_some_and(|end| tokio::time::Instant::now() < end) =>
                {
                    tokio::time::sleep(Duration::from_millis(
                        PG_CONNECT_RETRY_INTERVAL_MILLISECONDS,
                    ))
                    .await;
                }
                result => return result,
            }
        }
    }
}

pub fn corpus_setup() -> String {
    format!("SET statement_timeout = '{PG_STATEMENT_TIMEOUT_SECONDS}s'")
}

pub fn accounts_setup() -> String {
    format!(
        "SET statement_timeout = '{PG_STATEMENT_TIMEOUT_SECONDS}s'; SET lock_timeout = '{PG_LOCK_TIMEOUT_SECONDS}s'"
    )
}

pub async fn migrate(url: &str) -> Result<(), String> {
    let url = url.to_string();
    tokio::task::spawn_blocking(move || {
        let mut conn = <AsyncConnectionWrapper<AsyncPgConnection> as Connection>::establish(&url)
            .map_err(|error| error.to_string())?;
        conn.run_pending_migrations(MIGRATIONS)
            .map(|_| ())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

// Sent unnamed so Postgres plans it for its ids; a cached generic plan walks every poem.
pub struct Uncached<Q>(pub Q);

impl<Q: Query> Query for Uncached<Q> {
    type SqlType = Q::SqlType;
}

impl<Q> QueryId for Uncached<Q> {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<Q: QueryFragment<Pg>> QueryFragment<Pg> for Uncached<Q> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        self.0.walk_ast(out.reborrow())
    }
}

pub fn pool(
    url: &str,
    max_size: usize,
    wait: Duration,
    setup: String,
    connect_retry: Duration,
) -> Result<PgPool, String> {
    let mut config = ManagerConfig::<AsyncPgConnection>::default();
    config.custom_setup = Box::new(move |url| {
        let setup = setup.clone();
        Box::pin(async move {
            let mut conn = AsyncPgConnection::establish(url).await?;
            conn.batch_execute(&setup)
                .await
                .map_err(|error| ConnectionError::BadConnection(error.to_string()))?;
            Ok(conn)
        })
    });
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(url, config);
    let pool = Pool::builder(manager)
        .max_size(max_size)
        .wait_timeout(Some(wait))
        .create_timeout(Some(wait))
        .runtime(Runtime::Tokio1)
        .build()
        .map_err(|error| error.to_string())?;
    Ok(PgPool {
        pool,
        connect_retry,
    })
}

pub fn present<T>(value: Option<T>) -> Result<T, StoreError> {
    value.ok_or_else(|| StoreError::Database("unexpected null column".to_string()))
}

pub fn int<T: TryInto<i32>>(value: T) -> Result<i32, StoreError> {
    value
        .try_into()
        .map_err(|_| StoreError::Database("integer out of range".to_string()))
}

impl From<diesel::result::Error> for StoreError {
    fn from(error: diesel::result::Error) -> Self {
        StoreError::Database(error.to_string())
    }
}

impl From<PoolError<diesel_async::pooled_connection::PoolError>> for StoreError {
    fn from(error: PoolError<diesel_async::pooled_connection::PoolError>) -> Self {
        match error {
            PoolError::Timeout(_) | PoolError::Backend(_) | PoolError::Closed => {
                StoreError::Unavailable(error.to_string())
            }
            PoolError::NoRuntimeSpecified | PoolError::PostCreateHook(_) => {
                StoreError::Database(error.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use deadpool::managed::PoolError;

    use super::pool;

    const REFUSING_POSTGRES: &str = "postgres://nobody:nothing@127.0.0.1:1/nothing";

    #[tokio::test]
    async fn a_refused_connection_is_tried_again_until_the_retry_window_ends() {
        let pool = pool(
            REFUSING_POSTGRES,
            1,
            Duration::from_millis(250),
            String::new(),
            Duration::from_millis(400),
        )
        .unwrap();
        let started = Instant::now();
        let result = pool.get().await;
        assert!(matches!(result, Err(PoolError::Backend(_))));
        assert!(started.elapsed() >= Duration::from_millis(400));
    }

    #[tokio::test]
    async fn a_pool_without_a_retry_window_fails_at_once() {
        let pool = pool(
            REFUSING_POSTGRES,
            1,
            Duration::from_millis(250),
            String::new(),
            Duration::ZERO,
        )
        .unwrap();
        let started = Instant::now();
        let result = pool.get().await;
        assert!(matches!(result, Err(PoolError::Backend(_))));
        assert!(started.elapsed() < Duration::from_millis(400));
    }
}
