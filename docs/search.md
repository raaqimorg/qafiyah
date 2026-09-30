# Search

How full-text search works, and what the code does to Arabic text on the way in and out. For the
domain terms used here (poem, poet, hemistich, verse), see `docs/domain.md`. For the module
layout, see `apps/api/AGENTS.md`.

There are **no explanatory comments in `apps/api/src/es/`**, the intent lives in constant names.
This doc is that missing commentary.

## Pipeline

`apps/search-indexer` reads Postgres (poem content assembled as hemistichs joined by `*`), maps
rows to documents, bulk-writes them into a fresh versioned index (`poems_v<N>`, `poets_v<N>`),
then atomically swaps the `poems`/`poets` alias onto it. The API only ever queries the alias, so
a reindex is invisible to it. Poems and poets are **separate indices**.

## Arabic text handling

Two layers, and the split matters: **what gets searched is not what gets displayed.**

### In Rust, at index time

`title` and `poetName`/`name` are stored with tashkeel stripped (U+0610-061A, U+064B-065F,
U+06D6-06ED, plus U+0640 tatweel). The original vocalized text is kept alongside in
`titleDisplay`/`poetNameDisplay`/`nameDisplay`, mapped as `keyword, index: false`: stored for
output, never searchable. API responses prefer the Display field, so a reader sees the vocalized
original while matching happens against the stripped form.

`content` is deliberately **not** stripped in Rust, it relies entirely on the analyzer below.
`nameSort` is a fully folded keyword used only as an alphabetical tiebreaker when browsing.

### In Elasticsearch

One shared char filter, `arabic_letter_folding`, normalizes the letter variants that make Arabic
search frustrating: `أ إ آ ٱ` to `ا`, `ى` to `ي`, `ة` to `ه`, `ؤ` to `و`, `ئ` to `ي`, and deletes
`ء` and the tatweel `ـ`. A search for `احمد` finds `أحمد`.

Three analyzers build on it, all with the `standard` tokenizer:

| Analyzer            | Filters                                                           | Used by                               |
| ------------------- | ----------------------------------------------------------------- | ------------------------------------- |
| `arabic_normalized` | `lowercase`, `decimal_digit` (٠-٩ to 0-9), `arabic_normalization` | default for `title`, `content`, names |
| `arabic_stemmed`    | the above plus `_arabic_` stopwords and the Arabic stemmer        | the `.stemmed` subfield               |
| `autocomplete_2`    | the above plus `edge_ngram` 2-20                                  | the `.autocomplete` subfield          |

`autocomplete_2` is index-time only, its `search_analyzer` is `arabic_normalized`, so the query
itself is never ngrammed.

Multi-fields: `title` and the name fields get `.exact` (keyword), `.stemmed`, and
`.autocomplete`. **`content` gets `.stemmed` only**, no `.exact` and no `.autocomplete`, which is
asserted in the schema code. Both mappings are `dynamic: "strict"`.

## Relevance

Membership and ranking are decided separately, which is the single most important thing to
understand here:

- **A filter gates recall.** A document is in the result set if it matches `.stemmed` with
  `minimum_should_match: "1<75%"` (one term must match; with more than one, 75% must), or if it
  matches every term on the normalized field. The second clause matters when the stemmed analyzer
  drops the whole query, as it does for one made only of Arabic stopwords such as `هذا` or `من أنت`.
  It also matters for a stopword typed with diacritics, such as `هَذا` or `مِن`: the stop filter runs
  before normalization, so the query keeps that word on `.stemmed`, while unvocalized poems lost it
  at index time and now pass only through the normalized field. For any other query, a document
  holding every term already passes the first clause.
- **Tiers only rank.** Every tier clause sits in `should`, so it can add score but never admits a
  document on its own.

Poem tiers, final boost = tier boost times field weight (`title: 4`, `content: 1`):

| Tier             | Boost | Clause                               |
| ---------------- | ----- | ------------------------------------ |
| `SURFACE_EXACT`  | 32768 | `term` on `.exact` (title only)      |
| `SURFACE_PHRASE` | 4096  | `match_phrase`, normalized           |
| `STEM_PHRASE`    | 512   | `match_phrase` on `.stemmed`         |
| `SURFACE_ALL`    | 64    | `match`, `operator: and`, normalized |
| `STEM_ALL`       | 8     | `match`, `operator: and`, `.stemmed` |
| `STEM_SOME`      | 1     | `match`, default OR, `.stemmed`      |

The powers-of-two spacing is wide on purpose: an exact title hit cannot be outscored by an
accumulation of weak content matches.

Poets use a flatter, independent ladder: exact 12, phrase 6, stemmed 3, prefix/autocomplete 2,
fuzzy 1 (`fuzziness: AUTO`), with `minimum_should_match: 1`.

## Poems and poets are queried separately

Two independent ES requests, issued concurrently with `tokio::try_join!` and **never merged**.
The response carries separate `poems` and `poets` envelopes, each with its own pagination.
`relevance` is the raw `_score`, so **scores are not comparable between the two sets**, don't
interleave them.

Facets: poems filter by poet, era, meter, theme, rhyme, and collection; poets filter by era only.
Combining a poem-only facet with `types=poets` is a 400, not a silent no-op.

An empty `q` becomes `match_all` sorted by `id desc` (the `/poets` list: `poemsCount desc`, then
`nameSort asc`, then `id asc`) with no highlighting.
`exact=true` drops the whole ladder for a single `match_phrase`, with no tiers, fuzziness, or
ngrams, though letter folding still applies because it is a char filter, not a query option.

Limits: 20 results per page, page 500 max, `track_total_hits` 10000, `q` at most 50 characters, at
most 100 slugs per facet.

## Snippets

Highlighting asks for `number_of_fragments: 0`, so ES returns the **whole** content field with
`<mark>` inserted rather than fragments, using `matched_fields` across `content` and
`content.stemmed` so a stem hit still highlights the surface form.

The API then picks one verse to show. It splits content on `*` and walks **two hemistichs at a
time**, one verse per step, scoring each verse by its longest single `<mark>` run. Two details
worth knowing:

- The run is measured in **UTF-16 code units**, for parity with the JavaScript client.
- The comparison is strictly greater, so on a tie the **earlier** verse wins. That is what the
  `keeps_the_first_of_two_equal_spans` test pins.

With no highlight, it falls back to the opening verse. An unclosed `<mark>` scores zero.

## What search deliberately does not do

Worth stating so nobody goes looking: no synonyms, no recency decay or `function_score`, no
cross-index score normalization, and no `search_as_you_type` field (the edge-ngram is
hand-rolled). Fuzziness applies to **poet names only**, never to poems. Poet highlighting is
supported by the query builder but switched off in `/search`.
