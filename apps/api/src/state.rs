use std::sync::Arc;

use crate::accounts::cache::KeyCache;
use crate::accounts::keys::KeyRepository;
use crate::accounts::sessions::SessionRepository;
use crate::accounts::usage::UsageRecorder;
use crate::accounts::users::UserRepository;
use crate::auth::Keys;
use crate::db::PgPool;
use crate::db::feelings::PgFeelings;
use crate::db::keys::PgKeys;
use crate::db::poems::PgPoems;
use crate::db::poets::PgPoets;
use crate::db::sessions::PgSessions;
use crate::db::taxonomy::PgTaxonomy;
use crate::db::users::PgUsers;
use crate::domain::feelings::FeelingRepository;
use crate::domain::poems::PoemRepository;
use crate::domain::poets::PoetRepository;
use crate::domain::search::SearchIndex;
use crate::domain::taxonomy::TaxonomyRepository;
use crate::es::client::Es;
use crate::metrics::Metrics;
use crate::rate_limit::{Limiter, McpLimits};

#[derive(Clone)]
pub struct AppState {
    pub poems: Arc<dyn PoemRepository>,
    pub poets: Arc<dyn PoetRepository>,
    pub taxonomy: Arc<dyn TaxonomyRepository>,
    pub feelings: Arc<dyn FeelingRepository>,
    pub search: Arc<dyn SearchIndex>,
    pub users: Arc<dyn UserRepository>,
    pub sessions: Arc<dyn SessionRepository>,
    pub api_keys: Arc<dyn KeyRepository>,
    pub keys: Arc<Keys>,
    pub limiter: Arc<Limiter>,
    pub key_cache: Arc<KeyCache>,
    pub usage: Arc<UsageRecorder>,
    pub anon_requests: u32,
    pub mcp: McpLimits,
    pub metrics: Metrics,
}

impl AppState {
    pub fn new(
        corpus: PgPool,
        accounts: PgPool,
        search: Es,
        keys: Keys,
        anon_requests: u32,
        mcp: McpLimits,
        metrics: Metrics,
    ) -> Self {
        Self {
            poems: Arc::new(PgPoems::new(corpus.clone())),
            poets: Arc::new(PgPoets::new(corpus.clone())),
            taxonomy: Arc::new(PgTaxonomy::new(corpus.clone())),
            feelings: Arc::new(PgFeelings::new(corpus)),
            search: Arc::new(search),
            users: Arc::new(PgUsers::new(accounts.clone())),
            sessions: Arc::new(PgSessions::new(accounts.clone())),
            api_keys: Arc::new(PgKeys::new(accounts)),
            keys: Arc::new(keys),
            limiter: Arc::new(Limiter::default()),
            key_cache: Arc::new(KeyCache::default()),
            usage: Arc::new(UsageRecorder::default()),
            anon_requests,
            mcp,
            metrics,
        }
    }
}
