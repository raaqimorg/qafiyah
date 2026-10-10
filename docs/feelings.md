# Feelings

The `/feelings` page helps a reader who can't name what they feel. They pick a core feeling, then a
narrower one, then a precise one, from a tree that grows down from "بمَ تشعر؟", and the page shows
verses from the catalog that express that precise feeling.

## Shape

- `apps/web/src/lib/feelings/feeling-tree.ts`: the tree, three levels (7 core feelings, their
  sub-feelings, and 76 precise feelings). Every feeling has a stable `slug` in the same lowercase
  transliteration as the other taxonomies; the Arabic name can change without breaking anything
  that points at the slug.
- `apps/web/src/lib/feelings/feeling-verses.ts`: the verses shown for each precise feeling, keyed
  by its slug. Each entry is a poem slug, its title, poet and era, and the one row shown, with the
  matched word in `<mark>`.
- `apps/web/src/lib/feelings/upright-layout.ts`: where each feeling sits on screen. Pure: it takes
  the chosen path and whether the stage is narrow, and returns positions and the view transform.
- `apps/web/src/components/feelings/`: the island that draws the tree and the result.

Nothing here touches the database, Elasticsearch, or the API schema. The tree and the verses are
static data in the web app: they ship with the `/feelings` page's island (about 17 KB gzipped,
loaded only on that page), and the live site makes no search, API or database requests for them.
The search ran once, offline, when the verses were chosen.

## How the verses were chosen

For each precise feeling, its own word (معزول for معزول) was searched through the public
`/v1/search` endpoint, and the top 20 poem results were read one by one. A verse was kept when it
expresses the feeling itself, as felt by the speaker or by someone the poem sympathizes with. It
was dropped when the word appears in mockery or satire (هجاء، تهكم), in a negation (غير منزعج), or
in another sense of the same letters (مَحتِد, lineage, for مُحتدّ, angry). Up to three verses were
kept per feeling.

A feeling whose word kept returning another sense was renamed to a word for the same feeling that
the catalog uses in that sense: لبس → التباس، متدني → منكسر، عديم الأهمية → مهمَّش، and so on.

## Limitations

- **Verses are found by word, not by meaning.** A verse is a candidate only if it contains the
  feeling's word, in a form the search's light stemmer folds to it. A poem that expresses
  loneliness without the word معزول is never considered, and a feeling's best verses may be
  missing for that reason. Related forms with a different pattern are separate words to the
  stemmer (شراسة and شرس, عزلة and معزول), so they are not searched either.
- **Only the top 20 results per word were read.** The ranking favors titles and exact matches, not
  how well a verse expresses the feeling.
- **The selection is one reviewer's judgment.** Whether a verse is sincere, ironic, or about
  something else is read from the verse alone, without the rest of the poem. Some choices are
  arguable, and a few feelings have only one or two verses.
- **Colloquial poetry is included** where no Fusha verse fit, because the catalog holds both.
- **Verses are stored as text, not looked up live.** If a poem's row is later corrected in the
  catalog, the quoted row here does not change with it; the link still goes to the poem.

Adding or replacing a verse is an edit to `feeling-verses.ts`. Its tests check that every listed
feeling exists in the tree, that every precise feeling has at least one verse, and that every verse
marks its matched word.
