# Search Indexer Agent Guide

This Rust binary builds the Elasticsearch indices that `apps/api` searches. It does these steps:

1. It reads poems and poets from Postgres.
2. It maps each row to a document.
3. It writes the documents in bulk into a new versioned index (`poems_v<N>`, `poets_v<N>`).
4. It force-merges the index to one segment.
5. It swaps the `poems` or `poets` alias to the new index.

The API only queries the alias, so a reindex has no visible effect on it. `docs/search.md` explains what the documents contain and why (tashkeel stripping, the `*Display` fields, `nameSort`). The mappings and analyzers are in `crates/elasticsearch/schema.json`.

## Shape

- `main.rs` reads the environment. It connects the real Elasticsearch client and a lazy Postgres connection to `reindex::bootstrap`. It prints its progress and its result as log lines, then exits with 0.
- `reindex.rs` holds every decision of the job, with no I/O of its own. It reads through the `CorpusSource` trait, and writes through the `IndexStore` trait.
  - `bootstrap` provisions the read-only Elasticsearch user that the API connects as.
  - It skips the rebuild when both aliases already hold documents, unless the rebuild is forced. In that case, it never connects to Postgres.
  - Otherwise, it rebuilds the poems, then the poets.
  - `reindex` creates the next `_v<N>` index (`next_index_name`). It streams batches by an id cursor, with refresh turned off, and reports progress through a callback every 10,000 documents. It then merges to one segment, swaps the alias, and deletes the older indices.
  - A failed write deletes only the index that it created.
  - Its tests drive it against an in-memory corpus and an in-memory cluster. They check where the alias points, and what each index holds.
- `pg.rs` is `PgCorpus`, the `CorpusSource` adapter. It does the batched streaming reads through Diesel, over the schema in `crates/corpus`.
  - A page of poems is two queries: the poems' rows, then their verses.
  - Each poem's content is its stored rows joined by a newline. A full verse keeps the `*` between its hemistichs.
- `docs.rs` maps a row to a document (`to_poem_doc`, `to_poet_doc`). It also holds `Document`, the typed poem or poet that the job gives to `IndexStore::bulk`.
- `arabic.rs` does the tashkeel stripping, and the sort folding that is derived from the schema's char filter. `arabic-text.vectors.json` is the fixture for the `matches_the_shared_vectors` test, which holds this crate's output to it.
- `es.rs` is the Elasticsearch client, which implements `IndexStore`. It does these things:
  - It creates indices, and writes in bulk. Bulk serializes each `Document`, and fails the batch if one cannot be serialized.
  - It changes the refresh interval, and runs the force merge with its own 30-minute timeout.
  - It swaps aliases, and counts the documents behind them.
  - It creates the read-only role and user for the API.
  - It creates the monitor role, which holds only the cluster `monitor` privilege, and its user for elasticsearch_exporter.
- `error.rs` is `IndexerError`. It says which side failed (the configuration, Postgres, or Elasticsearch), and keeps the message of each failure.
- `log.rs` writes one-line structured log output.

## Environment

| Variable               | Meaning                                                          |
| ---------------------- | ---------------------------------------------------------------- |
| `DATABASE_URL`         | Postgres, the read-only `qafiyah_api` role                       |
| `ELASTICSEARCH_URL`    | Elasticsearch as the `elastic` superuser (it creates the reader) |
| `ES_READER_PASSWORD`   | password to set on the read-only Elasticsearch user              |
| `ES_MONITOR_PASSWORD`  | password to set on the monitor user elasticsearch_exporter uses  |
| `SEARCH_INDEXER_FORCE` | `true` rebuilds even when the aliases already hold documents     |

## Tests

The tests are inline, next to the code that they cover.

- The `reindex.rs` tests run the whole job against an in-memory corpus and cluster. They cover the alias swap and cleanup of a rebuild, a failed write, an empty corpus, progress every 10,000 documents, and `bootstrap` when it skips, forces, and provisions the reader.
- The `es.rs` tests drive the client against a local listener in place of Elasticsearch.
- `main.rs` holds two database-backed tests. Each runs the real `reindex` for poets into a throwaway alias.
  - One sets a low `index.translog.flush_threshold_size` in its index definition, so even a small dataset is written in several segments. It checks that the alias goes to one merged segment that holds every poet.
  - The other runs a good reindex, then one whose strict mapping refuses every poet. It checks that searches stay on the first index, and that the half-built index is deleted.
  - Both return early unless `QAFIYAH_TEST_DATABASE_URL` and `QAFIYAH_TEST_ELASTICSEARCH_ADMIN_URL` are set. So plain `cargo test` needs no infrastructure. `bun run rust:test:db` sets them from the dev stack. The `db` phase of CI runs them on the sample of 96 poets.

## Deliberate, non-obvious behavior

See the "Search indexer" section of `docs/exceptions.md`.
