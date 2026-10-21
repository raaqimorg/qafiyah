# Database Dumps

These are PostgreSQL custom-format (`pg_dump -Fc`) snapshots of the `public` schema.

## Where they are

The dumps live in the private R2 bucket `qafiyah-data`, under `db/`, not in git. They are public, and dedicated to the public domain under [CC0 1.0](../LICENSE). `../README.md` says how to get a read-only token, and how to download a dump.

If you do not want to wait for a token, there are two options:

- `data/db/0000_default/` is committed, and needs no token (see "Fallback sample dataset" below).
- The read-only public API at <https://api.qafiyah.com/v1/docs> serves the same corpus, with none of these steps.

## Directory naming

Each dump is in `{sequence}_{DD}_{MM}_{YYYY}`. `{sequence}` is a sort index of four digits, padded with zeros. For example, `0003_29_01_2026` is the third dump, from 29 January 2026. The complete folder with the highest number is always the current one.

Each folder holds one `qafiyah_public_{YYYYMMDD}_{HHMMSS}.dump` and a `SHA256SUMS`. Some also have a `CHANGES.md`: a diff of the schema and data against the previous dump. The `local-db-edit` skill makes it for manual edits.

## Fallback sample dataset

`data/db/0000_default/` is a small sample: 100 poems, with everything that they refer to. It is always committed, and needs no token. `bun run dev` restores it by default, for anyone, with or without a token. "Restore (local development)" below explains how to pick a full dump instead.

All 100 poems are from the Jahili (pre-Islamic) era, the oldest in the corpus. So the sample is a small, self-contained subset that can ship in git. Every taxonomy table (eras, meters, rhymes, themes, and so on) stays complete. They are category labels, not corpus content, and the filters of the UI need the full set.

The selection is fixed, not random, so the smoke tests pass without a token:

1. every poem of the fixture poets in `scripts/smoke/fixtures.ts` (the only real poems and poets that the probes name)
2. then the first poem of each other Jahili poet, in poet order, up to 100

`manifest.json` next to the dump lists what it holds. `scripts/smoke/fixtures.test.ts` fails if the sample is missing a fixture, or if a probe writes a real slug directly instead of using the fixtures.

After you change the fixtures, generate both files again with `scripts/db/create-fallback-dump.sh`. It restores the newest local dump into a scratch Postgres, which takes about seven minutes.

## Requirements

`pg_restore` from a PostgreSQL whose major version is the same as, or newer than, the version that made the dump. An older PostgreSQL can report an unsupported dump format version.

## Restore (local development)

The database seeds itself. `bun run dev` starts Postgres in Docker, which restores the `data/db/0000_default/` sample on a new volume. It downloads nothing, even with a token, and it uses the sample even when a full dump is on disk. `docker-compose.dev.yml` mounts only the chosen folder at `/dumps`, so `scripts/db/init.sh` picks it by the same rule as in production.

For data work, pick a full dump by its folder. It needs the four `DATA_*` values in `.env` to download (about 390 MB, once):

- `bun run dev --dump 0046_15_10_2026` downloads the dump if it is missing, and restores it on a new volume.
- `bun run db:reset --dump 0046_15_10_2026` does the same, and wipes the existing volume first.
- `bun run db:reset` goes back to the sample.

An existing volume keeps its data until `db:reset`. `bun run data:pull` names the newest dump in the bucket.

To wipe the volume and restore again:

```bash
bun run db:reset                          # the sample
bun run db:reset --dump 0046_15_10_2026   # a full dump
```

## Restore (manual or external use)

Download the dump (`../README.md`, "Download"). Then restore it:

```bash
DUMP=$(ls 0046_15_10_2026/*.dump)
dropdb --if-exists qafiyah && createdb qafiyah && \
pg_restore \
  -U qafiyah \
  -d qafiyah \
  --no-owner \
  --no-privileges \
  "$DUMP"
```

`data/db/0000_default/qafiyah_public_sample.dump` needs no download.

## Verify

```bash
psql -U qafiyah -d qafiyah -c "\dt"
psql -U qafiyah -d qafiyah -c "SELECT count(*) FROM poems;"
```
