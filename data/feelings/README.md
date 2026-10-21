# Feelings Snapshots

These are versioned snapshots of the feelings that a language model gave each classical verse. They are the input that fills `feeling_verses`. `docs/feelings.md` explains how the verses were classified, and how good the result is.

## Where they are

The snapshots live in the private R2 bucket `qafiyah-data`, under `feelings/`, not in git. They are public, and dedicated to the public domain under [CC0 1.0](../LICENSE). The database dumps already hold the same feelings, row by row. `../README.md` says how to get a read-only token (email dumps@qafiyah.com), and how to download a snapshot.

## Why keep them

A run costs money and hours. The database keeps only the verses that got a feeling, and ties each one to its poem and position. A snapshot keeps two things that the database does not:

- **The verses with no feeling.** The model found no feeling in about 70% of the texts. Without them, a new run cannot tell them from verses that were never classified, and pays for them again.
- **The feelings of each text, wherever it is.** When a row repair, a merge, or a split writes a poem's rows again, the database drops that poem's feelings. The snapshot gives them back to the same text, at no cost.

So a maintainer can load the feelings into a new dump without asking the model again, and pays only for the verse texts that are new since the snapshot.

## Directory naming

Each snapshot is in `{sequence}_{DD}_{MM}_{YYYY}`, the same scheme as `data/db/`, dated by the day of the run. Each snapshot holds every result, not only the new ones. So the newest snapshot alone is enough.

## Contents

Each snapshot folder holds `verse-feelings.json.gz` and a `SHA256SUMS`. When you unzip it, it is one JSON object:

- Each key is the text of a verse, cleaned the way `cleanVerse` in `scripts/corpus/feeling-load.ts` cleans it: no vowel marks or tatweel, and one space between the two halves.
- Each value is a list of `[slug, strength]` pairs, in the order that the model gave them. A slug is a feeling's slug in `scripts/db/sql/feelings.sql`. The strength is 3 (strong) or 2 (clear). An empty list means that the verse expresses no feeling.

`0000_14_10_2026` holds 548,304 texts, from the run on dump `0045_12_10_2026`.

## Save a snapshot

Write `verse-feelings.json.gz` in `data/feelings/{new-dir}/`, then upload it:

```bash
bun run data:push data/feelings/{new-dir}
```

## Restore

```bash
bun run data:pull feelings/0000_14_10_2026
mkdir -p reports/corpus
gunzip -c data/feelings/0000_14_10_2026/verse-feelings.json.gz > reports/corpus/verse-feelings.json
```

Then load it with `bun run corpus:classify-feelings` (`docs/feelings.md`, "Running it again").
