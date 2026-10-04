# {NAME} API

> {NAME} is an open-source project dedicated to the Arabic language, making its poetic heritage freely accessible to researchers, developers, and poetry lovers alike. This is its public, read-only API: an open catalog of Arabic poetry from the pre-Islamic era to the present, as JSON over HTTPS.

Base URL: {BASE}

## Documentation

- [OpenAPI spec]({BASE}/openapi.json): Machine-readable OpenAPI 3.1 document, with a description and a real example for every parameter and field
- [Interactive reference]({BASE}/docs): Rendered API docs
- [Developers]({SITE}/developers): Free API keys and usage limits

## Conventions

- Every endpoint is GET and returns JSON, except the random poem, which returns plain text.
- No key is needed. Anonymous callers get 60 requests an hour per address. A free key from {SITE}/developers, sent as the `x-api-key` header, raises this to 500 an hour, at most 10 a second, and at most 1,500 an hour from one address. A key the API does not recognize counts as no key.
- Responses, errors included, carry `x-ratelimit-limit`, `x-ratelimit-remaining`, and `x-ratelimit-reset` (Unix time in seconds when the window ends). A 429 or 503 carries `Retry-After` in seconds.
- Poems and poets have four-letter, case-sensitive slugs (`gnNg`, `PAKT`). Eras, meters, rhymes, themes, verse forms, and collections have lowercase transliterated slugs (`jahili`, `altawil`, `meem`, `alhikma`, `amudi`, `almuallaqat`).
- A list is `{"data": [...], "pagination": {"page", "pageSize", "totalPages", "totalItems"}}` with 1-based pages, and a single item is `{"data": {...}}`.
- Repeat a filter to match any of its values (`?meter=altawil&meter=alkamil`). Different filters combine with AND. Unknown query params are ignored.
- JSON responses carry an `ETag`. Send it back as `If-None-Match` to get a 304.
- Errors are `application/problem+json` (RFC 9457) with `type`, `title`, `status`, `code`, `instance`, and `detail`.
- Lists and counts hold primary readings only. A poem known in several readings (recensions) is listed once, and each reading links the others.
- The data is public domain (CC0 1.0) and the code is MIT.

## Poems

- [GET /v1/poems]({BASE}/poems?poet=PAKT&meter=altawil): Poems, 30 a page in catalog order. Filters: `poet`, `era`, `meter`, `rhyme`, `theme`, `collection`. A slug that matches nothing gives an empty page.
- [GET /v1/poems/{slug}]({BASE}/poems/gnNg): One poem: its verses as pairs of half-lines, poet, era, meter, rhyme, theme, verse form, the poet's previous and next poems, its other readings, and up to 10 related poems. A slug merged into another poem answers 301.
- [GET /v1/poems/random]({BASE}/poems/random): A random poem as plain text, never cached. The body is a poem slug, or with `?option=lines` one verse on two lines, a blank line, and the poet's name.
- [GET /v1/poems/count]({BASE}/poems/count): The number of poems
- [GET /v1/poems/facets]({BASE}/poems/facets?poet=PAKT): One poet's meters, rhymes, and themes with poem counts, for filtering their poems. Params: `poet` (required), then `meter`, `rhyme`, and `theme` as in GET /v1/poems.
- [GET /v1/poems/slugs]({BASE}/poems/slugs): Every poem slug, 45,000 a page in slug order, for crawling

## Poets

- [GET /v1/poets]({BASE}/poets?era=jahili): Poets with their poem counts, 30 a page, most poems first. Params: `page`, `era` (one slug), and `q` to search names, e.g. `?q=%D8%B2%D9%87%D9%8A%D8%B1%20%D8%A8%D9%86%20%D8%A3%D8%A8%D9%8A%20%D8%B3%D9%84%D9%85%D9%89`.
- [GET /v1/poets/{slug}]({BASE}/poets/PAKT): One poet: name, nickname, biography, era, poem count, and whether an avatar exists (served at `https://cdn.qafiyah.com/poets/{slug}/avatar.webp`). A slug merged into another poet answers 301.
- [GET /v1/poets/slugs]({BASE}/poets/slugs): Slugs of poets with at least one poem, 45,000 a page, for crawling

## Search

- [GET /v1/search]({BASE}/search?q=%D8%A3%D9%85%D9%86%20%D8%A3%D9%85%20%D8%A3%D9%88%D9%81%D9%89&types=poems): Full-text search in Arabic over poems (title and verses) and poets (name and nickname), as two sections paged on their own, 20 a page. Params: `q` (up to 100 characters), `types` (`poems`, `poets`, or both, the default), `poemsPage`, `poetsPage`, `exact=true` for the literal phrase only, and the repeatable filters `poetSlugs`, `eraSlugs`, `meterSlugs`, `rhymeSlugs`, `themeSlugs`, `poemTypeSlugs`, `collectionSlugs`. Every filter except `eraSlugs` applies to poems only, and all but `poetSlugs` need `types=poems`. Without `q` the sections are browsed instead. Each section's `totalItems` stops at 10,000.

## Classification

Each list returns every term with its counts. Each item takes the term's slug.

- [GET /v1/eras]({BASE}/eras) and [/v1/eras/{slug}]({BASE}/eras/jahili): Literary eras, oldest first, with poem and poet counts
- [GET /v1/meters]({BASE}/meters) and [/v1/meters/{slug}]({BASE}/meters/altawil): Prosodic meters, with poem and poet counts
- [GET /v1/rhymes]({BASE}/rhymes) and [/v1/rhymes/{slug}]({BASE}/rhymes/meem): Rhyme letters, with poem and poet counts
- [GET /v1/themes]({BASE}/themes) and [/v1/themes/{slug}]({BASE}/themes/alhikma): Themes, with poem counts
- [GET /v1/poem-types]({BASE}/poem-types) and [/v1/poem-types/{slug}]({BASE}/poem-types/amudi): Verse forms (amudi, hurr, and the rest), with poem and poet counts
- [GET /v1/collections]({BASE}/collections) and [/v1/collections/{slug}]({BASE}/collections/almuallaqat): Curated collections, with poem counts

## Links

- [Website]({SITE})
- [Source code]({BASE}/go/github): MIT-licensed code and CC0 data, on GitHub
