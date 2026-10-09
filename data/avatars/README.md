# Avatar Snapshots

These are versioned zip snapshots of the poet avatar images. They use the same directory pattern as `data/db/`.

## Open, passphrase on request

These snapshots are public, and dedicated to the public domain under [CC0 1.0](../LICENSE). Every image in them is already served as plaintext at `cdn.qafiyah.com/poets/<slug>/avatar.webp`. Nothing here is withheld. The zips are encrypted, not restricted.

Why encrypt something that is open? Because nobody can edit a public git history after it is pushed. The reason is the same as for `data/db/`:

- After a plaintext zip of every avatar is pushed, every fork, clone, and mirror keeps it for good. Nothing that the maintainers do after that can take it back.
- If an image must come out, for any reason, it must be possible to remove it everywhere that it went.
- Encryption keeps that possible. The plaintext copies are the ones given out on request. So there is always a way to reach each person who holds one.

A passphrase is quick to get. Email avatars@qafiyah.com, and say what you need the snapshot for: your own use, or a contribution that you plan. You get the passphrase at once. There is no vetting, and nobody has to qualify.

## Directory naming

Each snapshot is in `{sequence}_{DD}_{MM}_{YYYY}`, the same scheme as `data/db/`. For example, `0000_19_09_2026` is the first snapshot, from 19 September 2026.

## Contents

Each snapshot is `avatars.zip`, split and encrypted in the same way as the database dumps (below). When you unzip it, it has one folder for each poet:

```
poets/<slug>/avatar.webp
```

These paths match the object keys that are already live in R2 (the bucket `qafiyah-assets`, served publicly at `cdn.qafiyah.com/poets/<slug>/avatar.webp`). This directory is a backup and archive copy, not the serving path. The app reads avatars from R2 at `cdn.qafiyah.com`, never from here.

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

Know these two problems:

- **Cloudflare limits the rate of uploads.** With 16 uploads in parallel, 224 of 6,631 uploads came back with `429`. Collect the failures, then try them again, 4 at a time, with a short sleep and two or three attempts each. That cleared all 224.
- **`wrangler r2 bucket info` is wrong right after a bulk upload.** Its `object_count` is a billing and analytics statistic that updates late. It read `1` while thousands of objects were already live. Verify with real requests instead: `curl -sI https://cdn.qafiyah.com/poets/<slug>/avatar.webp` must give `200` and `content-type: image/webp`.

Then set `poet.has_avatar` in Postgres for the poets that you uploaded. That flag makes the web app show the image.

## Split and encryption

If `avatars.zip` is over about 45 MB, split it into `avatars.zip.part-aa`, `.part-ab`, and so on, with `scripts/db/split-dump.sh`. That script works on any file, not only dumps, so give it `avatars.zip`. Then encrypt each part with the same recipe as the database dumps:

```bash
openssl enc -aes-256-cbc -pbkdf2 -iter 600000 -salt -pass "pass:<passphrase>" -in "$part" -out "$part.enc"
```

Then store the passphrase as `AVATAR_KEY__{dir}` in both `secrets/dev.enc.env` and `secrets/prod.enc.env`. Add the key to `scripts/secrets/schema.ts` first, because `bun run secrets:check` refuses a key that the schema does not list. After `bun run secrets:pull`, maintainers find the passphrase in `.env`.

## Restore and decrypt

```bash
DIR=data/avatars/0000_19_09_2026   # or the newest snapshot
for f in "$DIR"/*.enc; do
  openssl enc -d -aes-256-cbc -pbkdf2 -iter 600000 -pass pass:"<passphrase>" -in "$f" -out "${f%.enc}"
done
cat "$DIR"/avatars.zip.part-* > /tmp/avatars.zip
unzip /tmp/avatars.zip -d /tmp/avatars
```

## No automated tooling yet

This directory has none of the automation of `data/db/`:

- no dedicated scripts to encrypt or resolve
- no entry in `keys.manifest`
- no key pattern in the secrets schema: each `AVATAR_KEY__*` is its own entry
- no restore on boot

Everything here was created by hand. If avatar snapshots become regular, make `scripts/db/encrypt-dump.sh` general, and do not write new scripts. Its file pattern (`*.dump` and `*.dump.part-*`) is the only part that is specific to dumps.
