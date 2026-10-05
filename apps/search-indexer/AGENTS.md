# Search Indexer Agent Guide

Rust binary that builds the Elasticsearch indices `apps/api` searches. It reads poems and poets from Postgres, maps each row to a document, bulk-writes into a fresh versioned index (`poems_v<N>`, `poets_v<N>`), force-merges it to one segment, and swaps the `poems`/`poets` alias onto it. The API only ever queries the alias, so a reindex is invisible to it. What the documents contain and why (tashkeel stripping, `*Display` fields, `nameSort`) is explained in `docs/search.md`; the mappings and analyzers live in `crates/elasticsearch/schema.json`.

## Shape

- `main.rs`: reads the environment, wires the real Elasticsearch client and a lazy Postgres connection into `reindex::bootstrap`, prints its progress and outcome as log lines, then exits 0.
- `reindex.rs`: everything the job decides, with no I/O of its own. It reads through the `CorpusSource` trait and writes through the `IndexStore` trait. `bootstrap` provisions the read-only Elasticsearch user the API connects as, skips the rebuild when both aliases already hold documents (unless forced, and without ever connecting to Postgres), and otherwise rebuilds poems and then poets. `reindex` creates the next `_v<N>` index (`next_index_name`), streams batches by id cursor with refresh turned off, reports progress through a callback every 10,000 documents, merges to one segment, swaps the alias, and drops the older indices; a failed write deletes only the index it created. Its tests drive it against an in-memory corpus and an in-memory cluster and check where the alias points and what each index holds.
- `pg.rs`: `PgCorpus`, the `CorpusSource` adapter: the batched streaming reads through Diesel, over the schema in `crates/corpus`. A poem page is two queries, its rows and then their verses, and each poem's content is its hemistichs joined by `*`.
- `docs.rs`: row to document mapping (`to_poem_doc`, `to_poet_doc`), and `Document`, the typed poem or poet the job hands to `IndexStore::bulk`.
- `arabic.rs`: tashkeel stripping and the sort folding derived from the schema's char filter. `arabic-text.vectors.json` is the fixture the `matches_the_shared_vectors` test pins this crate's output to.
- `es.rs`: the Elasticsearch client, which implements `IndexStore` (create index, bulk, which serializes each `Document` and fails the batch if one cannot be serialized, refresh-interval toggling, the force merge with its own 30-minute timeout, alias swap, alias counts, and the read-only role and user for the API, and the monitor role, which holds only the cluster `monitor` privilege, and its user for elasticsearch_exporter).
- `error.rs`: `IndexerError`, which says which side failed (configuration, Postgres, or Elasticsearch) and keeps each failure's message.
- `log.rs`: one-line structured log output.

## Environment

| Variable               | Meaning                                                          |
| ---------------------- | ---------------------------------------------------------------- |
| `DATABASE_URL`         | Postgres, the read-only `qafiyah_api` role                       |
| `ELASTICSEARCH_URL`    | Elasticsearch as the `elastic` superuser (it creates the reader) |
| `ES_READER_PASSWORD`   | password to set on the read-only Elasticsearch user              |
| `ES_MONITOR_PASSWORD`  | password to set on the monitor user elasticsearch_exporter uses  |
| `SEARCH_INDEXER_FORCE` | `true` rebuilds even when the aliases already hold documents     |

## Tests

Inline, next to the code they cover. The `reindex.rs` tests run the whole job against an in-memory corpus and cluster: a rebuild's alias swap and cleanup, a failed write, an empty corpus, progress every 10,000 documents, and `bootstrap` skipping, forcing, and provisioning the reader. The `es.rs` tests drive the client against a local listener standing in for Elasticsearch. `main.rs` holds two database-backed tests that run the real `reindex` for poets into a throwaway alias. One sets `index.translog.flush_threshold_size` low in its index definition so even a small dataset is written in several segments, and checks the alias lands on one merged segment holding every poet. The other follows a good reindex with one whose strict mapping rejects every poet, and checks searches stay on the first index and the half-built one is deleted. Both return early unless `QAFIYAH_TEST_DATABASE_URL` and `QAFIYAH_TEST_ELASTICSEARCH_ADMIN_URL` are set, so plain `cargo test` needs no infrastructure; `bun run rust:test:db` sets them from the dev stack, and CI's db phase runs them on the 96-poet sample.

## Deliberate, non-obvious behavior

See the Search indexer section of `docs/exceptions.md`.
