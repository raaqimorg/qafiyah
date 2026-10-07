---
name: local-db-edit
description: Use when about to make a direct manual edit to the qafiyah Postgres database, such as adding an index, deleting or fixing a row, altering a table, or any other ad-hoc change outside a normal migration. Also covers producing and encrypting the resulting dump snapshot afterward.
---

# DB Task

This skill wraps a manual edit to the local qafiyah Postgres database, which is seeded from a `data/db/` dump. It adds the checks that keep the edit safe:

1. Make sure that the edit applies to the latest dump.
2. Unlock the passphrase that the dump needs.
3. After the edit, make a snapshot and encrypt it again.

**Local only.** Every step here runs through `./scripts/dev/compose.sh exec db ...`. So it runs _inside_ the Docker `db` container that `bun run db:up` starts. It never uses a host-network connection to another Postgres.

- Never point a command in this skill at a production host, a tunnel, or a production `DATABASE_URL`.
- If the task really needs a change to production data, stop and say so. Do not improvise a production connection. That is a separate, deliberate action outside this skill.

When you run commands in the container this way, you need no local `psql` or `pg_dump`, and no password (the local socket uses trust authentication). The client also always matches the server's Postgres version. So do not use a client installed on the host.

## 1. Make sure that the latest dump is loaded

```bash
scripts/db/resolve-dump.sh
```

Compare its `using {dump}` line with the real latest dump: `ls -d data/db/*/ | xargs -n1 basename | grep -v 0000_default | sort | tail -1`. Do not use a plain `ls data/db/ | sort | tail -1`. It also lists `keys.manifest`, which sorts after the numbered directories.

- If they match, continue.
- If the loaded dump is older, or the script used `0000_default`, stop. Ask the user whether to continue on the old or sample data, or to get the passphrase of the latest dump first.

## 2. Get the passphrase

Ask the user for the passphrase of the dump that you work on. Do not guess it, and do not use an old one. Then run:

```bash
bun run dump:key:set   # pick the dump, paste the passphrase; saves to secrets/dev.enc.env and generates .env again
bun run db:reset       # wipes the volume, and seeds it again from the unlocked dump
```

## 3. Record the "before" state

Know which tables the edit changes before you make it. The task tells you, for example "add an index to `poems`" or "delete a row from `poets`". Take a snapshot of only those tables, into a scratch directory outside the repo. Your session's scratchpad directory is a good choice. Below, it is `$BEFORE`:

```bash
scripts/db/snapshot-state.sh "$BEFORE" {table}...
```

This records the full schema, which is small. It also writes a CSV for each named table, ordered by primary key. It does _not_ take a snapshot of every table, on purpose. `poem_verses`, `verses`, and `poem_relations` have millions of rows, so a full diff of the corpus on every task would be much too slow. Give only the tables that the task changes.

## 4. Make the edit

```bash
./scripts/dev/compose.sh exec db psql -U qafiyah -d qafiyah
```

You can also pipe in or mount a one-off SQL file, as the task needs.

## 5. Take a snapshot, compare, and encrypt

First, choose the directory name of the new dump. Find the latest number with the same command as in step 1, then use `{N+1:04d}_{DD}_{MM}_{YYYY}`. Below, it is `{new-dir}`:

```bash
scripts/db/snapshot-state.sh "$AFTER" {table}...
scripts/db/diff-state.sh "$BEFORE" "$AFTER" data/db/{new-dir}/CHANGES.md
```

`diff-state.sh` creates `data/db/{new-dir}/` if it does not exist. Then refresh the derived rows (`data/db/MAINTAINERS_GUIDE.md`, "Create the dump"), and write the real dump directly into that directory:

```bash
./scripts/dev/compose.sh exec -T db psql -U qafiyah -d qafiyah -v ON_ERROR_STOP=1 -c 'SELECT public.refresh_poem_tashkeel();'
./scripts/dev/compose.sh exec -T db psql -U qafiyah -d qafiyah -v ON_ERROR_STOP=1 -c 'SELECT public.refresh_poem_relations();'
./scripts/dev/compose.sh exec -T db psql -U qafiyah -d qafiyah -v ON_ERROR_STOP=1 -c 'SELECT public.refresh_taxonomy_stats();'
./scripts/dev/compose.sh exec -T db pg_dump -U qafiyah -d qafiyah --schema=public --no-owner --no-privileges --no-tablespaces -Fc >data/db/{new-dir}/qafiyah_public_$(date +%Y%m%d_%H%M%S).dump
```

If the dump is over 45 MB, split it:

```bash
scripts/db/split-dump.sh data/db/{new-dir}/qafiyah_public_{timestamp}.dump
```

Ask the user for a new passphrase. It must be unique, or `encrypt-dump.sh` refuses it. Then run:

```bash
DUMP_KEY__{new-dir}={new-passphrase} scripts/db/encrypt-dump.sh data/db/{new-dir}
```

This encrypts every plaintext `.dump` and `.dump.part-*` file in that directory, _and_ `CHANGES.md` if it is there. It uses the same passphrase for all of them, so the one dump key decrypts all of them.

Then store the passphrase with `bun run dump:key:set` (pick `{new-dir}`), so it goes into `secrets/dev.enc.env`. A passphrase that was only in the shell is lost when the shell closes.

Then do the rest of the "Post-dump checklist" in `data/db/MAINTAINERS_GUIDE.md`, from "Restores cleanly" to the end. It covers the fallback sample, the add-only commit, and shipping to production.
