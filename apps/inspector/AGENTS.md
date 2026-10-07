# Inspector Agent Guide

This is a dev-only Bun app. It renders a single HTML report with the metadata of one live example of every distinct page type on `apps/web`. It is not deployed, and it is not for production use.

## Shape

- `src/route-discovery.ts` scans `apps/web/src/pages/` with a plain filesystem read, not a module import. It changes Astro's file-based routing conventions into "route shapes". These are the ground truth of every distinct page type that must exist.
  - It updates itself. Add, remove, or rename a page in `apps/web`, and the next load of the report shows the change, with no change here.
- `src/site-crawl.ts` finds one live URL for each shape.
  - It fetches static shapes directly.
  - It finds dynamic shapes (`[slug]` routes) with a small, bounded crawl on the same origin. It follows the `href`s on pages that it already fetched, until every shape has a sample or it reaches its budget of 30 fetches.
  - A shape with no URL at the end is a real signal, not a bug. It means that the live site has no reachable example of that page type now.
- `src/inspector.ts` is the extension point: a plain `Inspector` interface (`id`, `title`, `inspect(subject)`). Every report type conforms to it.
- `src/inspectors/` has one file for each report type.
  - `page-metadata.ts` is the only one now. It checks the same OG, Twitter, canonical, JSON-LD, and title thresholds as the smoke `seo` suite (`scripts/smoke/suites/seo.ts`). The two copies are small and independent, and are not shared, on purpose.
  - To add a report type, add a file here that exports an `Inspector`, and add it to the `INSPECTORS` array in `src/index.ts`. Nothing else in the pipeline changes.
- `src/render.ts` builds the final report page, in plain HTML and CSS, with no client JavaScript.
- `src/index.ts` is the `Bun.serve()` entry point. Every request runs discovery, crawl, inspection, and rendering again from the start. So the report always shows the live `apps/web` dev server, never an old snapshot.
- It runs on `DEV_INSPECTOR_PORT` (from the root `config.ts`, imported as `@qafiyah/config`; the default is 4322). It targets `WEB_BASE_URL` (the default is `http://localhost:${DEV_WEB_PORT}`).
- `scripts/dev/run.ts` starts it after the web app reports ready, only when you pass `bun run dev --inspector`.

## Deliberate, non-obvious behavior

See the "Inspector" section of `docs/exceptions.md`.
