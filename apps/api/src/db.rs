use std::time::Duration;

use deadpool::Runtime;
use diesel::pg::Pg;
use diesel::query_builder::{AstPass, Query, QueryFragment, QueryId};
use diesel::{ConnectionError, QueryResult};
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, ManagerConfig};
use diesel_async::{AsyncConnection, AsyncPgConnection, SimpleAsyncConnection};

use crate::constants::PG_STATEMENT_TIMEOUT_SECONDS;
use crate::error::AppError;

#[path = "../generated/diesel/corpus.gen.rs"]
pub mod corpus;

pub type PgPool = Pool<AsyncPgConnection>;

pub fn corpus_setup() -> String {
    format!("SET statement_timeout = '{PG_STATEMENT_TIMEOUT_SECONDS}s'")
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

pub fn present<T>(value: Option<T>) -> Result<T, AppError> {
    value.ok_or_else(|| AppError::Database("unexpected null column".to_string()))
}

pub fn int<T: TryInto<i32>>(value: T) -> Result<i32, AppError> {
    value
        .try_into()
        .map_err(|_| AppError::Database("integer out of range".to_string()))
}
