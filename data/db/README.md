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

`data/db/0000_default/` is a small sample: 100 poems, with everything that they refer to. It is always committed, and needs no token. It lets `bun run dev` work at once for anyone who has no token. "Restore (local development)" below explains how it is picked.

All 100 poems are from the Jahili (pre-Islamic) era, the oldest in the corpus. So the sample is a small, self-contained subset that can ship in git. Every taxonomy table (eras, meters, rhymes, themes, and so on) stays complete. They are category labels, not corpus content, and the filters of the UI need the full set.

The selection is fixed, not random, so the smoke tests pass without a token:

1. every poem of the fixture poets in `scripts/smoke/fixtures.ts` (the only real poems and poets that the probes name)
2. then the first poem of each other Jahili poet, in poet order, up to 100

`manifest.json` next to the dump lists what it holds. `scripts/smoke/fixtures.test.ts` fails if the sample is missing a fixture, or if a probe writes a real slug directly instead of using the fixtures.

After you change the fixtures, generate both files again with `scripts/db/create-fallback-dump.sh`. It restores the newest local dump into a scratch Postgres, which takes about seven minutes.

## Requirements

`pg_restore` from a PostgreSQL whose major version is the same as, or newer than, the version that made the dump. An older PostgreSQL can report an unsupported dump format version.

## Restore (local development)

The database seeds itself. `bun run dev` downloads the newest dump (`bun run data:pull`), then starts Postgres in Docker, which restores the newest local dump on a new volume.

- With the four `DATA_*` values in `.env`, the newest dump downloads once (about 370 MB). A new dump in the bucket downloads at the next `bun run dev`.
- With no values, nothing downloads, and the small `data/db/0000_default/` sample is restored.
- If the download fails (no network, or a wrong token), `bun run dev` warns and goes on with the newest local dump, or the sample.

`bun run db:up` and `bun run db:reset` never download. They restore the newest local dump. Run `bun run data:pull` first to get the newest one.

To restore again on an existing volume:

```bash
bun run db:up       # or: bun run db:reset to wipe the volume and seed it again
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
