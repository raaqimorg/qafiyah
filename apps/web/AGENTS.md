# Web Agent Guide

This is the frontend for the qafiyah.com Arabic poetry catalog: Astro with server-side rendering (`output: 'server'`), and React islands. Pages fetch from `apps/api` through a typed OpenAPI client. A few components hydrate in the browser for interaction: search, the random poem, the poem with its reading toolbar, and settings.

## Shape

- `pages/` holds the file-based routes.
  - `.astro` files render HTML. `.xml.ts` and `.txt.ts` files generate the sitemaps, `robots.txt`, and `llms.txt`.
  - List and detail pages fetch through `lib/server/*`. When the result is "not found", they call `Astro.rewrite('/404')`.
- `lib/server/` is for the server only.
  - `client.ts` is `apiServer`, with the internal API URL.
  - `unwrap.ts` holds `safeCall`, `unwrap`, and `getOrNull`. Every fetcher below is built from this pattern: retry, not found, or throw.
  - `cache.ts` holds the Cache-Control builders.
  - `types.ts`: `Ok<'/path'>` takes the type of a resource directly from the generated schema. Do not declare a DTO by hand when the schema already has it.
  - There is one file for each resource: `poems`, `poets`, `taxonomies`, `collections`, and `sitemap`. `search-filter-options` builds the search filters of the home page from the taxonomy lists, at render time. When it fails, the page is not cached.
- `lib/api/` holds the client that the server and the browser share.
  - `browser-client.ts` (`apiBrowser`) is the same-origin client without a key. The islands use it in place of `lib/server/client.ts`.
  - `proxy-allowlist.ts` and `proxy-handler.ts` are behind the `/api/v1/[...path]` route that it calls.
- `lib/generated/`: nothing here is written by hand (see "Generated files" in `docs/code-conventions.md`). There is one subfolder for each source:
  - `openapi/schema.gen.ts`, from `apps/api/generated/openapi/openapi.json`. Generate it again with `bun run openapi:types`.
  - `well-known/well-known.gen.ts`, from the `well-known/*` templates. Generate it again with `bun run well-known:generate`.

  Do not edit these files. Run the script, and commit the diff.

- `components/ui/` holds the design-system primitives. `ui-extended/` holds composed pieces built from them. `search/` is the search island (React Query, with URL state in `nuqs`). `layout/` is the page frame, with the `is:inline` scripts that run before hydration (see the "Web" section of `docs/exceptions.md`).
- `lib/settings/` stores the theme and the font scale in the browser (localStorage, versioned). It parses every field carefully, so one bad value never discards the rest.
- `lib/observability/` holds these parts:
  - the Sentry reporting. `src/middleware.ts` runs three middlewares in order: the strip of Sentry's trace tags from cached pages (see `docs/exceptions.md`), Sentry's request handler, and the request timing.
  - the check for transient network errors, which controls the server-side retry in `lib/server/unwrap.ts`
  - the request timing. `src/middleware.ts` times every request that reaches Astro, to the end of its body (`request-timing.ts`). It records the time by method, route pattern, and status, into an OpenTelemetry exponential histogram (`request-metrics.ts`). The histogram is pushed every 15 seconds to `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`. Only compose sets that variable, so `bun run dev` and the tests record nothing. What reads it is in `apps/observability/AGENTS.md`.
- `lib/analytics/capture-event.ts` sends a PostHog event. It does nothing until PostHog starts, which happens only in release builds (`components/layout/posthog.astro`).
  - `search_completed`: once for each search, with the result counts and the filter counts. It carries the search text only when the search finds nothing.
  - `random_poem_requested`: a press on the random poem button.
  - `setting_changed`: a change of the theme or the poem font size.
- `lib/arabic.ts` holds text helpers for Arabic: digit conversion, the agreement of singular, dual, and plural nouns, and input sanitization.
- `lib/seo/` holds the metadata and JSON-LD builders for each route type.

## Deliberate, non-obvious behavior

See the "Web" section of `docs/exceptions.md`.

## Everything else

The navigation, the footer, the PostHog setup, the generation of the sitemaps, `robots.txt`, and `llms.txt`, and the Tailwind design tokens (`lib/constants/design-tokens.ts`) are ordinary Astro and React plumbing. Each has one file, and nothing in them is unexpected.
