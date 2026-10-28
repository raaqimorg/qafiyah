# Avatar Snapshots

These are versioned zip snapshots of the poet avatar images. They use the same directory pattern as `data/db/`.

## Where they are

The snapshots live in the private R2 bucket `qafiyah-data`, under `avatars/`, not in git. They are dedicated to the public domain under [CC0 1.0](../LICENSE). Every image in them is already served at `cdn.qafiyah.com/poets/<slug>/avatar.webp`. Nothing here is withheld. `../README.md` says how to download a snapshot.

## Directory naming

Each snapshot is in `{sequence}_{DD}_{MM}_{YYYY}`, the same scheme as `data/db/`. For example, `0000_19_09_2026` is the first snapshot, from 19 September 2026.

Each snapshot holds every avatar that is live on its date, not only the new ones. So the newest snapshot alone restores them all.

## Contents

Each snapshot folder holds `avatars.zip` and a `SHA256SUMS`. When you unzip it, it has one folder for each poet:

```
poets/<slug>/avatar.webp
```

These paths match the object keys that are already live in R2 (the bucket `qafiyah-assets`, served publicly at `cdn.qafiyah.com/poets/<slug>/avatar.webp`). The snapshots are a backup and archive copy, not the serving path. The app reads avatars from R2 at `cdn.qafiyah.com`, never from `qafiyah-data`.

`0000_19_09_2026` is the exception: its folders are `avatars/<slug>/avatar.webp`.

## Uploading a batch to R2

The dashboard allows only 100 files for each drag and drop, so a real batch needs the CLI. There is no script for this yet. The last run (6,631 avatars) was a shell loop over `wrangler r2 object put`:

```bash
find <src-dir> -maxdepth 1 -type f -name '*.webp' -print0 \
  | xargs -0 -P 16 -I{} bash -c '
      slug=$(basename "$1" .webp)
      wrangler r2 object put "qafiyah-assets/poets/${slug}/avatar.webp" \
        --file "$1" --content-type image/webp --remote
    ' _ {}
```

Know these three problems:

- **Cloudflare limits the rate of uploads.** With 16 uploads in parallel, 224 of 6,631 uploads came back with `429`. Collect the failures, then try them again, 4 at a time, with a short sleep and two or three attempts each. That cleared all 224.
- **`wrangler r2 bucket info` is wrong right after a bulk upload.** Its `object_count` is a billing and analytics statistic that updates late. It read `1` while thousands of objects were already live. Verify with real requests instead: `curl -sI https://cdn.qafiyah.com/poets/<slug>/avatar.webp` must give `200` and `content-type: image/webp`.
- **wrangler can fail with `fetch failed` while `curl` works.** This happens when Node picks an IPv6 route that does not work, for example over WARP. Run wrangler with `NODE_OPTIONS=--dns-result-order=ipv4first`.

Then set `poet.has_avatar` in Postgres for the poets that you uploaded. That flag makes the web app show the image.

## Removing an avatar

When a poet leaves the database (deleted, or merged into another poet), its avatar stays in R2 until you delete it. Delete it with `wrangler r2 object delete "qafiyah-assets/poets/<slug>/avatar.webp" --remote`, and leave it out of the next snapshot. Do this only after the release that removes the poet, because production keeps the old poet until then.

## Save a snapshot

Zip every live avatar as `avatars.zip` in `data/avatars/{new-dir}/`, then upload it:

```bash
bun run data:push data/avatars/{new-dir}
```

## Restore

```bash
bun run data:pull avatars/0001_09_10_2026   # or the newest snapshot
unzip data/avatars/0001_09_10_2026/avatars.zip -d /tmp/avatars
```
