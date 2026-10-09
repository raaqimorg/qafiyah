# Domain Model

This doc says what a poem, poet, meter, rhyme, era, theme, and collection mean in Qafiyah. It is for a contributor who does not already know classical Arabic prosody. It explains concepts, not implementation. For the internals of the schema and the modules, see `apps/api/AGENTS.md` and `apps/web/AGENTS.md`.

## Poem

The poem is the core content entity. It has these fields:

- `title`
- `slug`: four mixed-case letters, for example `TnKK`
- `verses`: one entry for each stored line, in order
- `verse_count`: the number of stored lines, so always the length of `verses`
- `sample`: the first three hemistichs, for previews
- `keywords`

A poem does **not** have its own era. Its era is its poet's era, and the database keeps a synchronized copy of it (see "Era" below).

Every poem has exactly one poet, meter, theme, rhyme, and poem type. The collection, form, register, genre, and rhyme majra are optional (see "Poem type" below).

A poem's content is its ordered rows (`poem_verses` to `verses`), one row for each displayed line.

- A row that holds a full verse separates its two hemistichs with `*`.
- A row with no `*` is a single line: a free-verse line, the closing line of a stanza, or a half-line stored alone.
- The API returns each row as one entry of its `*`-separated parts. It never pairs a row with the next row (`apps/api/src/domain/poems.rs`, `parse_poem_rows`).

For a عمودي poem, the `title` is its first hemistich, with these changes:

- Diacritics, tatweel, punctuation, digits, and non-standard letter forms are deleted. They are not replaced by a space.
- Runs of whitespace become one space.

This is a data rule that the dump process keeps. The application does not enforce it when it reads.

## Verse and hemistich

These are not database entities. They describe the shape of a poem's content.

A **verse** (بيت, the classical Arabic couplet or line) is a pair of **hemistichs** (شطر, half-lines). The first hemistich starts the line, and the second completes it. Together, the two carry one metrical unit.

A poem's `verses` field holds one entry for each stored row:

- two hemistichs for a full verse
- one part for a single line
- more parts only where the source stored them that way

There is no separate "fragment" entity. That word is in the codebase only for the share excerpt of the random poem. The excerpt is one stored row that holds exactly two hemistichs, plus the poet's name, with a maximum length for social sharing (`build_excerpt`, `apps/api/src/domain/poems.rs`). A poem with no such row has no excerpt. So a shareable excerpt is always a complete verse, never half of one, and never halves of two.

## Poem type (نوع القصيدة, poem_type)

The poem type is the poem's **form**: how its lines are built. It is different from what the poem is about (Theme), and from the pattern it scans to (Meter). `poems.poem_type_id` is NOT NULL, and has one of five values:

| slug         | Arabic | what it is                                                                                                                                       |
| ------------ | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `amudi`      | عمودي  | Classical verse. Every line is a bayt of two hemistichs, with one bahr and one rawi from start to end. Most of the catalog is this type.         |
| `hurr`       | حر     | Free verse (شعر التفعيلة) and prose poetry. The lines have different lengths, and there is no single rhyme.                                      |
| `muwashshah` | موشح   | The Andalusi strophic form. The line lengths are regular, but the rhyme changes between strophes on purpose. This is what makes it not a qasida. |
| `muzdawij`   | مزدوج  | The couplet form, usually rajaz. The two hemistichs of each bayt rhyme with each other, and the rhyme changes from bayt to bayt.                 |
| `majhul`     | مجهول  | Unknown. See "The unknown value" below.                                                                                                          |

The difference between these types is structural, not editorial:

- A qasida holds one bahr, so its hemistich lengths are all close. It also holds one rawi, so every ajuz ends on the same rhyme consonant.
- A muwashshah has the first property, but not the second.
- A muzdawij has both. So length and rhyme alone cannot separate it from a qasida.

`poem_type` has no listing page on the website. These parts use it:

- The API lists it with counts (`GET /v1/poem-types`, from `poem_type_stats`).
- Search filters poems by it (`poemTypeSlugs`).
- The poem detail endpoint returns it as `poemType` (`{ name, slug }`).
- The poem page on the website reads it for the layout.
  - An `amudi` poem shows each two-part entry as two offset lines. The sadr is against the right, and the ajuz is against the left, of a column sized in `em`.
  - A single line, a longer entry, and every other type stay centered, with their parts one above the other.
  - Free verse (`hurr`), and any poem whose entries are all single lines, has spacing line by line, not verse by verse.

### Schema-only attributes

Nothing in `apps/api`, `apps/web`, or `apps/search-indexer` reads these poem columns. This is different from meter, rhyme, theme, era, and collection, which all have their own pages and counts. All of these columns can be null:

- `form_id` (`forms`): `qasida` قصيدة, `muqattaa` مقطعة, `abyatthaniya` ابيات ثانية, `baytmufrad` بيت مفرد, `shatrbayt` شطر بيت. It says how much of a poem survives, from a full ode down to a single half-line.
- `register_id` (`registers`): `fasih` فصيح, `nabati` نبطي, `hadith` حديث.
- `genre_id` (`genres`): `shir` شعر, `khatira` خاطرة.
- `rhyme_majra_id` (`majras`): the vowel on the rawi. The values are `fatha` فتحة, `damma` ضمة, `kasra` كسرة, their tanwin forms, and `sukun` سكون.

So a change to any of these has no visible effect on the site today, and needs no reindex. This is a fact about the application as it is now, not a promise.

## Poet

A poet has these fields:

- `name` and `slug`
- an optional `nickname` and `bio`
- one `era`. Poems do not carry their own era.
- `poems_count`, which is computed in advance
- `has_avatar`: whether an image exists for the poet. R2 serves it (see `data/avatars/README.md`).
- `is_anonymous` (see "The unknown value" below)
- `is_hidden` (see "Hidden poets" below)

`nickname` holds any other name that the poet is known by: a kunya (أبو سعيد), a laqab (سراج الهند), or a shuhra (الحياوي). No column says which kind it is. About a fifth of poets have one.

- A nickname that repeats the `name` exactly adds no information, so it is stored as NULL.
- A nickname that is a _part_ of the name is still stored. An example is الحياوي for عبد الحسين الحياوي. This is true for 1,353 poets in the 0039 snapshot.
- The website shows a nickname only when the name does not contain it (`pickAdditiveNickname`, `apps/web/src/lib/seo/poets-page.ts`).

## Meter (بحر, bahr)

The meter is the metrical pattern of a poem. Classical Arabic poetry is quantitative: it is built from fixed patterns of syllable weight. An example is the slug `altawil` for الطويل. Meters apply to poems and to poets, and a meter page shows both counts (`apps/api/src/domain/taxonomy.rs`).

## Rhyme (قافية, qafiyah)

The rhyme is classified by the **rhyme letter** (حرف الروي). This is the consonant that every verse of the poem ends on. An example is the slug `meem` for م.

The rhyme taxonomy of the catalog covers the whole Arabic alphabet. The rhyme pages are ordered by `id`, roughly from hamza to ya. The project's name also comes from here: قافية (qafiyah) is the Arabic word for a poem's rhyme.

## Era (عصر, asr)

An era is a period of time, for example pre-Islamic (جاهلي), Islamic, Umayyad, or Abbasid. Eras have a `sort_order`, so era lists show the real chronological order, not the alphabetical order.

The era is an attribute of the **poet**. Every poem gets its era from its poet.

- `poems.era_id` is a copy of the poet's era, for filters.
- The composite foreign key `(poet_id, era_id) REFERENCES poets (id, era_id) ON UPDATE CASCADE` keeps the copy equal to the poet's era. So a poem cannot hold another era, and a change to a poet's era moves their poems with it.
- The poems list filters on that copy through `(era_id, id)`, like every other facet.

## Theme (غرض, gharad)

The theme is the poem's genre or purpose. An example is `alnasib` (النسيب), the love prelude that classical Arabic poems often start with. Themes apply only to poems. Poets do not have a theme.

## Collection (ديوان, diwan)

A collection is a published, curated anthology that a poem belongs to. It can be a diwan, or one of the classical foundational compilations. `apps/web/src/lib/seo/taxonomy-copy.ts` describes this taxonomy as "دواوين وأمهات الكتب" (diwans and foundational anthologies). Like themes, collections apply only to poems.

## The "unknown" value

Poet, meter, and era each have a real row for **unknown** (`غير معروف`), not a foreign key that can be null. A poem or a poet goes there when the classical sources do not record that attribute.

Theme and rhyme have no such row:

- Every poem has a rhyme letter.
- `المتفرقات` (miscellany) is an ordinary theme, and most of the corpus is in it.

Collection is simply optional (`poems.collection_id` can be null).

The slug of the unknown meter and the unknown era is `ghayrmaruf`. Poet slugs are always four random letters. So a test for `poets.slug = 'ghayrmaruf'` matches nothing, and gives no error. The website finds the unknown rows of meter and era by `name` (`UNKNOWN_ENTITY_NAME`, `apps/web/src/lib/seo/meta-text.ts`).

Poets do not have a single unknown row. They have one anonymous poet for each era:

- `غير معروف` (`JJHE`) holds the anonymous poems whose era is also unknown.
- `مجهول (عباسي)` and the similar rows hold the anonymous poems whose era is known. So each poem keeps its era.
- All of them have `poets.is_anonymous` set. The API sends it as `poet.isAnonymous`. Code checks this flag, never a poet's name or slug.

The poem type spells its unknown value differently, `majhul` (مجهول), and it has a second meaning. A poem is `majhul` for one of two reasons:

- The sources never said what form it is.
- A structural validation pass could not confirm the form that the poem claimed.

The second case means "not verified", never "verified as something else". A poem that fails validation becomes `majhul`, never `hurr`. A poem too short or too irregular to test keeps its label, because there is no evidence against it. So `majhul` says what is known. It does not say that the poem has no form.

`majhul` behaves like any other taxonomy value: it has its own listing page and its own count. But the website removes it from the "top N" attribution lists on purpose. When the website needs a representative sample, it picks the first poem whose poet is not anonymous. So the site never shows an anonymous poet's name as if it were a real byline (`apps/web/src/lib/seo/taxonomy-copy.ts`, `apps/web/src/lib/seo/meta-text.ts`).

## Related poems

Each poem has a list of up to 10 related poems, computed in advance (the `poem_relations` table: `poem_id`, `related_id`, `rank`). `refresh_poem_relations()` refreshes it before every database dump snapshot (see `data/db/MAINTAINERS_GUIDE.md`).

The generator takes its candidates from a pool (`tmp_pool` in `scripts/db/sql/refresh-poem-relations.sql`). So a poem is _suggested_ only if it meets all of these conditions:

- It is a primary عمودي (`amudi`) poem.
- Its meter is known.
- Its poet is not anonymous.
- Its era is in the pool: from jahili to mamluki.

A poem outside that pool still _gets_ its own list, but a shorter one, because a group that points outside the pool adds nothing.

- An Ottoman, modern, contemporary, or unknown-era poem gets only classical poems that share its theme, meter, or rhyme.
- A poem of unknown meter gets no matches by meter.

## Random poem

`GET /v1/poems/random` (the random poem button on the site) picks a poet first, then one of the poet's poems. So every eligible poet has the same chance, whatever the number of their poems.

A poem is eligible when it meets all of these conditions:

- It is a primary, and it is not hidden.
- Its poet is named (not anonymous), and is of the jahili, islami, umawi, or abbasi era.
- It is عمودي, with at least four verses.
- Its meter is known.
- It has tashkeel (`has_tashkeel`, see "Tashkeel").
- It has at least one stored row with exactly two halves, so it has an excerpt.

`refresh_random_poem_pool()` (`scripts/db/sql/random-poem.sql`) computes the eligible set in advance, into `random_poem_pool` (`poet_rank`, `poem_id`). It runs on every restore. So a request is two index lookups (a random `poet_rank`, then a random poem of that poet), not a filter over the corpus.

`?option=lines` then takes one verse of the poem. It tries up to five poems, until the verse and the poet's name fit in 280 characters. `random_poem_json()` returns the poem's rows as `lines`.

The pool is only as current as its last refresh.

- When a poem is deleted, the delete cascades out of the pool. So a manual edit between refreshes can leave a `poet_rank` with no poems. An example is a `merge_poem` that absorbs the only eligible poem of a poet. A request that lands on that rank fails until the next refresh.
- Hiding a poet in place also leaves their poems in the pool.
- Production never sees either problem, because data changes reach it only through a dump, and every restore fills the pool again.

After such an edit on a running database, run `SELECT public.refresh_random_poem_pool();`.

## Tashkeel

`poems.has_tashkeel` says that a poem is vocalized. It is true when the poem's verses hold at least 0.3 harakat (U+064B to U+0652) for each Arabic letter (U+0621 to U+064A). A careful selective vocalization measures about 0.4, and a full one about 0.8.

`refresh_poem_tashkeel()` (`scripts/db/sql/poem-tashkeel.sql`) sets it when a dump is made. A restore keeps the stored values. After a manual edit of verses, run `SELECT public.refresh_poem_tashkeel();`.

## Merged poems and aliases

When two rows hold the same poem by the same poet, one row survives, and `merge_poem(keep, absorb)` merges the other into it (`scripts/db/sql/merge-poem.sql`):

- The survivor keeps its own text.
- From the absorbed copy, it fills an unknown meter, theme, or poem type, and an empty collection, form, register, genre, or majra.
- The absorbed row is deleted. Its slug becomes a row in `poem_aliases`. So `GET /v1/poems/<old slug>` answers `301` to the survivor, and the website redirects the page the same way.

Aliases always point at a live poem. A later merge of a survivor points its aliases again, and a new poem never gets a slug that an alias holds.

`merge_poem` refuses two different poets, because that is a question of attribution, not a duplicate. The excerpt merge of issue #271 crossed poets once, by moving each poem to the winning poet first (`docs/exceptions.md`).

The rules for the survivor are these, in order:

1. A poem in a collection (the curated Mu'allaqat) always survives, and is always the primary.
2. Otherwise, the longest text survives.
3. Then the text with the most vocalization.
4. Then the poem with the most known fields.
5. Then the lowest id.

## Merged poets and re-attribution

`reattribute_poem(poem, poet)` (`scripts/db/sql/merge-poet.sql`) moves a poem to another poet. It moves a primary together with its recensions, and gives them the era of the new poet. A recension never moves alone.

When two poet rows are one person, `merge_poet(keep, absorb)` does these steps:

1. It moves all of the absorbed poet's poems.
2. It fills the survivor's empty fields (and an unknown era) from the absorbed poet.
3. It deletes the absorbed poet, and records its slug in `poet_aliases`. So `GET /v1/poets/<old slug>` answers `301` to the survivor, and the website redirects the page, with its `?page`.

It refuses these merges:

- anonymous poets
- a pair with two different known eras
- an absorbed poet that holds the avatar, because avatars are stored under the slug

A named poet wins over an anonymous one. If a named poet has a poem, the anonymous copy of that poem moves to that poet. Then it is merged, or linked as a recension, like any duplicate by the same poet. A poem that the sources attribute to two poets stays under both.

## Hidden poets

For the site, a poet with `poets.is_hidden` set does not exist:

- The poet and every one of their poems answer 404.
- Their old slugs stop redirecting.
- They are not in any list, search result, sitemap, random poem, related-poems list, or taxonomy count.

The rows stay in the database, so unhiding the poet brings everything back. Hiding is an editorial choice. A removal request needs a deletion, because the encrypted dumps go to anyone who asks.

`poems.is_hidden` is a copy of the poet's flag. The composite foreign key `(poet_id, is_hidden) REFERENCES poets (id, is_hidden) ON UPDATE CASCADE` keeps it equal, the same arrangement as `era_id`.

- So `UPDATE poets SET is_hidden = true WHERE slug = '<slug>'` hides the poems in the same statement.
- The partial indexes on primaries only (`recension_of_id IS NULL AND NOT is_hidden`) keep the list queries index-only.
- `reattribute_poem` gives a moved poem the flag of its new poet.
- `merge_poet` refuses to merge a hidden poet with a shown one.

A change takes effect on the site with the next dump. The snapshot steps refresh the counts and the related poems, and the reseed rebuilds search.

## Recension (رواية, riwaya)

A poem often comes down in more than one reading. A word differs, a verse is missing or added, or the lines come in another order. The corpus keeps each reading as its own row, and links them:

- The primary (`recension_of_id` is NULL) is the reading that the site shows as the main one.
- Every other reading has a `recension_of_id` that points at the primary. The primary is always a poem of the same poet.

A reading can name its `source`: who narrated it, edited it, or vocalized it. It is free text, and NULL when it is not known.

These show primaries only: lists, counts (live and `*_stats`), the sitemap, search browsing, the random poem, and related poems. The facet indexes are partial on `recension_of_id IS NULL`, so the list keeps its index-only scans.

A text search matches every reading, but shows each poem once. It shows the reading that matches best, and prefers the primary (see `docs/search.md`).

A recension keeps its own page and URL, and names its primary. Its canonical URL is the URL of the primary. The page of the primary lists its other recensions.

The Mu'allaqat, which classical sources carry in several recensions, are the model case for more readings later. Seven of them have Faisal Al-Mansour's vocalized edition of al-Anbari's recension as their primary (dump 0040). Their older primaries became recensions. The primaries kept their URLs, so the old texts moved to new URLs (`corpus:promote-reading`).

A merge of a poem that has recensions moves them to the survivor (`merge_poem`).

## Taxonomy counts

The `*_stats` relations hold the poem and poet counts for each term, which the listing pages show:

- `poet_stats`, `meter_stats`, `rhyme_stats`, `era_stats`, `theme_stats`, `collection_stats`, `poem_type_stats`
- the schema-only `form_stats`, `register_stats`, `genre_stats`, `majra_stats`, `nation_stats`, `gender_stats`

They are **tables**, not views. As views, they counted the whole `poems` table again on every request, which took hundreds of milliseconds for each taxonomy index page.

`refresh_taxonomy_stats()` (`scripts/db/sql/refresh-taxonomy-stats.sql`) rebuilds them in about 3 seconds for the full corpus. It runs with `refresh_poem_relations()` before every dump, and again on restore in `scripts/db/init.sh`. Until it runs, any change to a poem's or a poet's taxonomy leaves the tables out of date.

`GET /v1/poems` also reads its total from these tables when the filter is a single term (one value of one facet). So an out-of-date table also makes the pagination of that list wrong.
