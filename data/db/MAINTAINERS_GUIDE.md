# Maintainers Guide: Database Dumps

How and when to create a new PostgreSQL dump. Restore instructions for everyone else are in `README.md` next to this file.

## When to create a new dump

Any of: a schema change (columns, tables, indexes); a significant data-quality pass; the verse or poem count moved by more than ~1,000; or 30+ days since the last dump.

## Naming

1. Next sequence number: `ls -d data/db/*/ | xargs -n1 basename | grep -v 0000_default | sort | tail -1` (a bare `ls data/db/` also lists `keys.manifest`, which sorts after the numbered directories).
2. Create `data/db/{N+1:04d}_{DD}_{MM}_{YYYY}/` (e.g. `0032_24_09_2026`).
3. The dump file inside is `qafiyah_public_{YYYYMMDD}_{HHMMSS}.dump`.

## Create the dump

Two sets of derived rows must be fresh before `pg_dump`: `poem_relations` (`refresh_poem_relations()`, TRUNCATE + INSERT, under a minute on the full corpus; it drops and re-adds the table's two foreign keys around the insert, which locks `poems` against reads for its last ~15 seconds, so on a live database it briefly stalls every poem query) and the `*_stats` count tables (`refresh_taxonomy_stats()`, a few seconds). `docs/domain.md` explains both.

- **From the local dev database**, the usual path after an ad-hoc edit: `.claude/skills/local-db-edit/SKILL.md` is the ordered runbook. It refreshes, dumps inside the `db` container straight into the new directory, and captures a `CHANGES.md` diff of the tables you touched.
- **From a Postgres reachable over the network** (production over an SSH tunnel, for instance): `scripts/db/create-dump.sh <host>` refreshes and dumps into the current directory; move the file into the new directory. Needs `psql`/`pg_dump` whose major version is at least the server's.

## Split large dumps

GitHub blocks any single file over 100MB and warns from 50MB. `pg_dump -Fc` is already compressed, so gzip on top gains nothing. Split instead:

```bash
scripts/db/split-dump.sh data/db/{new-dir}/qafiyah_public_{timestamp}.dump
```

Files at or under 45MB stay whole. Larger ones are split in place into `.dump.part-aa`, `.part-ab`, ... and the whole file is removed. `scripts/db/init.sh` reassembles them on restore.

## Post-dump checklist

- [ ] Non-empty: `ls -lh data/db/{new-dir}/`
- [ ] Split if needed (above)
- [ ] Encrypt with a fresh, unique passphrase: `scripts/db/encrypt-dump.sh data/db/{new-dir}` (prompts unless `DUMP_KEY__{new-dir}` is set; refuses a passphrase another dump already uses, via `data/db/keys.manifest`; also encrypts `CHANGES.md` if present)
- [ ] Store the passphrase: `bun run dump:key:set` writes it to `secrets/dev.enc.env`. Keep it somewhere durable too, passphrase requests arrive at dumps@qafiyah.com.
- [ ] Restores cleanly: `bun run db:reset`, then `./scripts/dev/compose.sh exec db psql -U qafiyah -d qafiyah -c "SELECT count(*) FROM poems;"`
- [ ] Regenerate the fallback sample and its `manifest.json` from it: `scripts/db/create-fallback-dump.sh`, then `bun test scripts/smoke/fixtures.test.ts`
- [ ] Commit, adding only: `git add data/db/ && git commit -m "chore(db): add {new-dir} snapshot"`. Never rename or move an existing dump directory in the same commit (`data/README.md` explains the push failure that causes).
- [ ] Ship it: add the passphrase to `secrets/prod.enc.env` as `DUMP_KEY__{new-dir}`, then run `bun run db:reseed`. A deploy keeps the data volume, so production only picks up a new dump this way (it restores the corpus database in place and rebuilds Elasticsearch, a few minutes of API downtime, accounts untouched). Ordered steps: `.claude/skills/deploy/SKILL.md`, step 3.
