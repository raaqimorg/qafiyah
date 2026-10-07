# Corpus Crate Agent Guide

`qafiyah-corpus` is the Diesel schema of the corpus database. `apps/api` (which re-exports it as `db::corpus`) and `apps/search-indexer` share it. The tables come from the `data/db/` dumps, which `docs/domain.md` describes.

- `generated/diesel/corpus.gen.rs` is the output of `diesel print-schema` for the corpus tables. `bun run db:schema` writes it from the dev database, and `bun run db:schema:check` checks it. Never edit it by hand. Generate it again after a dump that changes the corpus schema.
- `src/lib.rs` exposes that file as `schema`.
