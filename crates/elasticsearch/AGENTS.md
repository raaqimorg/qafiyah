# Elasticsearch Crate Agent Guide

`qafiyah-elasticsearch` is the Elasticsearch code that both Rust services share. `apps/search-indexer` creates indices from it, and `apps/api` queries them through it. `docs/search.md` describes the search behavior that these definitions produce.

- `schema.json` holds the index definitions (settings, analyzers, and mappings) for `poems` and `poets`. It also holds the `identity` block: the alias names, the prefixes of the versioned indices, the read-only API role and user, and the bulk batch size.
  - `schema.rs::load` embeds it at compile time.
  - The inline tests check the properties that `docs/search.md` relies on: strict mappings, which fields have `.exact`, `.stemmed`, or `.autocomplete`, and the letter-folding char filter.
  - `scripts/es/snapshot.ts` reads the same file for the alias names.
  - A mapping change needs a rebuilt index: `bun run reindex`.
- `schema.rs::folding_rules` parses the mappings of the `arabic_letter_folding` char filter into `(from, to)` pairs. So the indexer folds `nameSort` in Rust with exactly the rules that Elasticsearch applies at query time. `folding_mappings` returns the raw strings.
- `endpoint.rs` parses `ELASTICSEARCH_URL`, and removes a final slash. So both services build requests in the same way. reqwest takes `user:password@` out of each request URL, and uses it as percent-decoded basic auth.
