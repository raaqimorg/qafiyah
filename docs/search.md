# Search

How full-text search works, and what the code does to Arabic text on the way in and out. For the
domain terms used here (poem, poet, hemistich, verse), see `docs/domain.md`. For the module
layout, see `apps/api/AGENTS.md`.

There are **no explanatory comments in `apps/api/src/es/`**, the intent lives in constant names.
This doc is that missing commentary.

## Pipeline

`apps/search-indexer` reads Postgres (poem content assembled as hemistichs joined by `*`), maps
rows to documents, bulk-writes them into a fresh versioned index (`poems_v<N>`, `poets_v<N>`),
force-merges it to a single segment (the index is never written again, and one segment halves
the query phase), then atomically swaps the `poems`/`poets` alias onto it. The API only ever queries the alias, so
a reindex is invisible to it. Poems and poets are **separate indices**.

The poems index holds every reading that isn't hidden: primaries and their alternate readings
(recensions, see `docs/domain.md`), each with `primaryId` (its primary's id, its own id for a
primary) and `isPrimary`. A ranked or exact poem search matches every reading and collapses the
results on `primaryId`, so each poem appears once, represented by its best-scoring reading, and a
line that exists only in an alternate still finds the poem. An alternate's score counts half
(`ALTERNATE_READING_WEIGHT`), so the primary represents its poem unless an alternate matches at
least twice as well; without that, the shorter reading would usually win on length alone. The result
links to that reading's page. Totals count poems, not readings: a `cardinality` aggregation on
`primaryId` (an estimate, close to exact below its `precision_threshold` of 10,000, the same cap
totals already have). A total at that cap is a floor, not a count, and the website shows it as
`أكثر من ١٠٬٠٠٠` (more than 10,000). Browsing with no text lists primaries only (`isPrimary: true`). Collapsing
needs `primaryId` in the live index, so this API must not be deployed before the reindex that adds
it.

## Arabic text handling

Two layers, and the split matters: **what gets searched is not what gets displayed.**

### In Rust, at index time

`title` (poems) and `name` (poets) are stored with tashkeel stripped (U+0610-061A, U+064B-065F,
U+06D6-06ED, plus U+0640 tatweel). The original vocalized text is kept alongside in `titleDisplay`
and `nameDisplay`, and a poem carries its poet's name only as `poetNameDisplay`, since poet names
are searched on the poets index. API responses use the Display fields, so a reader sees the
vocalized original while matching happens against the stripped form.

Every field the API only shows (the Display fields, `eraName`, `meterName`, `slug`,
`poetHasAvatar`, `poetIsAnonymous`) is mapped with `index: false` and `doc_values: false`: kept in
`_source` for output, neither searchable nor sortable, so a reindex spends nothing on it. The
`every_field_the_indices_can_search_or_sort_is_one_a_search_reads` test in `es/query.rs` builds
every kind of request and fails when the schema indexes a field none of them reads.

`content` is deliberately **not** stripped in Rust, it relies entirely on the analyzer below.
`nameSort` is a fully folded keyword used only as an alphabetical tiebreaker when browsing.

### In Elasticsearch

One shared char filter, `arabic_letter_folding`, normalizes the letter variants that make Arabic
search frustrating: `أ إ آ ٱ` to `ا`, `ى` to `ي`, `ة` to `ه`, `ؤ` to `و`, `ئ` to `ي`, the Persian
`ی` to `ي` and `ک` to `ك`, and deletes `ء`, the tatweel `ـ`, and the dagger alef. A search for `احمد`
finds `أحمد`, and one for `الرحمن` finds `الرحمٰن`. It also deletes the invisible format characters
that pasted text carries (zero-width spaces and joiners, direction marks, the byte order mark, the
soft hyphen): the tokenizer drops them at the edge of a word, but inside one they split it or stay
in the token.

A second char filter, `invisible_marks`, runs in every analyzer and in the `arabic_exact` normalizer,
right after the folding. It deletes what neither the folding nor Lucene's `arabic_normalization`
removes: the marks U+0610 to U+061A, U+0653 to U+065F (maddah, hamza above and below, and the rest
of that block) and U+06D6 to U+06ED (Quranic annotation), and the bidi embedding, override and
isolate controls (U+202A to U+202E, U+2066 to U+2069). Rust strips the marks, though not the bidi
controls, from titles and names already; content keeps them for display, so the analyzer has to. Without it about 480 poems held
words like `الْحٓرُّ` that indexed as `الحٓر` and never matched `الحر`.

Two analyzers build on it, both with the `standard` tokenizer:

| Analyzer            | Filters                                                           | Used by                               |
| ------------------- | ----------------------------------------------------------------- | ------------------------------------- |
| `arabic_normalized` | `lowercase`, `decimal_digit` (٠-٩ to 0-9), `arabic_normalization` | default for `title`, `content`, names |
| `arabic_stemmed`    | the above plus `_arabic_` stopwords and the Arabic stemmer        | the `.stemmed` subfield               |

Multi-fields: `title` gets `.exact` (keyword), `.stemmed` and `.hamza`; `content` gets `.stemmed`
and `.hamza`, never `.exact`. On the poets index `name` gets `.exact`, `.stemmed` and
`.autocomplete`, and `nickname` gets `.stemmed` and `.autocomplete`. Both mappings are
`dynamic: "strict"`.

Deleting the standalone `ء` makes different words identical on every analyzed field (`ماء` and
`ما`, `سماء` and `سما`), which helps recall, since poets drop the hamza for meter, but blurs
ranking. So there is a second char filter, `arabic_letter_folding_keep_hamza`: the same rules
without the `ء` deletion (a schema test keeps the two lists in step). It feeds the
`arabic_hamza_kept` analyzer behind the `title.hamza` and `content.hamza` subfields, and the
`arabic_exact` normalizer.

The `.exact` keywords use that `arabic_exact` normalizer: the hamza-keeping folding and the same
token filters as `arabic_normalized`, applied to the whole value. A `term` query on a normalized
keyword is normalized too, so an exact title match ignores hamza forms on a carrier and diacritics
(`امي` matches the title `أمي`) with no change to the query code, while `ماء` and `ما` stay
different. `على` and `علي` still match each other through the alef maqsura fold. Two more char
filters, `punctuation_as_space` and `trimmed_spaces`, turn punctuation into a space and squeeze and
trim the spaces, so `يا قلب ؟`, `يا قلب؟` and `(يا قلب)` all equal the title `يا قلب`. No indexed
title holds punctuation, so without them a query with any punctuation never reached the exact tier.

### Poet names

`name` and `nickname` on the poets index use their own analyzers (`arabic_name_normalized`,
`arabic_name_stemmed`, `autocomplete_name`): the same folding and filters, plus two steps that only
make sense for names. `autocomplete_name` adds an `edge_ngram` of 2 to 20 letters at index time
only (its `search_analyzer` is `arabic_name_normalized`), so the query itself is never ngrammed. `name_equivalents` is a closed synonym list: the case forms of the five nouns
grammar inflects by letters (`ابو`/`ابي`/`ابا`, `اخو`/`اخي`/`اخا`, `ذو`/`ذي`/`ذا`), the three
spellings of `امرؤ`, and `ابن`/`بن`. `name_compound_split` writes `عبدالله`, `ابوالطيب` and `ابوبكر`
as two words, on the stored name and the query alike: `ابو` splits from any following run of two
letters or more (so `ابوه`, "his father", stays whole), `عبد` only from `ال`, because `عبده` and
`عبدون` are names of their own. Poem text never sees either, so `أبي` in a verse still means "my
father".

## Relevance

Membership and ranking are decided separately, which is the single most important thing to
understand here:

- **A filter gates recall.** A document is in the result set if it matches `.stemmed` with
  `minimum_should_match: "2<75%"` (a query of one or two terms must match every term; with more,
  75%, rounded down, so three terms need two and four need three), or if it matches every term on
  the normalized field. Requiring both words of a two-word query keeps the first page the same for
  most searches and drops the poems that have only one of the words, which otherwise fill the
  results and inflate the total (`قفا نبك`: 3,166 poems with either word, 206 with both). The second clause matters when the stemmed analyzer
  drops the whole query, as it does for one made only of Arabic stopwords such as `هذا` or `من أنت`.
  The stop filter runs after `arabic_normalization`, so a stopword typed with diacritics, such as
  `هَذا`, is dropped too. For any other query, a document holding every term already passes the first
  clause. Because the folding runs before the stop filter, the stopwords Lucene spells with `ى`
  (`على`, `حتى`, `لدى`) arrive as `علي`, `حتي`, `لدي` and are not dropped. `علي` is also the name Ali,
  so they are left in on purpose.
- **Tiers only rank.** Every tier clause sits in `should`, so it can add score but never admits a
  document on its own.

Poem tiers. The surface tiers are multiplied by the field weight (`title: 4`, `content: 1`); the stemmed tiers are not:

| Tier             | Boost | Clause                               |
| ---------------- | ----- | ------------------------------------ |
| `SURFACE_EXACT`  | 32768 | `term` on `.exact` (title only)      |
| `SURFACE_PHRASE` | 4096  | `match_phrase`, normalized           |
| `STEM_PHRASE`    | 512   | `match_phrase` on `.stemmed`         |
| `SURFACE_ALL`    | 64    | `match`, `operator: and`, normalized |
| `STEM_ALL`       | 8     | `match`, `operator: and`, `.stemmed` |
| `STEM_SOME`      | 1     | `match`, default OR, `.stemmed`      |

The stemmed tiers carry no title weight because the stemmed analyzer drops stopwords: a query made
mostly of particles, such as `من لي لها`, is the single stemmed word `لي`, and weighted by 4 on a short
title field that one word outranked the poem holding all three words as typed. The surface tiers keep
the weight, so a title that holds the words as typed still outranks the same words in content.

The powers-of-two spacing is wide on purpose: an exact title hit cannot be outscored by an
accumulation of weak content matches.

A query of three words or more adds one group above every tier: the poems that hold the typed
words as a phrase, in the title or the text. Inside that group the oldest classical era comes
first (`CLASSICAL_ERA_SLUGS`, in chronological order), and every later or unknown era shares one
rank below them; within a rank the tiers decide. A famous line quoted by later poets, often as
their title, therefore lists the poem it comes from first: of 76 famous verses, the original went
from first in 46 to first in 67, and from the top 3 in 60 to 74. It is a `dis_max` of a
`function_score` (`score_mode: first`, `boost_mode: replace`) that gives each verbatim poem a
constant by era (`VERBATIM_FLOOR` plus `VERBATIM_ERA_STEP` per rank, far above any tier score)
against the tier ladder, with `tie_breaker` 0.01 so the ladder still orders poems of one rank.
Here the alternate-reading half applies to the ladder only, never to the era constant, so a
classical line that survives only in an alternate reading keeps its era's rank instead of falling
below every later poem that quotes it; both readings of one poem share the constant, and the
ladder still lets the primary represent it.
One or two words would put thousands of poems in the group, so they keep the plain ladder. Only
words holding a letter or a digit count, so a lone `؟` or `...` does not turn a two-word query into
a three-word one. The later eras share a rank because their split does not follow time in practice
(modern poets quoting a contemporary one are filed under the earlier era), and an unknown era is not
a time.

When the query contains a standalone `ء`, a ranked poem search also multiplies by
`TYPED_HAMZA_WEIGHT` (1.5) the score of a poem whose title or content has the query as typed, as a
phrase on `title.hamza` or `content.hamza`. Poems that only match the folded spelling still match,
just lower. A query without a standalone `ء` gets no such function.

A ranked poem search with no era filter then multiplies the score of a poem from a classical era
(jahili through mamluki, `CLASSICAL_ERA_SLUGS`) by `CLASSICAL_ERA_WEIGHT` (1.1) in a
`function_score`, and leaves every other poem's score unchanged, so a classical poem passes a later
one only when its score was already within about 9% of it. It never changes which poems match. A
search with an era filter and `exact=true` are not boosted. The empty-`q` browse with no era filter
uses the same weight (see below), where it becomes a strict order because every browsed poem scores
the same. The list is the same eight eras as the related-poems pool (`tmp_pool` in
`scripts/db/sql/refresh-poem-relations.sql`); change both together.

Poets use a flatter, independent ladder over the name and the nickname: exact 12 (name only),
phrase 6, stemmed 3, prefix/autocomplete 2, fuzzy 1 (`fuzziness: AUTO:4,7`, name only). The fuzzy tier
scores a typo match by its edit similarity alone (`fuzzy_rewrite: top_terms_boost_50`): the default
rewrite blends document frequencies across the expansions, and with it the same query on the same
index could rank poets differently from one Elasticsearch restart to the next. As with poems,
a filter decides membership and the ladder only ranks. The filter, a `cross_fields` match with
`operator: and`, admits a poet when every query word matches the name or the nickname as a stem, or
every word matches as a prefix, or every word is within a typo of a word of the name
(`fuzziness: AUTO:4,7`, `prefix_length: 1`: no typo for a word of up to three letters, one from four
to six, two from seven, and the first letter must be right). Three-letter words get no typo because
nearly every one is a letter away from some name word: `أمي` was one edit from `ابي`, which the name
synonyms equate with every `أبو`, and listed 1,952 poets. A line of verse lists no poet; a single
common word can, through a real name word (`الله` in `عبد الله`) or a prefix (`حب` admits `حبيب`).
Elasticsearch groups `cross_fields` fields by analyzer, which is why the two readings don't mix.
Prefixes start at two letters, so a trailing one-letter word (`نزار ق`) admits no one, and words the
analyzers drop, like punctuation, are ignored.

## Poems and poets are queried separately

Two independent ES requests, issued concurrently with `tokio::try_join!` and **never merged**.
The response carries separate `poems` and `poets` envelopes, each with its own pagination.
`relevance` is the raw `_score`, so **scores are not comparable between the two sets**, don't
interleave them.

Every Elasticsearch response must report `timed_out: false` and `_shards.failed: 0` before its
hits are used. A timeout or failed shard returns 503 with `Retry-After` and
`Cache-Control: no-store`, even when Elasticsearch returned HTTP 200. This also applies to `/poets`, so partial
results never become a cached page or a 304. Complete empty results remain cacheable.

Equal scores get an explicit tiebreak, so the order doesn't depend on Lucene's internal document
order, which segment merges can reshuffle: ranked and exact poem searches sort by `_score desc`,
then `id asc`; poet searches by `_score desc`, then the `/poets` list order (`poemsCount desc`,
`nameSort asc`, `id asc`), so among equally good matches the poet with more poems comes first.

Facets: poems filter by poet, era, meter, theme, rhyme, verse form (`poemTypeSlug`), and
collection; poets filter by era only.
Combining a poem-only facet with `types=poets` is a 400, not a silent no-op.

An empty `q` becomes `match_all` with no highlighting. For poems with an era filter it is sorted by
`id desc`. With no era filter it is wrapped in the classical-era `function_score` and sorted by
`_score desc`, then `id desc`: every browsed poem scores the same, so every classical poem comes
first, then the rest, each group newest id first. With the 10,000-result window, a large filter such
as meter `altawil` (about 26,900 classical poems) then only reaches its classical poems by paging;
the rest need an era filter. Poets browse by `id desc`, and the `/poets` list by `poemsCount desc`,
then `nameSort asc`, then `id asc`.
`exact=true` drops the whole ladder for a single phrase match (on the title or the text for poems,
on the name or the nickname for poets), with no tiers, fuzziness, or ngrams, though letter folding still applies because it is
a char filter, not a query option. For poems the phrase is matched on `title.hamza` and
`content.hamza`, so diacritics and hamza seats still fold but a standalone `ء` is kept: exact `ماء`
finds `ماء` and never `ما`, and exact `السماء` does not find a poet's `السما`. The normal search still
finds both. From three words, exact poems are ordered by the same verbatim era group as the ranked
search, oldest classical era first, BM25 within a rank; shorter exact queries are ordered by BM25.

Limits: 20 results per page, page 500 max, `track_total_hits` 10000, `q` at most 100 characters
(UTF-16 units, so a classical verse copied with its diacritics fits: 99.7% of them do), at most 100
slugs per facet.

The API normalizes `q` before searching, on `/search` and `/poets` alike: Unicode NFKC (letters
pasted from a PDF in presentation forms become plain letters, a decomposed hamza is composed), then
every run of whitespace becomes one space and the ends are trimmed. The response echoes the
normalized query. The 100-character limit counts the query as sent, before normalization, because
NFKC can expand one ligature (U+FDFA) to 18 letters.

## Snippets

Highlighting asks for `number_of_fragments: 0`, so ES returns the **whole** content field with
`<mark>` inserted rather than fragments, using `matched_fields` across `content` and
`content.stemmed` so a stem hit still highlights the surface form. Every request carries its own
`highlight_query`, never the whole search query, because the highlighter runs that query again
against each hit: a ranked search highlights through the five `content` tiers of the ladder (the
same `<mark>`s as the full query, which only adds title clauses, era weights and the verbatim
group, none of them on `content`), at a quarter of the cost on a long poem. For a query with a
standalone `ء`, a `highlight_query` on `content.hamza` marks only the words as typed (`ماء`, not
`ما`); a poem that matched only through the folded spelling then has no highlight and shows its
opening verse. An exact search always highlights through a `match_phrase` on `content.hamza`, so
only the phrase as typed is marked.

`content` and its `.stemmed` and `.hamza` subfields index their character offsets
(`index_options: offsets`), so the highlighter reads where each word sits from the index instead
of analyzing the poem's text again. Without them a long poem cost its full length on every search
that returned it: a search returning the longest poem (about 390,000 characters) took 176 ms, 10 ms
of it without highlighting, and pages holding the longest poems now highlight eight to fourteen
times faster with identical marks. Offsets make the merged poems index about a third larger (810 MB
to 1.08 GB). The three fields change together or not at all: when the fields one highlight reads
disagree, Elasticsearch refuses the whole search (`field 'content' was indexed without offsets,
cannot highlight`), so every search that highlights fails.

The API then picks one verse to show. It splits content on `*` and walks **two hemistichs at a
time**, one verse per step, scoring each verse by its longest single `<mark>` run. Three details
worth knowing:

- A phrase can be highlighted across the `*` (the tokenizer treats it as an ordinary separator),
  so one `<mark>` may open in one hemistich and close in the next. Before scoring, the API closes
  such a mark at the end of its hemistich and reopens it at the start of the next, so every
  hemistich has balanced tags and each part of the mark counts for its own verse.
- The run is measured in **UTF-16 code units**, for parity with the JavaScript client.
- The comparison is strictly greater, so on a tie the **earlier** verse wins. That is what the
  `keeps_the_first_of_two_equal_spans` test pins.

With no highlight, it falls back to the opening verse.

## What search deliberately does not do

Worth stating so nobody goes looking: no synonyms beyond that closed list of name forms, no recency decay, no cross-index score
normalization, and no `search_as_you_type` field (the edge-ngram is hand-rolled). Fuzziness admits a
poet within a typo of the name (none below four letters), and never applies to poems. Poet highlighting is supported by the query builder but switched off in `/search`.
