# Feelings Snapshots

These are versioned snapshots of the feelings that a language model gave each classical verse. They are the input that fills `feeling_verses`. `docs/feelings.md` explains how the verses were classified, and how good the result is.

## Open, passphrase on request

These snapshots are public, and dedicated to the public domain under [CC0 1.0](../LICENSE). The database dumps in `data/db/` already hold the same feelings, row by row. The files here are encrypted, not restricted, for the same reason as the dumps (`data/README.md`).

A passphrase is quick to get. Email dumps@qafiyah.com, and say what you need the snapshot for: your own use, or a contribution that you plan. You get the passphrase at once. There is no vetting, and nobody has to qualify.

## Why keep them

A run costs money and hours. The database keeps only the verses that got a feeling, and ties each one to its poem and position. A snapshot keeps two things that the database does not:

- **The verses with no feeling.** The model found no feeling in about 70% of the texts. Without them, a new run cannot tell them from verses that were never classified, and pays for them again.
- **The feelings of each text, wherever it is.** When a row repair, a merge, or a split writes a poem's rows again, the database drops that poem's feelings. The snapshot gives them back to the same text, at no cost.

So a maintainer can load the feelings into a new dump without asking the model again, and pays only for the verse texts that are new since the snapshot.

## Directory naming

Each snapshot is in `{sequence}_{DD}_{MM}_{YYYY}`, the same scheme as `data/db/`, dated by the day of the run. Each snapshot holds every result, not only the new ones. So the newest snapshot alone is enough.

## Contents

Each snapshot is `verse-feelings.json.gz`, encrypted. When you unzip it, it is one JSON object:

- Each key is the text of a verse, cleaned the way `cleanVerse` in `scripts/corpus/feeling-load.ts` cleans it: no vowel marks or tatweel, and one space between the two halves.
- Each value is a list of `[slug, strength]` pairs, in the order that the model gave them. A slug is a feeling's slug in `scripts/db/sql/feelings.sql`. The strength is 3 (strong) or 2 (clear). An empty list means that the verse expresses no feeling.

`0000_14_10_2026` holds 548,304 texts, from the run on dump `0045_12_10_2026`.

## Encryption

The snapshot is under 45 MB, so it is one file. Encrypt it with the same recipe as the database dumps, with the passphrase in an environment variable:

```bash
openssl enc -aes-256-cbc -pbkdf2 -iter 600000 -salt -pass env:FEELINGS_KEY__<dir> -in verse-feelings.json.gz -out verse-feelings.json.gz.enc
```

Store the passphrase as `FEELINGS_KEY__{dir}` in both `secrets/dev.enc.env` and `secrets/prod.enc.env`. Add the key to `scripts/secrets/schema.ts` first, because `bun run secrets:check` refuses a key that the schema does not list. After `bun run secrets:pull`, maintainers find the passphrase in `.env`.

## Restore and decrypt

```bash
set -a; source .env; set +a
mkdir -p reports/corpus
DIR=data/feelings/0000_14_10_2026
openssl enc -d -aes-256-cbc -pbkdf2 -iter 600000 -pass env:FEELINGS_KEY__0000_14_10_2026 \
  -in "$DIR/verse-feelings.json.gz.enc" | gunzip > reports/corpus/verse-feelings.json
```

Then load it with `bun run corpus:classify-feelings` (`docs/feelings.md`, "Running it again").
