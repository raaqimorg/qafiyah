# Elasticsearch Crate Agent Guide

`qafiyah-elasticsearch`, the code both Rust services share for Elasticsearch. `apps/search-indexer` creates indices from it; `apps/api` queries them through it. The search behavior these definitions produce is described in `docs/search.md`.

- `schema.json`: the index definitions (settings, analyzers, mappings) for `poems` and `poets`, plus the `identity` block (alias names, versioned-index prefixes, the read-only API role and user, bulk batch size). `schema.rs::load` embeds it at compile time; the inline tests pin the properties `docs/search.md` relies on (strict mappings, which fields carry `.exact`/`.stemmed`/`.autocomplete`, the letter-folding char filter). `scripts/es/snapshot.ts` reads the same file for the alias names. A mapping change needs a rebuilt index: `bun run reindex`.
- `schema.rs::folding_rules`: the `arabic_letter_folding` char filter's mappings parsed into `(from, to)` pairs, so the indexer folds `nameSort` in Rust with exactly the rules Elasticsearch applies at query time; `folding_mappings` returns the raw strings.
- `endpoint.rs`: parses `ELASTICSEARCH_URL` and trims a trailing slash, so both services build requests the same way. reqwest lifts `user:password@` out of each request URL into percent-decoded basic auth.
