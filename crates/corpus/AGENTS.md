# Corpus Crate Agent Guide

`qafiyah-corpus`, the Diesel schema of the corpus database, shared by `apps/api` (re-exported as `db::corpus`) and `apps/search-indexer`. The tables themselves come from the `data/db/` dumps, described in `docs/domain.md`.

- `generated/diesel/corpus.gen.rs`: the `diesel print-schema` output for the corpus tables, written by `bun run db:schema` from the dev database and checked by `bun run db:schema:check`. Never edit it by hand; regenerate it after a dump that changes the corpus schema.
- `src/lib.rs`: exposes that file as `schema`.
