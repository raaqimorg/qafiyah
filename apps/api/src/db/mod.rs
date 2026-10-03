use std::time::Duration;

use deadpool::Runtime;
use diesel::pg::Pg;
use diesel::query_builder::{AstPass, Query, QueryFragment, QueryId};
use diesel::sql_types::{Interval, Nullable, Text, Timestamptz};
use diesel::{Connection, ConnectionError, QueryResult};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, ManagerConfig};
use diesel_async::{AsyncConnection, AsyncPgConnection, SimpleAsyncConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::constants::{PG_LOCK_TIMEOUT_SECONDS, PG_STATEMENT_TIMEOUT_SECONDS};
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

pub type PgPool = Pool<AsyncPgConnection>;

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

pub fn pool(url: &str, max_size: usize, wait: Duration, setup: String) -> Result<PgPool, String> {
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
    Pool::builder(manager)
        .max_size(max_size)
        .wait_timeout(Some(wait))
        .create_timeout(Some(wait))
        .runtime(Runtime::Tokio1)
        .build()
        .map_err(|error| error.to_string())
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

impl From<deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>> for StoreError {
    fn from(
        error: deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>,
    ) -> Self {
        StoreError::Database(error.to_string())
    }
}
