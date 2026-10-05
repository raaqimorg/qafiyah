# Web Agent Guide

Astro (SSR, `output: 'server'`) + React-island frontend for the qafiyah.com Arabic poetry catalog. Pages fetch from `apps/api` through a typed OpenAPI client; a handful of components hydrate client-side for interactivity (search, random poem, settings).

## Shape

- `pages/`: file-based routes. `.astro` files render HTML; `.xml.ts`/`.txt.ts` files generate sitemaps/robots/llms.txt. List/detail pages fetch through `lib/server/*` and `Astro.rewrite('/404')` when that resolves to "not found."
- `lib/server/`: SSR-only. `client.ts` (`apiServer`, internal API URL). `unwrap.ts`: `safeCall`/`unwrap`/`getOrNull`, the retry+not-found+throw pattern every fetcher below is built from. `cache.ts`: Cache-Control builders. `types.ts`: `Ok<'/path'>` derives a resource's type straight from the generated schema; don't hand-declare a DTO that already exists there. One file per resource: `poems`, `poets`, `taxonomies`, `collections`, `sitemap`, plus `search-filter-options`, which builds the home page's search filters from the taxonomy lists at render time (the page is not cached when it fails).
- `lib/api/`: the client shared by server and browser: `browser-client.ts` (`apiBrowser`) is the keyless, same-origin client the islands use instead of `lib/server/client.ts`. `proxy-allowlist.ts` and `proxy-handler.ts` back the `/api/v1/[...path]` route it calls.
- `lib/generated/`: nothing here is hand-authored (see `docs/code-conventions.md`'s "Generated files" rule); one subfolder per source: `openapi/schema.gen.ts` (from `apps/api/generated/openapi/openapi.json`), `well-known/well-known.gen.ts` (from `well-known/*` templates). Don't hand-edit; regenerate via the matching `bun run <name>:generate` script and commit the diff.
- `components/ui/`: design-system primitives. `ui-extended/`: composed pieces built from them. `search/`: the search island (React Query + `nuqs` URL state). `layout/`: page chrome, including the `is:inline` scripts that run before hydration (see below).
- `lib/settings/`: client-only theme/font-scale persistence (localStorage, versioned, every field parsed defensively so one bad value never discards the rest).
- `lib/observability/`: Sentry reporting, the transient-network-error check that drives the SSR retry in `lib/server/unwrap.ts`, and request timing: `src/middleware.ts` times every request that reaches Astro to the end of its body (`request-timing.ts`) and records it by method, route pattern, and status into an OpenTelemetry exponential histogram (`request-metrics.ts`) pushed every 15 s to `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`. Only compose sets that variable, so `bun run dev` and tests record nothing. What reads it: `apps/observability/AGENTS.md`.
- `lib/arabic.ts`: Arabic-specific text helpers: digit conversion, singular/dual/plural noun agreement, input sanitization.
- `lib/seo/`: per-route-type metadata and JSON-LD builders.

## Deliberate, non-obvious behavior

See the Web section of `docs/exceptions.md`.

## Everything else

Nav, footer, PostHog wiring, sitemap/robots/llms.txt generation, Tailwind design tokens (`lib/constants/design-tokens.ts`): ordinary Astro/React plumbing, one file each, no surprises.
