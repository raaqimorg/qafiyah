# Maintainers Guide: Database Dumps

This guide says how and when to create a new PostgreSQL dump. The restore steps for everyone else are in `README.md`, next to this file.

## When to create a new dump

Create one when any of these is true:

- The schema changed (columns, tables, indexes).
- A significant data-quality pass is complete.
- The verse count or the poem count changed by more than about 1,000.
- 30 or more days passed since the last dump.

## Naming

1. Find the newest dump: run `bun run data:pull`. Its last line names it (`using db/{dir}`). The next sequence number is one higher.
2. Create `data/db/{N+1:04d}_{DD}_{MM}_{YYYY}/` (for example, `0032_24_09_2026`).
3. Name the dump file in it `qafiyah_public_{YYYYMMDD}_{HHMMSS}.dump`.

## Create the dump

Three sets of derived rows must be current before `pg_dump`. `docs/domain.md` explains all three.

- `poems.has_tashkeel`, from `refresh_poem_tashkeel()`, in about two minutes on the full corpus. It writes only the rows whose value changes.
- `poem_relations`, from `refresh_poem_relations()`. It does a TRUNCATE and an INSERT, in under a minute on the full corpus. It drops the two foreign keys of the table around the insert, and adds them again. That locks `poems` against reads for the last 15 seconds or so. So on a live database, it briefly stops every poem query.
- the `*_stats` count tables, from `refresh_taxonomy_stats()`, in a few seconds.

There are two paths:

- **From the local dev database.** This is the usual path after a manual edit. `.claude/skills/local-db-edit/SKILL.md` is the ordered runbook. It refreshes all three sets, dumps inside the `db` container directly into the new directory, and records a `CHANGES.md` diff of the tables that you changed.
- **From a Postgres reachable over the network** (for example, production over an SSH tunnel). `scripts/db/create-dump.sh <host>` refreshes all three sets, and dumps into the current directory. Move the file into the new directory. It needs `psql` and `pg_dump` with a major version at least as new as the server's.

## Post-dump checklist

- [ ] The directory holds the dump: `ls -lh data/db/{new-dir}/`.
- [ ] It restores correctly: run `bun run db:reset`, then `./scripts/dev/compose.sh exec db psql -U qafiyah -d qafiyah -c "SELECT count(*) FROM poems;"`.
- [ ] The fallback sample and its `manifest.json` are generated again from it: run `scripts/db/create-fallback-dump.sh`, then `bun test scripts/smoke/fixtures.test.ts`. Commit them.
- [ ] It is uploaded: `bun run data:push data/db/{new-dir}`. Git ignores the folder, so the dump itself is never committed.
- [ ] It is shipped. A deploy keeps the data volume, so production gets a new dump only in this way, and only when the maintainer asks:
  1. Release the version that holds the code for the new dump into `main` (step 0 of `.claude/skills/deploy/SKILL.md`).
  2. Run `bun run db:reseed {new-dir}`. It builds the API and the indexer from `main`, downloads that dump to the server, deletes every other dump there, restores it in place, and rebuilds Elasticsearch. The API is down for a few minutes, and the accounts database is not touched.
  3. Right after it, run `bun run deploy`. This rolls out the web image, with the new filter options.

  The ordered steps are in step 3 of `.claude/skills/deploy/SKILL.md`.
