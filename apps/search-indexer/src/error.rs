#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum IndexerError {
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Postgres(String),
    #[error("{0}")]
    Elasticsearch(String),
    #[error("indexed count overflow")]
    CountOverflow,
}
