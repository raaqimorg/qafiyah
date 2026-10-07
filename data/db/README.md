# Database Dumps

These are PostgreSQL custom-format (`pg_dump -Fc`) snapshots of the `public` schema.

## Open, passphrase on request

These dumps are public, and dedicated to the public domain under [CC0 1.0](../LICENSE). They are encrypted, not restricted.

Why encrypt something that is open? Because nobody can edit a public git history after it is pushed.

- After a plaintext dump is pushed, every fork, clone, and mirror keeps it for good. Nothing that the maintainers do after that can take it back.
- If a record must come out of the corpus, for any reason, it must be possible to remove it everywhere that it went.
- Encryption keeps that possible. The plaintext copies are the ones given out on request. So there is always a way to reach each person who holds one.
- It records where copies went. It does not control who gets one.

A passphrase is quick to get. Email dumps@qafiyah.com, name the dump that you want, and say what you need it for: your own use, or a contribution that you plan. You get the passphrase at once. There is no vetting, and nobody has to qualify.

If you do not want to wait for a reply, there are two options:

- `data/db/0000_default/` is plaintext, and needs no passphrase (see "Fallback sample dataset" below).
- The read-only public API at <https://api.qafiyah.com/v1/docs> serves the same corpus, with none of these steps.

## Directory naming

Each dump is in `{sequence}_{DD}_{MM}_{YYYY}`. `{sequence}` is a sort index of four digits, padded with zeros. For example, `0003_29_01_2026` is the third dump, from 29 January 2026. The directory with the highest number is always the current one.

A dump over about 45 MB is committed in parts, `{name}.dump.part-aa`, `.part-ab`, and so on, not as one `{name}.dump` file. This keeps each file under the size limits of GitHub (see `MAINTAINERS_GUIDE.md`). `bun run db:up` and `bun run db:reset` join the parts automatically. A manual restore needs `cat` first (below).

## Encryption

Every real dump here is encrypted with its own passphrase before it is committed (`openssl enc`, one `.dump.part-*.enc` file for each part). So nothing sensitive is in the git history as plaintext.

`data/db/keys.manifest` records a salted hash for each dump, only to check that each passphrase is unique. It never contains a passphrase. `MAINTAINERS_GUIDE.md` explains how to encrypt a new dump.

Some dump directories also have a `CHANGES.md`: a diff of the schema and data against the previous dump. The `local-db-edit` skill makes it for manual edits. It is encrypted in the same way, as `CHANGES.md.enc`, with the passphrase of its dump. So one key decrypts both.

## Fallback sample dataset

`data/db/0000_default/` is a small sample: 100 poems, with everything that they refer to. It is always plaintext, and always committed. It is never encrypted, and needs no passphrase. It lets `bun run dev` work at once for anyone who does not have the passphrase of a real dump. "Restore (local development)" below explains how it is picked.

All 100 poems are from the Jahili (pre-Islamic) era, the oldest in the corpus. So the sample is a small, self-contained subset that can ship as plaintext. Every taxonomy table (eras, meters, rhymes, themes, and so on) stays complete. They are category labels, not corpus content, and the filters of the UI need the full set.

The selection is fixed, not random, so the smoke tests pass without a passphrase:

1. every poem of the fixture poets in `scripts/smoke/fixtures.ts` (the only real poems and poets that the probes name)
2. then the first poem of each other Jahili poet, in poet order, up to 100

`manifest.json` next to the dump lists what it holds. `scripts/smoke/fixtures.test.ts` fails if the sample is missing a fixture, or if a probe writes a real slug directly instead of using the fixtures.

After you change the fixtures, generate both files again with `scripts/db/create-fallback-dump.sh`. It restores the newest decrypted dump into a scratch Postgres, which takes about seven minutes.

## Requirements

`pg_restore` from a PostgreSQL whose major version is the same as, or newer than, the version that made the dump. An older PostgreSQL can report an unsupported dump format version.

## Restore (local development)

The database seeds itself. `bun run db:up` starts Postgres in Docker, and restores a dump on a new volume. `bun run dev` does the same, and also starts the app.

The `DUMP_KEY__<dump-dir>` environment variables decide which dump it restores. They are kept in `secrets/dev.enc.env` (see `docs/deployment/secrets.md`).

- It restores the newest real dump that you have a working passphrase for.
- If you have none, it restores the small `data/db/0000_default/` sample.

To pick a dump and set its passphrase, run `bun run dump:key:set`. It asks for the passphrase, checks that it works, saves it to `secrets/dev.enc.env`, and generates `.env` again.

Decryption writes plaintext next to the `.enc` files, and git ignores it. Removing a key does not delete that plaintext. So a dump that you decrypted once stays in use until you remove it yourself (`git clean -Xd data/db/`).

To restore again on an existing volume:

```bash
bun run db:up       # or: bun run db:reset to wipe the volume and seed it again
```

## Restore (manual or external use)

Real dumps are encrypted. First decrypt them with the passphrase of the dump (email dumps@qafiyah.com if you need one):

```bash
DIR=$(ls -d data/db/*/ | grep -v 0000_default | sort | tail -1)   # newest real dump directory
for f in "$DIR"/*.enc; do
  openssl enc -d -aes-256-cbc -pbkdf2 -iter 600000 -pass pass:"<passphrase>" -in "$f" -out "${f%.enc}"
done
```

Then join the parts if necessary, and restore:

```bash
if ls "$DIR"/*.dump >/dev/null 2>&1; then
  DUMP=$(ls "$DIR"/*.dump)
else
  DUMP=$(mktemp)
  cat "$DIR"/*.dump.part-* > "$DUMP"   # the dump was split, so join it first
fi
dropdb --if-exists qafiyah && createdb qafiyah && \
pg_restore \
  -U qafiyah \
  -d qafiyah \
  --no-owner \
  --no-privileges \
  "$DUMP"
```

`data/db/0000_default/qafiyah_public_sample.dump` needs none of these steps. It is already plaintext.

## Verify

```bash
psql -U qafiyah -d qafiyah -c "\dt"
psql -U qafiyah -d qafiyah -c "SELECT count(*) FROM poems;"
```
