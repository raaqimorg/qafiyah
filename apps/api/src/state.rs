use std::sync::Arc;

use crate::accounts::cache::KeyCache;
use crate::accounts::usage::UsageRecorder;
use crate::auth::Keys;
use crate::db;
use crate::es::client::Es;
use crate::rate_limit::Limiter;

#[derive(Clone)]
pub struct AppState {
    pub pg: db::PgPool,
    pub accounts: db::PgPool,
    pub es: Arc<Es>,
    pub keys: Arc<Keys>,
    pub limiter: Arc<Limiter>,
    pub key_cache: Arc<KeyCache>,
    pub usage: Arc<UsageRecorder>,
    pub anon_requests: u32,
}
