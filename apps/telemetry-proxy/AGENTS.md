# Telemetry Proxy Agent Guide

This is a Cloudflare Worker on `t.qafiyah.com`. It forwards browser Sentry envelopes from qafiyah.com to Sentry. A first-party hostname carries them past tracker blockers, and no telemetry touches the VPS or the WAF. It is one file, `src/index.ts`, with no runtime dependencies. `docs/topology.md` shows where it is in the traffic map.

## Behavior

- `POST /api/<projectId>/envelope/` is the only route.
  - The project id must be in `SENTRY_ALLOWED_PROJECT_IDS`. Otherwise, the answer is 403.
  - Any other path gets 404, except `OPTIONS`, which is answered first (see below).
  - Any other method on the route gets 405.
- `OPTIONS` answers the CORS preflight only for `https://qafiyah.com` and `https://www.qafiyah.com` (`ALLOWED_ORIGINS`). Every proxied response carries the same CORS headers.
- Only `Content-Type`, `Content-Encoding`, `Origin`, and `Referer` go upstream. The body passes through unchanged.

## Deploying

The Worker is not part of `docker compose`.

- `bun run deploy` in this directory runs `wrangler deploy`. First run `bunx wrangler login`, or set `CLOUDFLARE_API_TOKEN`.
- `wrangler.jsonc` binds the custom domain `t.qafiyah.com`, and turns on Workers observability. `bunx wrangler tail` streams the live logs.
- The ordered steps are in `.claude/skills/deploy/SKILL.md` (step 7).

A change here needs `wrangler deploy`. A change to how the web app calls the Worker ships with the normal VPS deploy instead. That covers the host in the Sentry DSN and the CSP `connect-src` in `apps/web`. Keep the hostname neutral, with no `sentry` or `analytics` in it, so tracker blockers do not match it.
