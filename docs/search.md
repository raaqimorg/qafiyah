# Search

This doc explains how full-text search works, and what the code does to Arabic text on the way in and out. For the domain terms used here (poem, poet, hemistich, verse), see `docs/domain.md`. For the module layout, see `apps/api/AGENTS.md`.

There are **no explanatory comments in `apps/api/src/es/`**. The constant names carry the intent, and this doc is the missing commentary.

## Pipeline

`apps/search-indexer` does these steps:

1. It reads Postgres. A poem's content is its stored rows joined by a newline. Each full verse keeps the `*` between its hemistichs.
2. It maps the rows to documents.
3. It writes them in bulk into a new versioned index (`poems_v<N>`, `poets_v<N>`).
4. It force-merges the index to a single segment. Nothing writes to the index again, and one segment halves the query phase.
5. It swaps the `poems` or `poets` alias to the new index in one atomic step.

The API only queries the alias, so a reindex has no visible effect on it. Poems and poets are **separate indices**.

The poems index holds every reading that is not hidden: the primaries and their alternate readings (recensions, see `docs/domain.md`). Each document has `primaryId` (the id of its primary, or its own id for a primary) and `isPrimary`.

- A ranked or exact poem search matches every reading, and collapses the results on `primaryId`. So each poem appears once, as its reading with the best score. A line that exists only in an alternate reading still finds the poem.
- The score of an alternate reading counts half (`ALTERNATE_READING_WEIGHT`). So the primary represents its poem, unless an alternate matches at least twice as well. Without this, the shorter reading would usually win on length alone.
- The result links to the page of that reading.
- Totals count poems, not readings. They come from a `cardinality` aggregation on `primaryId`. This is an estimate, close to exact below its `precision_threshold` of 10,000, which is also the cap on totals. A total at that cap is a minimum, not a count. The website shows it as `أكثر من ١٠٬٠٠٠` (more than 10,000).
- Browsing with no text lists only primaries (`isPrimary: true`).

The live index must have every field that the API reads, such as `primaryId` for the collapse. The indexer rebuilds only when it is forced or when the aliases are empty. So when a change adds a field that the API reads, run `bun run reindex:prod` right after its deploy.

## Arabic text handling

There are two layers, and the split is important: **what is searched is not what is displayed.**

### In Rust, at index time

`title` (poems) and `name` (poets) are stored without tashkeel (U+0610-061A, U+064B-065F, U+06D6-06ED, and the tatweel U+0640). The original vocalized text is kept next to them, in `titleDisplay` and `nameDisplay`. A poem carries its poet's name only as `poetNameDisplay`, because poet names are searched on the poets index. API responses use the Display fields. So a reader sees the vocalized original, while the match uses the stripped form.

Every field that the API only shows is mapped with `index: false` and `doc_values: false`. These fields are the Display fields, `eraName`, `meterName`, `slug`, `poetHasAvatar`, and `poetIsAnonymous`. They are kept in `_source` for the output, but they cannot be searched or sorted, so a reindex spends nothing on them. The `every_field_the_indices_can_search_or_sort_is_one_a_search_reads` test in `es/query.rs` builds every kind of request. It fails when the schema indexes a field that no request reads.

`content` is **not** stripped in Rust, on purpose. It relies fully on the analyzer below. `nameSort` is a fully folded keyword, used only to break ties alphabetically when browsing.

### In Elasticsearch

One shared char filter, `arabic_letter_folding`, normalizes the letter variants that make Arabic search difficult:

- `أ إ آ ٱ` become `ا`, `ى` becomes `ي`, `ة` becomes `ه`, `ؤ` becomes `و`, and `ئ` becomes `ي`.
- The Persian `ی` becomes `ي`, and `ک` becomes `ك`.
- It deletes `ء`, the tatweel `ـ`, and the dagger alef.

So a search for `احمد` finds `أحمد`, and a search for `الرحمن` finds `الرحمٰن`.

It also deletes the invisible format characters that pasted text carries: zero-width spaces and joiners, direction marks, the byte order mark, and the soft hyphen. The tokenizer drops them at the edge of a word, but inside a word they split it or stay in the token.

A second char filter, `invisible_marks`, runs in every analyzer and in the `arabic_exact` normalizer, right after the folding. It deletes what neither the folding nor Lucene's `arabic_normalization` removes:

- the marks U+0610 to U+061A
- U+0653 to U+065F (maddah, hamza above and below, and the rest of that block)
- U+06D6 to U+06ED (Quranic annotation)
- the bidi embedding, override, and isolate controls (U+202A to U+202E, U+2066 to U+2069)

Rust already strips the marks (but not the bidi controls) from titles and names. Content keeps them for display, so the analyzer must remove them. Without this filter, about 480 poems held words like `الْحٓرُّ`. They were indexed as `الحٓر`, and never matched `الحر`.

Two analyzers build on it, both with the `standard` tokenizer:

| Analyzer            | Filters                                                           | Used by                               |
| ------------------- | ----------------------------------------------------------------- | ------------------------------------- |
| `arabic_normalized` | `lowercase`, `decimal_digit` (٠-٩ to 0-9), `arabic_normalization` | default for `title`, `content`, names |
| `arabic_stemmed`    | the above plus `_arabic_` stopwords and the Arabic stemmer        | the `.stemmed` subfield               |

The multi-fields are these:

- `title` gets `.exact` (keyword), `.stemmed`, and `.hamza`.
- `content` gets `.stemmed` and `.hamza`, never `.exact`.
- On the poets index, `name` gets `.exact`, `.stemmed`, and `.autocomplete`, and `nickname` gets `.stemmed` and `.autocomplete`.

Both mappings are `dynamic: "strict"`.

The deletion of the standalone `ء` makes different words identical on every analyzed field: `ماء` and `ما`, `سماء` and `سما`. This helps recall, because poets drop the hamza for meter, but it blurs ranking. So there is a second folding char filter, `arabic_letter_folding_keep_hamza`. It has the same rules, without the deletion of `ء`, and a schema test keeps the two lists the same. It feeds the `arabic_hamza_kept` analyzer behind the `title.hamza` and `content.hamza` subfields, and the `arabic_exact` normalizer.

The `.exact` keywords use that `arabic_exact` normalizer. It applies the folding that keeps hamza and the same token filters as `arabic_normalized`, to the whole value.

- Elasticsearch also normalizes a `term` query on a normalized keyword. So an exact title match ignores the hamza forms on a carrier and the diacritics (`امي` matches the title `أمي`), with no change to the query code.
- `ماء` and `ما` stay different.
- `على` and `علي` still match each other, through the fold of the alef maqsura.

Two more char filters, `punctuation_as_space` and `trimmed_spaces`, change punctuation to a space, then join and trim the spaces. So `يا قلب ؟`, `يا قلب؟`, and `(يا قلب)` all equal the title `يا قلب`. No indexed title holds punctuation. So without these filters, a query with any punctuation never reached the exact tier.

### Poet names

`name` and `nickname` on the poets index use their own analyzers: `arabic_name_normalized`, `arabic_name_stemmed`, and `autocomplete_name`. They have the same folding and filters, plus two steps that apply only to names.

`autocomplete_name` adds an `edge_ngram` of 2 to 20 letters, only at index time. Its `search_analyzer` is `arabic_name_normalized`, so the query itself is never cut into ngrams.

`name_equivalents` is a closed synonym list:

- the case forms of the five nouns that grammar inflects by letters: `ابو`/`ابي`/`ابا`, `اخو`/`اخي`/`اخا`, `ذو`/`ذي`/`ذا`
- the three spellings of `امرؤ`
- `ابن` and `بن`

`name_compound_split` writes `عبدالله`, `ابوالطيب`, and `ابوبكر` as two words, in the stored name and in the query:

- `ابو` splits from any next run of two letters or more. So `ابوه` ("his father") stays one word.
- `عبد` splits only from `ال`, because `عبده` and `عبدون` are names of their own.

Poem text never goes through either step. So `أبي` in a verse still means "my father".

## Relevance

Membership and ranking are decided separately. This is the most important thing to understand here.

**A filter controls recall.** A document is in the result set if one of these is true:

- It matches `.stemmed` with `minimum_should_match: "2<75%"`. A query of one or two terms must match every term. With more terms, it must match 75%, rounded down. So three terms need two, and four need three.
- It matches every term on the normalized field.

The first clause keeps the first page the same for most searches. A two-word query requires both words, so the poems that have only one of the words drop out. Otherwise they fill the results and make the total too large. For `قفا نبك`, 3,166 poems have either word, and 206 have both.

The second clause is for a query that the stemmed analyzer drops fully, such as a query of only Arabic stopwords (`هذا`, or `من أنت`).

- The stop filter runs after `arabic_normalization`. So a stopword typed with diacritics, such as `هَذا`, is also dropped.
- For any other query, a document that holds every term already passes the first clause.
- The folding runs before the stop filter. So the stopwords that Lucene spells with `ى` (`على`, `حتى`, `لدى`) arrive as `علي`, `حتي`, and `لدي`, and the filter does not drop them. `علي` is also the name Ali, so they stay on purpose.

**Tiers only rank.** Every tier clause is in `should`. So it can add score, but it never admits a document alone.

The poem tiers are below. The surface tiers are multiplied by the field weight (`title: 4`, `content: 1`). The stemmed tiers are not:

| Tier             | Boost | Clause                               |
| ---------------- | ----- | ------------------------------------ |
| `SURFACE_EXACT`  | 32768 | `term` on `.exact` (title only)      |
| `SURFACE_PHRASE` | 4096  | `match_phrase`, normalized           |
| `STEM_PHRASE`    | 512   | `match_phrase` on `.stemmed`         |
| `SURFACE_ALL`    | 64    | `match`, `operator: and`, normalized |
| `STEM_ALL`       | 8     | `match`, `operator: and`, `.stemmed` |
| `STEM_SOME`      | 1     | `match`, default OR, `.stemmed`      |

The stemmed tiers have no title weight, because the stemmed analyzer drops stopwords. A query of mostly particles, such as `من لي لها`, becomes the single stemmed word `لي`. With a weight of 4 on a short title field, that one word ranked above the poem that held all three words as typed. The surface tiers keep the weight. So a title that holds the words as typed still ranks above the same words in the content.

The spacing by powers of two is wide on purpose. Many weak content matches together cannot score above an exact title hit.

**The verbatim group.** A query of three or more words adds one group above every tier: the poems that hold the typed words as a phrase, in the title or the text.

- In that group, the oldest classical era comes first (`CLASSICAL_ERA_SLUGS`, in chronological order). Every later or unknown era shares one rank below them. Within a rank, the tiers decide.
- So for a famous line that later poets quote, often as their title, the poem that the line comes from comes first. Of 76 famous verses, the original went from first in 46 to first in 67, and from the top 3 in 60 to the top 3 in 74.
- The group is a `dis_max` of two parts, with `tie_breaker` 0.01, so the ladder still orders poems of one rank:
  - a `function_score` (`score_mode: first`, `boost_mode: replace`) that gives each verbatim poem a constant for its era: `VERBATIM_FLOOR`, plus `VERBATIM_ERA_STEP` for each rank, far above any tier score.
  - the tier ladder.
- The half weight of an alternate reading applies only to the ladder, never to the era constant. So a classical line that survives only in an alternate reading keeps the rank of its era. It does not fall below every later poem that quotes it. Both readings of one poem share the constant, and the ladder still lets the primary represent the poem.
- One or two words would put thousands of poems in the group, so they keep the plain ladder.
- Only words that hold a letter or a digit count. So a lone `؟` or `...` does not make a two-word query into a three-word query.
- The later eras share a rank, because in practice their split does not follow time. Modern poets who quote a contemporary poet are filed under the earlier era. And an unknown era is not a time.

**Typed hamza.** When the query contains a standalone `ء`, a ranked poem search also multiplies a score by `TYPED_HAMZA_WEIGHT` (1.5). This applies to a poem whose title or content has the query as typed, as a phrase on `title.hamza` or `content.hamza`. Poems that match only the folded spelling still match, but lower. A query without a standalone `ء` gets no such function.

**Classical eras.** A ranked poem search with no era filter then multiplies the score of a poem from a classical era by `CLASSICAL_ERA_WEIGHT` (1.1), in a `function_score`. The classical eras are jahili to mamluki (`CLASSICAL_ERA_SLUGS`).

- Every other poem's score stays the same. So a classical poem passes a later one only when its score was already within about 9% of it.
- It never changes which poems match.
- A search with an era filter, and a search with `exact=true`, get no boost.
- The browse with an empty `q` and no era filter uses the same weight (see below). There it becomes a strict order, because every browsed poem has the same score.
- The list is the same eight eras as the related-poems pool (`tmp_pool` in `scripts/db/sql/refresh-poem-relations.sql`). Change both together.

**Poets** use a flatter, independent ladder over the name and the nickname: exact 12 (name only), phrase 6, stemmed 3, prefix or autocomplete 2, and fuzzy 1 (`fuzziness: AUTO:4,7`, name only).

- The fuzzy tier scores a typo match by its edit similarity alone (`fuzzy_rewrite: top_terms_boost_50`). The default rewrite mixes document frequencies across the expansions. With it, the same query on the same index could rank poets differently after each Elasticsearch restart.

As with poems, a filter decides membership, and the ladder only ranks. The filter is a `cross_fields` match with `operator: and`. It admits a poet when one of these is true:

- every query word matches the name or the nickname as a stem
- every word matches as a prefix
- every word is within a typo of a word of the name (`fuzziness: AUTO:4,7`, `prefix_length: 1`)

The typo rule allows no typo for a word of up to three letters, one typo for four to six letters, and two from seven letters. The first letter must be correct. Three-letter words get no typo, because almost every one is one letter away from some word of a name. For example, `أمي` was one edit from `ابي`. The name synonyms make that equal to every `أبو`, so it listed 1,952 poets.

- A line of verse lists no poet.
- A single common word can list poets, through a real word of a name (`الله` in `عبد الله`) or a prefix (`حب` admits `حبيب`).
- Elasticsearch groups `cross_fields` fields by analyzer. This is why the two readings do not mix.
- Prefixes start at two letters. So a final one-letter word (`نزار ق`) admits no one.
- Words that the analyzers drop, like punctuation, are ignored.

**Poet popularity.** A ranked poet search multiplies the ladder score by the poet's poem count, damped. It is a `function_score` with `field_value_factor` on `poemsCount` (`modifier: log2p`, `factor: 0.01`, `POET_POPULARITY_FACTOR`).

- So a poet with 104 poems scores about 1.6 times a poet with a single poem, and a poet with 2,000 poems about 5 times.
- It settles near-ties for the poets that readers look for. Before, `زهير` ranked poets whose whole nickname is that word above Zuhayr ibn Abi Sulma (26th). A one-word field scores more than the same word in a four-word name.
- It was measured on the full index (#185), over the common names of 42 famous poets. Before: 33 first, and Zuhayr not on page 1. After: 39 first, and all on page 1 (Zuhayr 5th, behind other poets of that name with more poems).
- 300 minor poets searched by their whole name, and 40 by a nickname that no one else has, all stayed first.
- A factor of 0.02 already put the exact names of two minor poets second. So the exact tier still wins only while the factor stays this small.
- Exact (`exact=true`) poet searches and browsing are not multiplied.

## Poems and poets are queried separately

There are two independent Elasticsearch requests. They run at the same time, with `tokio::try_join!`, and are **never merged**. The response has separate `poems` and `poets` envelopes, each with its own pagination. `relevance` is the raw `_score`. So **scores cannot be compared between the two sets**. Do not interleave them.

Equal scores get an explicit tiebreak, so the order does not depend on Lucene's internal document order, which segment merges can change:

- Ranked and exact poem searches sort by `_score desc`, then `id asc`.
- Poet searches sort by `_score desc`, then by the order of the `/poets` list (`poemsCount desc`, `nameSort asc`, `id asc`). So among equally good matches, the poet with more poems comes first.

Facets:

- Poems filter by poet, era, meter, theme, rhyme, verse form (`poemTypeSlug`), and collection.
- Poets filter by era only.
- A poem-only facet with `types=poets` is a 400, not a request that silently does nothing.

**An empty `q`** becomes `match_all`, with no highlighting.

- For poems with an era filter, it sorts by `id desc`.
- With no era filter, it is wrapped in the classical-era `function_score`, and sorts by `_score desc`, then `id desc`. Every browsed poem has the same score, so every classical poem comes first, then the rest, each group with the newest id first.
- The result window is 10,000. So a large filter, such as meter `altawil` (about 26,900 classical poems), reaches only its classical poems by paging. The rest need an era filter.
- Poets browse by `id desc`. The `/poets` list sorts by `poemsCount desc`, then `nameSort asc`, then `id asc`.

**`exact=true`** replaces the whole ladder with a single phrase match: on the title or the text for poems, and on the name or the nickname for poets. It has no tiers, no fuzziness, and no ngrams. Letter folding still applies, because it is a char filter, not a query option.

- For poems, the phrase is matched on `title.hamza` and `content.hamza`. So diacritics and hamza carriers still fold, but a standalone `ء` stays. Exact `ماء` finds `ماء` and never `ما`, and exact `السماء` does not find a poet's `السما`. The normal search still finds both.
- From three words, exact poems use the same verbatim era group as the ranked search: the oldest classical era first, and BM25 within a rank. Shorter exact queries are ordered by BM25.

**Limits:**

- 20 results on each page, and page 500 at most.
- `track_total_hits` is 10,000.
- `q` has at most 100 characters. These are characters, so a classical verse copied with its diacritics fits: 99.7% of them do.
- A facet takes at most 100 slugs.

The API normalizes `q` before it searches, on `/search` and on `/poets`:

1. Unicode NFKC. Letters pasted from a PDF in presentation forms become plain letters, and a decomposed hamza is composed.
2. Every run of whitespace becomes one space, and the ends are trimmed.

The response echoes the normalized query. The 100-character limit counts the query as sent, before normalization, because NFKC can expand one ligature (U+FDFA) to 18 letters.

## Snippets

Highlighting asks for `number_of_fragments: 0`. So Elasticsearch returns the **whole** content field with `<mark>` tags, not fragments. It uses `matched_fields` across `content` and `content.stemmed`, so a stem hit still highlights the surface form.

Every request has its own `highlight_query`, never the whole search query, because the highlighter runs that query again against each hit.

- A ranked search highlights through the five `content` tiers of the ladder. These give the same `<mark>` tags as the full query, which only adds title clauses, era weights, and the verbatim group, none of them on `content`. On a long poem, this costs a quarter as much.
- For a query with a standalone `ء`, a `highlight_query` on `content.hamza` marks only the words as typed (`ماء`, not `ما`). A poem that matched only through the folded spelling then has no highlight, and shows its first verse.
- An exact search always highlights through a `match_phrase` on `content.hamza`. So only the phrase as typed is marked.

`content` and its `.stemmed` and `.hamza` subfields index their character offsets (`index_options: offsets`). So the highlighter reads the position of each word from the index, and does not analyze the poem's text again.

- Without offsets, a long poem cost its full length on every search that returned it. A search that returned the longest poem (about 390,000 characters) took 176 ms, and 10 ms without highlighting.
- With offsets, pages that hold the longest poems highlight eight to fourteen times faster, with identical marks.
- Offsets make the merged poems index about a third larger (810 MB to 1.08 GB).
- The three fields change together or not at all. When the fields that one highlight reads disagree, Elasticsearch refuses the whole search (`field 'content' was indexed without offsets, cannot highlight`). Then every search that highlights fails.

The API then picks one stored row to show. It splits the content on the newline. So a snippet is always one full verse or one single line, and never joins two rows. It scores each row by its longest single `<mark>` run. Three details are important:

- A phrase can be highlighted across a `*` or a newline, because the tokenizer treats both as ordinary separators. So one `<mark>` can open in one hemistich or row, and close in the next. Before scoring, the API closes such a mark at the end of its part, and opens it again at the start of the next part. So every part has balanced tags, and each part of the mark counts for its own row.
- The run is measured in **UTF-16 code units**, to match the JavaScript client.
- The comparison is strictly greater. So on a tie, the **earlier** row wins. The `keeps_the_first_of_two_equal_spans` test checks this.

With no highlight, the API uses the first row.

## What search does not do, on purpose

These are stated so that nobody looks for them:

- no synonyms beyond the closed list of name forms
- no recency decay
- no normalization of scores across the indices
- no `search_as_you_type` field (the edge ngram is our own)
- no fuzziness for poems. For poets, it admits a typo within the name (none below four letters).

The query builder supports poet highlighting, but `/search` turns it off.
