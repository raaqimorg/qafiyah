# Search Indexer Agent Guide

Rust binary that builds the Elasticsearch indices `apps/api` searches. It reads poems and poets from Postgres, maps each row to a document, bulk-writes into a fresh versioned index (`poems_v<N>`, `poets_v<N>`), force-merges it to one segment, and swaps the `poems`/`poets` alias onto it. The API only ever queries the alias, so a reindex is invisible to it. What the documents contain and why (tashkeel stripping, `*Display` fields, `nameSort`) is explained in `docs/search.md`; the mappings and analyzers live in `crates/elasticsearch/schema.json`.

## Shape

- `main.rs`: reads the environment, then `bootstrap`: provisions the read-only Elasticsearch user the API connects as, skips the rebuild when both aliases already hold documents (unless forced), otherwise runs `reindex` for poems and then poets, then exits 0.
- `pg.rs`: the batched streaming reads through Diesel, over the schema in `crates/corpus`. A poem page is two queries, its rows and then their verses, and each poem's content is its hemistichs joined by `*`.
- `docs.rs`: row to document mapping (`to_poem_doc`, `to_poet_doc`).
- `arabic.rs`: tashkeel stripping and the sort folding derived from the schema's char filter. `arabic-text.vectors.json` is the fixture the `matches_the_shared_vectors` test pins this crate's output to.
- `es.rs`: the Elasticsearch calls (create index, bulk, refresh-interval toggling, the force merge with its own 30-minute timeout, alias swap, `next_index_name` for the `_v<N>` counter).
- `log.rs`: one-line structured log output.

## Environment

| Variable               | Meaning                                                          |
| ---------------------- | ---------------------------------------------------------------- |
| `DATABASE_URL`         | Postgres, the read-only `qafiyah_api` role                       |
| `ELASTICSEARCH_URL`    | Elasticsearch as the `elastic` superuser (it creates the reader) |
| `ES_READER_PASSWORD`   | password to set on the read-only Elasticsearch user              |
| `SEARCH_INDEXER_FORCE` | `true` rebuilds even when the aliases already hold documents     |

## Tests

Inline, next to the code they cover. The `es.rs` tests drive the client against a local listener standing in for Elasticsearch. `main.rs` holds two database-backed tests that run the real `reindex` for poets into a throwaway alias. One sets `index.translog.flush_threshold_size` low in its index definition so even a small dataset is written in several segments, and checks the alias lands on one merged segment holding every poet. The other follows a good reindex with one whose strict mapping rejects every poet, and checks searches stay on the first index and the half-built one is deleted. Both return early unless `QAFIYAH_TEST_DATABASE_URL` and `QAFIYAH_TEST_ELASTICSEARCH_ADMIN_URL` are set, so plain `cargo test` needs no infrastructure; `bun run rust:test:db` sets them from the dev stack, and CI's db phase runs them on the 96-poet sample.

## Deliberate, non-obvious behavior

See the Search indexer section of `docs/exceptions.md`.
