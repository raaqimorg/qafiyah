# {NAME} API

> {NAME} is an open-source project dedicated to the Arabic language, making its poetic heritage freely accessible to researchers, developers, and poetry lovers alike. This is its public, read-only API: structured, reliable data over JSON/HTTPS. No key is needed; anonymous callers share a per-address hourly limit, and a free key from {SITE}/developers raises it (send it as the x-api-key header).

Base URL: {BASE}

## Documentation

- [OpenAPI spec]({BASE}/openapi.json): Machine-readable OpenAPI 3 specification
- [Interactive reference]({BASE}/docs): Rendered API docs

## Endpoints

- [GET /v1/poems]({BASE}/poems): List and filter poems. Query params: page, poet, era, theme, meter, rhyme, collection (slug filters repeatable)
- [GET /v1/poems/{slug}]({BASE}/poems): Full poem by slug
- [GET /v1/poems/slugs]({BASE}/poems/slugs): Poem slugs, paginated (for sitemaps)
- [GET /v1/poems/count]({BASE}/poems/count): Total number of poems
- [GET /v1/poems/random]({BASE}/poems/random): Random poem as text/plain (append ?option=lines for verse content)
- [GET /v1/poets]({BASE}/poets): List poets. Query params: page, era
- [GET /v1/poets/{slug}]({BASE}/poets): Poet by slug
- [GET /v1/poets/slugs]({BASE}/poets/slugs): Slugs of poets with at least one poem, paginated (for sitemaps)
- [GET /v1/eras]({BASE}/eras): List historical eras with counts
- [GET /v1/eras/{slug}]({BASE}/eras): Era by slug
- [GET /v1/meters]({BASE}/meters): List classical Arabic meters with counts
- [GET /v1/meters/{slug}]({BASE}/meters): Meter by slug
- [GET /v1/poem-types]({BASE}/poem-types): List verse forms (amudi, hurr, and the rest) with counts
- [GET /v1/poem-types/{slug}]({BASE}/poem-types): Verse form by slug
- [GET /v1/rhymes]({BASE}/rhymes): List rhyme letters with counts
- [GET /v1/rhymes/{slug}]({BASE}/rhymes): Rhyme by slug
- [GET /v1/themes]({BASE}/themes): List themes with counts
- [GET /v1/themes/{slug}]({BASE}/themes): Theme by slug
- [GET /v1/collections]({BASE}/collections): List curated collections with counts
- [GET /v1/collections/{slug}]({BASE}/collections): Collection by slug
- [GET /v1/search]({BASE}/search): Full-text search across poems and poets. Query params: q, types, poemsPage, poetsPage, and slug filters

## Links

- [Website]({SITE})
