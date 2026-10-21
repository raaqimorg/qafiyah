# Data

The versioned snapshots of the data live in the private R2 bucket `qafiyah-data`, not in git. This directory holds their READMEs, and the small sample that `bun run dev` restores by default.

- `db/`: PostgreSQL dumps. Scripts handle them (`bun run db:reset`, `bun run db:reseed`). `bun run dev --dump <folder>` downloads one and seeds itself from it. See `db/README.md` and `db/MAINTAINERS_GUIDE.md`.
- `avatars/`: zips of the poet avatar images. R2 serves a copy at `cdn.qafiyah.com`. See `avatars/README.md`.
- `feelings/`: the feelings that a language model gave each classical verse, the input of `feeling_verses`. See `feelings/README.md`.

All three use the same layout, in the bucket and on disk: `{category}/{sequence}_{DD}_{MM}_{YYYY}/`. Git ignores every downloaded folder. Only `db/0000_default/` (the sample) is committed.

## The bucket

- Each snapshot folder holds its files and a `SHA256SUMS`. The upload writes `SHA256SUMS` last, so a folder without it is not complete.
- A bucket lock keeps every snapshot under `db/`, `avatars/`, and `feelings/` forever. No token can delete or overwrite one, not even the maintainer's.
- The bucket has no public URL.

Everything here is public, and dedicated to the public domain under [CC0 1.0](LICENSE). This is different from the code in the rest of the repo, which is under MIT. The bucket is private so that the maintainers know where each copy went. If a record must come out, for any reason, they can then reach each person who holds a copy.

## Get access

Email dumps@qafiyah.com for `db/` or `feelings/`, or avatars@qafiyah.com for `avatars/`. Say what you need it for: your own use, or a contribution that you plan. You get a read-only token at once. There is no vetting, and nobody has to qualify.

A token gives four values: `DATA_R2_ENDPOINT`, `DATA_BUCKET`, `DATA_R2_ACCESS_KEY_ID`, and `DATA_R2_SECRET_ACCESS_KEY`.

## Download

With a clone of this repo, put the four values in a root `.env` file, which git ignores. Then:

```bash
bun run data:pull                              # the newest dump, into data/db/
bun run data:pull db/0046_15_10_2026           # one dump
bun run data:pull avatars/0001_09_10_2026      # one avatar snapshot
```

It checks `SHA256SUMS` after each download. `bun run dev --dump <folder>` and `bun run db:reset --dump <folder>` run the second command for you.

Without a clone, use [rclone](https://rclone.org), then check the files:

```bash
export RCLONE_CONFIG_R2_TYPE=s3 RCLONE_CONFIG_R2_PROVIDER=Cloudflare RCLONE_CONFIG_R2_REGION=auto
export RCLONE_CONFIG_R2_ENDPOINT="<DATA_R2_ENDPOINT>"
export RCLONE_CONFIG_R2_ACCESS_KEY_ID="<DATA_R2_ACCESS_KEY_ID>"
export RCLONE_CONFIG_R2_SECRET_ACCESS_KEY="<DATA_R2_SECRET_ACCESS_KEY>"
rclone lsf r2:qafiyah-data/db/                 # list the dumps
rclone copy r2:qafiyah-data/db/0046_15_10_2026 0046_15_10_2026
cd 0046_15_10_2026 && sha256sum -c SHA256SUMS
```

## Upload (maintainers)

```bash
bun run data:push data/db/0047_25_10_2026
```

It writes `SHA256SUMS`, uploads the files, and uploads `SHA256SUMS` last. It needs the maintainer token, and it refuses a folder that is already complete in the bucket. If an upload fails, run it again. It uploads the missing files.

## Take a record out

The lock refuses every delete, so a removal takes three steps:

1. Remove the lock rule of the prefix: `wrangler r2 bucket lock remove qafiyah-data --name <category>-forever`.
2. Delete the object.
3. Add the rule again: `wrangler r2 bucket lock add qafiyah-data <category>-forever <category>/ --retention-indefinite`.
