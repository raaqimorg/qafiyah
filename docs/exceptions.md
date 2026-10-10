# Exceptions

This file lists the approved departures from the standard approach (see "Keep it boring" in the root `AGENTS.md`), grouped by component. Anything that is not listed here must be standard. The `AGENTS.md` file of each component describes its shape. The reasons that are not obvious are here. A date is when the behavior first appeared in the repo.

Each entry has this form:

```md
### <short title>

- **What:** the unusual thing, in one line
- **Where:** file paths
- **Why:** the reason the standard approach did not work
- **Normal approach:** what we would usually do instead
- **Date:** YYYY-MM-DD
```

## Needs review

These departures are not approved yet. A full scan found them on 2026-09-24. They are in order, from the most to the least concerning. These entries use **Why it's unusual** and **Status** in place of **Why** and **Date**. After a review, a justified entry moves into its component section below, with a **Why** and a **Date**. Every other entry gets a plan to replace it.

### The quality gate is a custom runner

- **What:** the gate is `scripts/ci.ts`, a 340-line orchestrator with phases, a worker pool, and output scrubbing. GitHub Actions runs it on every push and pull request. The non-Docker phases run in one job. Each Docker phase (`--phase db`, `origin`, `stack`) runs in its own job, when the change touches what that phase uses.
- **Where:** `scripts/ci.ts`, `scripts/ci/phases.ts`, `.github/workflows/ci.yml`
- **Why it's unusual:** the runner re-does turbo's parallelism and log prefixing, then filters turbo's banners with a hard-coded prefix list.
- **Normal approach:** root tasks become turbo tasks, and the workflow calls them directly as separate jobs.
- **Status:** Needs review

### Full database dumps are committed to git under a homemade key scheme

- **What:** every Postgres dump is committed as openssl-encrypted parts of 45 MB, with a hand-built key manifest. In October 2026, HEAD holds 90 parts (about 3.5 GB), and `.git` is about 3.5 GB.
- **Where:** `data/README.md`, `data/db/`, `scripts/db/split-dump.sh`, `scripts/db/encrypt-dump.sh`, `scripts/db/resolve-dump.sh`, `scripts/dev/set-dump-key.sh`, `scripts/dev/check-dump-key.sh`
- **Why it's unusual:** encrypted blobs never delta-compress, so every clone and every VPS fetch carries every version forever, and pushes already need the `pack.useSparse=false` workaround. The manifest "hash" is `openssl enc -P` with a salt hard-coded in two scripts. Passphrases go on the command line as `-pass "pass:..."`, where `ps` can read them. The "newest dump" lookup is repeated in six scripts.
- **Normal approach:** dumps published as GitHub Release assets or R2 objects (R2 already holds avatars and backups) and downloaded by `db:up`. If encryption stays, use `age`, or at least `-pass env:VAR`.
- **Status:** Needs review

### Deploys build images on the production VPS over SSH

- **What:** `bun run deploy` joins shell files, uploads them over SSH to a temporary file on the host, and runs that file there with stdin from `/dev/null`. It runs `git reset --hard`, builds every image on the 8 GB production server, and rolls replicas with a custom `rollout()`. `bun run db:reseed` builds `api` and `search-indexer` in the same way before it restores.
- **Where:** `scripts/deploy/vps.sh`, `scripts/lib/remote.sh`, `scripts/db/reseed.sh`, `scripts/es/reindex-prod.sh`, `.github/workflows/images.yml`, `.claude/skills/deploy/SKILL.md`, `docs/deployment/architecture.md`
- **Why it's unusual:** release Rust builds (`lto = true`, one codegen unit) and the Astro build compete with the live stack for memory. This is why the docs keep swap. With no registry, a rollback means a revert of `main` and a new build. The Images workflow builds each image and discards it. `rollout()` copies the docker-rollout plugin, with fixed timings: a health deadline of 150 seconds, and a settle time of 7 seconds.
- **Normal approach:** CI builds on merge and pushes SHA-tagged images to GHCR, the VPS runs `docker compose pull` and `docker rollout` (or Kamal), and rollback redeploys the previous tag.
- **Status:** Needs review

### Dependabot never updates Docker images

- **What:** `.github/dependabot.yml` has `bun`, `cargo`, and `github-actions` entries but no `docker` or `docker-compose` entry.
- **Where:** `.github/dependabot.yml`
- **Why it's unusual:** base images (`rust`, `alpine`, `oven/bun`, `postgres`, and the exact Elasticsearch and CRS tags) never get update PRs.
- **Normal approach:** `docker` and `docker-compose` entries next to the existing ones.
- **Status:** Needs review

### Dev stacks are documented as running on the production VPS

- **What:** the dev override renames containers, ports, volumes, and subnets so a dev clone can run beside production on the same 8 GB host.
- **Where:** `docker-compose.dev.yml`, `docs/deployment/architecture.md` ("Prod vs dev isolation (both on one host)"), `docs/topology.md`
- **Why it's unusual:** the memory limits in `docker-compose.yml` already add up to about 5.5 GB on the 8 GB server. A rollout then doubles `api` and `web`. The dev override alone gives Elasticsearch a heap of 1500m and `mem_limit: 3g`. So a dev run can push production out of memory.
- **Normal approach:** develop on laptops or a separate staging VM, and keep the production host for production.
- **Status:** Needs review

### The web container runs two processes and is also the API's public ingress

- **What:** the web image runs nginx and Bun side by side under a shell loop that polls every 5 seconds. That nginx also carries the `api.qafiyah.com` server block (routing, the `/account` 404, and the API CSP).
- **Where:** `apps/web/Dockerfile`, `apps/web/docker-entrypoint.sh`, `apps/web/nginx.conf`, `apps/web/nginx-csp-api.conf`, `docker-compose.yml` (`web` networks and `depends_on`), `docs/deployment/architecture.md`, `docs/topology.md`
- **Why it's unusual:** requests pass through Cloudflare, the edge-gateway nginx, the web nginx, then Bun or the API. A change to the edge behavior of the API needs a new web image. A crash of the Astro process also takes `api.qafiyah.com` down. The `while kill -0 ...; sleep 5` loop is a homemade supervisor. `web` also waits for `db` (`depends_on`), but it never talks to Postgres.
- **Normal approach:** one process for each container. Route each hostname to its own service at the existing edge gateway (or with cloudflared ingress rules), and remove the `db` dependency of `web`.
- **Status:** Needs review

### A self-hosted WAF that has only ever logged

- **What:** edge-gateway runs ModSecurity with OWASP CRS behind Cloudflare, and it has been in `DetectionOnly` since 2026-06-17.
- **Where:** `docker-compose.yml` (`edge-gateway`, `MODSEC_RULE_ENGINE`), `apps/edge-gateway/`, `docs/deployment/services.md`, `.claude/skills/deploy/SKILL.md`
- **Why it's unusual:** it is an extra nginx hop that blocks nothing. `services.md` records enforcement as validated safe on 2026-06-18, but it stays off. Each image upgrade needs the vendor template copied and patched again by hand.
- **Normal approach:** Cloudflare's managed WAF rules and rate limiting, which are already in the path, with one reverse proxy at the origin.
- **Status:** Needs review

### OAuth is a hand-written client with a redundant signed state

- **What:** the authorize URLs, token exchange, and userinfo calls are hand-written. The state is HMAC-signed with `SESSION_STATE_SECRET` on top of the cookie comparison, signatures are compared with a homemade constant-time loop, and there is no PKCE.
- **Where:** `apps/web/src/lib/server/oauth/` (`state.ts`, `providers.ts`, `exchange.ts`), `apps/web/src/pages/auth/[provider].ts`, `apps/web/src/pages/auth/callback/[provider].ts`, `apps/web/docker-entrypoint.sh`
- **Why it's unusual:** the whole standard CSRF defense is a comparison of a random state in an HttpOnly cookie with the query parameter. The signature binds nothing (no expiry, provider, or session). Anyone can get a validly signed state by visiting `/auth/google`. So the secret adds a required variable, and no security. The name suggests sessions, but the secret only signs the OAuth state. The approved "Web owns identity" entry covers where OAuth lives, not a client written by hand.
- **Normal approach:** Arctic (`Google`, `GitHub`, `generateState`, `generateCodeVerifier`, `validateAuthorizationCode`) with the state and PKCE verifier in short-lived HttpOnly cookies, keeping the existing `email_verified` checks.
- **Status:** Needs review

### Cookies, redirects, and session lookup bypass Astro's built-ins

- **What:** the app uses no `Astro.cookies`, no `redirect()`, and no middleware or `Astro.locals`. Instead there are three copy-pasted cookie parsers, hand-built `Set-Cookie` strings in seven files, and ten hand-built 302 responses.
- **Where:** `apps/web/src/lib/server/session.ts`, `apps/web/src/lib/server/new-key-cookie.ts`, `apps/web/src/lib/server/oauth/redirect.ts`, `apps/web/src/lib/viewer-hint.ts`, `apps/web/src/pages/auth/`, `apps/web/src/pages/account/`, `apps/web/src/pages/login.astro`, `apps/web/src/pages/api/me.ts`, `apps/web/src/pages/poems/random.ts`, `apps/web/src/test/context.ts`
- **Why it's unusual:** Astro ships a cookie API and `context.redirect()`. Middleware is the usual place to resolve the session into `locals`, and to set `no-store` on the auth prefixes. The encoding is not consistent: the new-key cookie is URI-encoded, and the others are not. Each route repeats `resolveViewer(request.headers.get('cookie'))`. The test helper builds a `cookies` mock that production never uses.
- **Normal approach:** `context.cookies.get/set/delete`, `return context.redirect('/login')`, and a `src/middleware.ts` that sets `locals.viewer` and the `no-store` header for `/account`, `/auth`, and `/login`.
- **Status:** Needs review

### Web environment config is split four ways instead of `astro:env`

- **What:** `envin` validates one optional public variable. Server variables are raw `process.env` reads that fall back to `''`. The checks for required variables are in the shell entrypoint. `astro.config.mjs` declares `PROD_SITE_URL` and `DEV_WEB_PORT` again.
- **Where:** `apps/web/src/env.ts`, `apps/web/src/lib/server/env.ts`, `apps/web/docker-entrypoint.sh`, `apps/web/astro.config.mjs`, `apps/web/src/lib/constants/config.ts`
- **Why it's unusual:** Astro has a typed env schema (`envField`, `context: 'server'`, `access: 'secret'`) that fails at startup. The `''` fallbacks are why the approved `turbo.json` entry says that a missing variable "silently behaves as if unconfigured". In production, a missing `INTERNAL_API_URL` falls back to localhost, with no message. Reading env at import time forces route tests to change `process.env`, reset modules, and import routes dynamically.
- **Normal approach:** declare every variable in `env.schema` in `astro.config.mjs` and import from `astro:env/server` or `astro:env/client`.
- **Status:** Needs review

### The search filters use a hand-built multi-select combobox

- **What:** `Select` is a 302-line homemade ARIA combobox.
- **Where:** `apps/web/src/components/ui-extended/select.tsx`, `apps/web/src/components/search/filters.tsx`, `apps/web/src/components/poet-poem-filters.tsx`
- **Why it's unusual:** Option ids (`option-${index}`) repeat across instances. The single-select, `clearValue`, and `disabled` modes are never used.
- **Normal approach:** a maintained primitive (shadcn Popover with cmdk, Headless UI `Listbox multiple`, or React Aria), or a checkbox group in a `<fieldset>` with a `<legend>`.
- **Status:** Needs review

### The API reports errors to Sentry by hand

- **What:** the error layer calls `sentry::capture_error` with tags for the contract code, method, and path. Logging and request ids are `tracing` and `tower-http` (#195), but nothing ties a Sentry event to the request's `x-request-id` or its log lines.
- **Where:** `apps/api/src/error.rs`, `apps/api/src/sentry.rs`
- **Why it's unusual:** the Sentry SDK has integrations that do this from the request and the log: `sentry-tower` attaches the request, and `sentry-tracing` turns `error` events into Sentry events with their span fields.
- **Normal approach:** `sentry-tower`'s layers, or the `sentry-tracing` layer on the `tracing` subscriber.
- **Status:** Needs review

### `bun run dev` is a custom process supervisor that reads its children's output

- **What:** `scripts/dev/run.ts` (560 lines) starts cargo, turbo, astro, and the inspector. It decides readiness and status by regex-matching their stdout, the API's JSON log fields, and `resolve-dump.sh`'s stderr.
- **Where:** `scripts/dev/run.ts`, `scripts/dev/clean.sh`, `scripts/smoke/run.ts`, `apps/web/package.json` (`dev`)
- **Why it's unusual:** startup depends on the exact words of other programs' output: Astro's "Local" banner, turbo's banners, shell log lines, and field names in `apps/api/src/log.rs`. So a reworded line breaks `dev`, the smoke phase, and the gate, with no clear error. Leftovers are killed with `pkill -f "${ROOT}.*astro"`. A local `bun run ci` stack run stops the shared `qafiyah-dev` project, then starts again only the dev `db` and `elasticsearch` containers that were running. A dev server that uses them loses them while the stack phase runs.
- **Normal approach:** a `dev` script in `apps/api/package.json` (`cargo watch -x run`) supervised by `turbo run dev`, or `docker compose watch`, with readiness checked through `/healthz`.
- **Status:** Needs review

### Worktree isolation is layered over Compose and applied inconsistently

- **What:** a `--worktree` flag hashes the worktree name into port offsets and a Compose project suffix across about eight scripts. `compose.sh` turns it on automatically in a linked worktree, while `run.ts`, `db-test.ts`, the smoke scripts, and `conformance.ts` honor only the explicit flag.
- **Where:** `scripts/dev/worktree.ts`, `scripts/dev/compose.sh`, `scripts/dev/run.ts`, `scripts/ci.ts`, `scripts/smoke/surfaces.ts`, `scripts/api/conformance.ts`, `scripts/dev/db-test.ts`, `docker-compose.dev.yml`, `docs/development.md`
- **Why it's unusual:** Compose already isolates each project. The repo overrides that with a fixed `COMPOSE_PROJECT_NAME`, `container_name`, and volume names, then builds isolation again on top.
  - In a worktree without the flag, `compose.sh` starts the database and Elasticsearch on offset ports. But `run.ts` points the API at offset 0, which is the containers of the primary checkout.
  - `docs/development.md` says that the flag is never implied, and `compose.sh` contradicts that.
  - The fixed dev subnets also stop two worktrees from running full stacks at the same time.
- **Normal approach:** let Compose scope by project (no `container_name` or fixed volume names in dev). Keep the ports of each checkout in its own `.env`, or read them with `docker compose port`.
- **Status:** Needs review

### Smoke and contract tests run on a bespoke HTTP test framework

- **What:** `scripts/smoke/` (about 2,600 lines) is its own test runner, with a probe DSL, suites, surfaces, verdicts, a concurrency pool, retries, latency budgets, and a reporter.
- **Where:** `scripts/smoke/`
- **Why it's unusual:** it gives up filtering, watch mode, standard reporters, and `test.each`, and the reinvented pieces drift. The "p95" over five samples is the maximum. `target.ts` can call `process.exit` at import time. `surfaces.ts` takes the first argument that is not `--worktree` as the target. So, from a reading of the code, `bun run smoke:dev --suite search` fails as an unknown target.
- **Normal approach:** `bun test` with `describe.each`/`test.each` over the probe tables (or Hurl for black-box HTTP). OpenAPI conformance already uses the standard tool, Schemathesis (`scripts/api/conformance.ts`).
- **Status:** Needs review

### Shared constants are hand-copied into Rust and infra files and synced by regex

- **What:** 29 `config.ts` values are copied into `apps/api/src/constants.rs`, and more are pinned as literals in nginx, Astro, Compose, the Elasticsearch schema, and `vps.sh`. `scripts/check/constants.ts` parses them all with regexes to catch drift.
- **Where:** `config.ts`, `apps/api/src/constants.rs`, `scripts/check/constants.ts`, root `AGENTS.md`
- **Why it's unusual:** there are two sources of truth, plus a third file that encodes the mapping between them, down to Rust syntax. At least eight `config.ts` values have no TypeScript consumer except the checker: `X_PROFILE_URL`, `TELEGRAM_URL`, `GITHUB_URL`, `GITHUB_DB_DUMPS_URL`, `GITHUB_AVATARS_URL`, `RAAQIM_URL`, `X_INTENT_TWEET_URL`, and `MAX_TWEET_LENGTH`. The checker also hides them from knip.
- **Normal approach:** the API publishes its contract limits through the OpenAPI document, which already generates the TypeScript types. Anything else shared lives in one JSON or TOML file that Rust reads with `include_str!` and TypeScript imports.
- **Status:** Needs review

### API configuration is read three ways, and trusted proxy subnets are compiled in

- **What:** `Config::from_vars` is injected and tested, `sentry::Config::from_env` reads `std::env` directly, and `main.rs` reads `ENVIRONMENT` once more to start logging before `Config` exists. The trusted proxy networks are a compile-time constant parsed by a hand-written CIDR parser.
- **Where:** `apps/api/src/config.rs`, `apps/api/src/sentry.rs`, `apps/api/src/main.rs`, `apps/api/src/constants.rs::TRUSTED_PROXY_NETWORKS`, `apps/api/src/client_ip.rs`, `docker-compose.yml`, `docker-compose.dev.yml`, `apps/web/nginx.conf`, `scripts/dev/worktree.ts`
- **Why it's unusual:** it breaks the repo's own "no globals/singletons" rule. The `172.26` to `172.29` subnets appear in four places with no sync check, and production images also trust the dev subnets. If a Compose subnet changes, nginx becomes an untrusted peer and every public caller shares one anonymous bucket, with no error anywhere. The approved `client_ip` entry covers the trust policy, not compiling the ranges in.
- **Normal approach:** one `Config`, passed into `AppState` and the layers. The trusted proxies come from an environment variable (for example `TRUSTED_PROXIES`), parsed by the `ipnet` crate.
- **Status:** Needs review

### JSON-LD builders validate their own output at runtime and throw

- **What:** each schema.org builder builds an object from typed data, runs it through a hand-written valibot schema, and throws if validation fails.
- **Where:** `apps/web/src/lib/seo/json-ld/` (`document.ts::parseNode` and every builder), `apps/web/src/lib/seo/serialize-json-ld.ts`
- **Why it's unusual:** this is the app's own output, not untrusted input, and it is typed twice: a valibot schema, and a hand-written node type for each builder. Any empty field becomes a 500 for the whole page. Examples are one poem with an empty title in a taxonomy list, or a title that `sanitizeMetaText` reduces to `''`.
- **Normal approach:** plain builder functions typed with `schema-dts`, a unit test per shape, and no runtime schema.
- **Status:** Needs review

### The random poem has its own client and retry loop, plus a client-side copy of the redirect

- **What:** `/poems/random` uses raw `fetch`, a neverthrow `Result`, and a second backoff loop. `RandomPoemButton` then intercepts its `<a href="/poems/random">` to repeat the same lookup in the browser through the `/api/v1` proxy.
- **Where:** `apps/web/src/lib/api/random-poem.ts`, `apps/web/src/pages/poems/random.ts`, `apps/web/src/components/random-poem-button.tsx`, `apps/web/src/lib/api/proxy-allowlist.ts`
- **Why it's unusual:** it is a third API client with a second retry policy. Unlike `safeCall`, it retries every error, 429 included. neverthrow, which `docs/code-conventions.md` prescribes, is used only here, so the web app now has four failure conventions. `poems/random` is in the proxy allowlist only for the button. The button needs a `pageshow` handler to undo its own loading state after back navigation.
- **Normal approach:** `/poems/random` is in the OpenAPI document (`poems.random`), so call it through `apiServer` with the existing retry, and render a plain `<a href="/poems/random">`.
- **Status:** Needs review

### The home search submits on blur and moves focus itself

- **What:** the home search is a bare input with no `<form>`. It submits on Enter through a `keydown` handler, on the icon button, and on blur, and it places the caret and focus by hand.
- **Where:** `apps/web/src/components/search/use-search.tsx`, `apps/web/src/components/ui-extended/search-input.tsx`, `apps/web/src/components/search/search-container.tsx`
- **Why it's unusual:** tapping elsewhere or dismissing the mobile keyboard runs a search, and the clear button needs `onMouseDown={preventDefault}` to avoid it. The first click moves the caret to the end by hand, and focus jumps from the input to a results heading after every search. `tabIndex={-1}` on wrapper elements makes them take focus on click, which blurs the input and fires a search. The poets page, by contrast, uses a plain `<form method="GET" role="search">`.
- **Normal approach:** `<form role="search" onSubmit>` around `<input type="search" enterKeyHint="search">`, submitting only on Enter or the button, and an `aria-live` region to announce results.
- **Status:** Needs review

### Arabic digits are converted two ways

- **What:** digits are converted both by a lookup table (`toArabicDigits`) and by `Intl.NumberFormat('ar-SA')` (`formatArabicNumber`).
- **Where:** `apps/web/src/lib/arabic.ts`, `apps/web/src/lib/pagination.ts`
- **Why it's unusual:** pagination shows ungrouped digits ("١٢٣٤") while counts show grouped ones ("١٬٣٣٨").
- **Normal approach:** format every number with one `Intl.NumberFormat`.
- **Status:** Needs review

### JavaScript runtime semantics spread beyond `js.rs`

- **What:** the ETag is FNV-1a over UTF-16 code units, lengths are counted with `encode_utf16().count()`, and `go.rs` hand-writes `encodeURIComponent`.
- **Where:** `apps/api/src/cache.rs`, `apps/api/src/domain/poems.rs`, `apps/api/src/domain/search.rs`, `apps/api/src/routes/go.rs`
- **Why it's unusual:** the approved `js.rs` entry says not to reach for JavaScript semantics outside that module. No consumer recomputes the ETag, so hashing UTF-16 buys nothing.
- **Normal approach:** any stable hash over the bytes for the ETag (`sha2` is already a dependency), `chars().count()`, and `percent_encoding` or `url::Url::query_pairs_mut`.
- **Status:** Needs review

### `.env` is read by five different parsers

- **What:** five readers parse the root `.env`: bash `source`, Docker Compose, Bun's auto-load, a grep regex, and a custom dotenv parser. Dev passwords are hard-coded in four places.
- **Where:** `scripts/db/resolve-dump.sh`, `scripts/lib/tag-db-container.sh`, `scripts/db/encrypt-dump.sh`, `scripts/dev/set-dump-key.sh`, `scripts/dev/compose.sh`, `scripts/secrets/dotenv.ts`, `scripts/secrets/schema.ts`, `scripts/dev/run.ts`, `scripts/dev/service-urls.ts`, `scripts/deploy/build-images.ts`, `scripts/smoke/surfaces.ts`
- **Why it's unusual:** the parsers disagree on quotes and `$`. A `DUMP_KEY__*` passphrase with a space or a `$` works in Compose, but `source .env` changes it. The custom parser refuses `export FOO=` lines, which `compose.sh` accepts on purpose. `schema.ts` adds one more registry of variable names.
- **Normal approach:** one `.env.example` (or `${VAR:-default}` once in `docker-compose.dev.yml`) for dev defaults, and one loader instead of `source .env`.
- **Status:** Needs review

### Custom lint scripts overlap the installed linters

- **What:** `check:boundaries`, `check:no-parent-imports`, and `check:naming` scan sources and file names with regexes, while oxlint and dependency-cruiser already enforce overlapping rules.
- **Where:** `scripts/check/boundaries.ts`, `scripts/check/no-parent-imports.ts`, `scripts/check/naming.ts`, `scripts/lib/imports.ts`, `.oxlintrc.json`, `.dependency-cruiser.cjs`
- **Why it's unusual:** cross-app imports are checked twice, and so are cycles. The regex import extractor misses dynamic `import()`, and can match text inside strings and comments. oxlint ships `import/no-relative-parent-imports`, which is not used. It also ships `unicorn/filename-case`, which is explicitly off, next to a 137-line naming checker.
- **Normal approach:** dependency-cruiser rules for cross-app and parent imports, and `unicorn/filename-case` or ls-lint for naming.
- **Status:** Needs review

### A search schema change is never reindexed by a deploy

- **What:** the indexer skips the rebuild whenever both aliases hold documents, whatever the mapping, and the deploy has no reindex step for `schema.json` changes.
- **Where:** `apps/search-indexer/src/main.rs`, `crates/elasticsearch/schema.json`, `crates/elasticsearch/AGENTS.md`, `scripts/deploy/vps.sh`, `.claude/skills/deploy/SKILL.md`
- **Why it's unusual:** this goes past the approved init-job entry, which covers data-only changes. A deploy ships an API built against the new mapping while the live index keeps the old one until someone remembers `reindex:prod`.
- **Normal approach:** put a schema version or hash in the index name, and reindex when the alias points at a different version.
- **Status:** Needs review

### The whole poem hydrates as a React island for the reading toolbar and a private `#h=` fragment

- **What:** `PoemDisplay` renders the title, byline, and every verse as a `client:idle` island. The client-side needs are the reading toolbar (font size, spacing, and tashkeel, which reset on every load, and the theme toggle) and highlighting from a homemade `#h=term1,term2` fragment.
- **Where:** `apps/web/src/pages/poems/[slug].astro`, `apps/web/src/components/poem-display.tsx`, `apps/web/src/components/poem-toolbar.tsx`, `apps/web/src/lib/urls.ts`, `apps/web/src/lib/highlight.ts`
- **Why it's unusual:** the verses ship twice, as HTML and as serialized props. The scale is applied twice: an inline `fontSize` on every hemistich, and a `--poem-scale` variable.
- **Normal approach:** render the poem statically in Astro, and keep a small island for the toolbar. Highlight with Text Fragments (`#:~:text=`), or with a small script that uses the CSS Custom Highlight API.
- **Status:** Needs review

### The home page hand-copies the search UI as a placeholder and deletes it by DOM query

- **What:** `index.astro` duplicates the search UI as a static, inert `#search-shell` overlay, and the `client:only` island removes it on mount with `document.querySelector('#search-shell')?.remove()`.
- **Where:** `apps/web/src/pages/index.astro`, `apps/web/src/components/search/search-with-providers.tsx`
- **Why it's unusual:** the markup, classes, and icons of the shell must be kept the same as `search-container.tsx` and `search-input.tsx` by hand. The icons are drawn again with `icon.astro`, not lucide. It is also unusual for an island to reach outside its own root to delete page DOM.
- **Normal approach:** Astro's `<div slot="fallback">` inside the `client:only` component, which Astro swaps out on load, or server-render the island with `client:load`.
- **Status:** Needs review

### The mobile menu is a hand-rolled drawer

- **What:** the mobile navigation is a fixed `div`, opened by a script. The script toggles classes, `inert`, `aria-hidden`, the `tabIndex` of each link, and `body.style.overflow`. It also adds a document-level Escape listener.
- **Where:** `apps/web/src/components/layout/nav.astro`
- **Why it's unusual:** focus never moves into the menu on open or back to the button on close. The manual `tabIndex` and `aria-hidden` repeat what `inert` already does. The settings panel in the same app uses a native `<dialog>`, so there are two overlay implementations.
- **Normal approach:** a `<dialog>` opened with `showModal()`, which provides the focus trap, Escape, an inert background, and focus return. A shadcn Sheet also works.
- **Status:** Needs review

### API errors render through several paths and four problem structs

- **What:** handlers return a bare status with a `Problem` in the response extensions, which `error::layer` re-renders. The limiter and the account guard call `render_at` directly, and `/account` JSON bodies fall back to axum's plain-text `Json` rejection.
- **Where:** `apps/api/src/error.rs`, `apps/api/src/openapi.rs` (`ProblemDetail`), `apps/api/src/rate_limit.rs`, `apps/api/src/routes/account.rs`, `apps/api/src/extract.rs`
- **Why it's unusual:** handing a value from `IntoResponse` to a middleware through response extensions is a rare trick. Error output is inconsistent, since some rejections are not problem+json, and four structs (`RouteProblem`, `Problem`, `ProblemBody`, `ProblemDetail`) describe one wire shape.
- **Normal approach:** one `ProblemDetail` deriving `Serialize` and `ToSchema`, returned by `impl IntoResponse for AppError`, with extractors wrapped through `#[from_request(via(axum::Json), rejection(AppError))]`.
- **Status:** Needs review

### The API uses a hand-rolled CORS middleware

- **What:** `cors::layer` answers every `OPTIONS` request, on any path including `/account`, with a 204 preflight, and stamps `*` on everything.
- **Where:** `apps/api/src/cors.rs`, `apps/api/Cargo.toml` (no `tower-http`)
- **Why it's unusual:** CORS is a solved problem, and the layer skips checks that `CorsLayer` makes, such as requiring `Access-Control-Request-Method` on a preflight.
- **Normal approach:** `tower_http::cors::CorsLayer` configured with the same origin, methods, headers, and max age, applied to the `/v1` nest.
- **Status:** Needs review

### OpenAPI constraints are patched in a post-processing pass

- **What:** `openapi::finish` walks the generated document to set `maxItems` and `default` on facet arrays by string-matching `$ref`s, and rewrites error content types to `application/problem+json`.
- **Where:** `apps/api/src/openapi.rs::finish`, `apps/api/src/routes/search.rs`
- **Why it's unusual:** utoipa supports `max_items` on params and `#[content("application/problem+json")]` on responses, and `search.rs` already sets `max_items = 2` inline for `types`. The same constraint is expressed two ways, and the walk depends on schema names inside `$ref` strings.
- **Normal approach:** attributes on each param (or an `IntoParams` struct) and on the error response variants.
- **Status:** Needs review

### The settings store is built for more than one value

- **What:** a module cache persists through a versioned, forward-compatible schema, with migration of legacy keys. All of this holds only `theme`.
- **Where:** `apps/web/src/lib/settings/settings-store.ts`, `apps/web/src/lib/settings/settings-storage.ts`, `apps/web/src/lib/settings/settings-schema.ts`
- **Why it's unusual:** a whole schema layer (a version field, a fallback for each field, and the keeping of unknown keys) guards one value. `applyTheme` also repeats `theme-init.astro`'s apply logic, while the approved entry covers only the parse duplication.
- **Normal approach:** read and write the one `theme` key in `localStorage` directly.
- **Status:** Needs review

### Outbound links go through the API's `/v1/go/*` redirector

- **What:** About page and footer links point to `api.qafiyah.com/v1/go/{x,github,telegram,db,avatars,raaqim}`, while the same destinations exist as web constants for JSON-LD `sameAs`.
- **Where:** `apps/web/src/lib/urls.ts`, `apps/web/src/pages/about.astro`, `apps/web/src/components/footer.tsx`, `apps/web/src/lib/constants/site-meta.ts`, `apps/api/src/routes/go.rs`, `apps/api/src/lib.rs`
- **Why it's unusual:** each click is a round trip through the data API, and counts against the visitor's anonymous quota. So a spent quota gives a 429, not a redirect. There are also two sources of truth for the URLs.
- **Normal approach:** link to the external URL directly from one web constant.
- **Status:** Needs review

### Branded slug types are never validated

- **What:** seven valibot brand schemas exist, six are used only to derive types, and routes cast `Astro.params` to the brand.
- **Where:** `apps/web/src/lib/api/brands.ts`, `apps/web/src/pages/poems/[slug].astro`, `apps/web/src/pages/poets/[slug].astro`, `apps/web/src/pages/poets/index.astro`, `apps/web/src/lib/seo/taxonomy-copy.ts`
- **Why it's unusual:** the brands suggest a validation that never happens. A lint-disable in `taxonomy-copy.ts` claims "the route validates each slug before the lookup builds its brand". But the taxonomy routes pass `Astro.params.slug` straight through. Casts in `.astro` files also escape the no-cast rule of oxlint.
- **Normal approach:** `v.safeParse` at the route with a 404 on failure, or plain `string`, since the API already validates and its 400 becomes a 404.
- **Status:** Needs review

### The API's tests use homemade snapshot, randomness, and skip tooling

- **What:** ES query snapshots go through a dedicated `es-query-dump` binary, a Bun script, and a committed vectors file. An xorshift `Rng` drives the "never panics" loops, and DB tests return early, and so pass, when `QAFIYAH_TEST_*` is unset.
- **Where:** `apps/api/src/bin/es-query-dump.rs`, `scripts/es/query-snapshot.ts`, `apps/api/generated/es/query.vectors.json`, `apps/api/src/test_support.rs`, `apps/api/tests/db.rs`, `apps/api/src/es/query.rs`, `apps/api/src/domain/search.rs`
- **Why it's unusual:** each piece is a homemade version of a standard crate. The dump binary ships in the release build, `rand` is already a dependency, and a skipped DB test shows up as a pass.
- **Normal approach:** `insta::assert_json_snapshot!`, `proptest` (or a seeded `rand` RNG), and `#[ignore = "needs QAFIYAH_TEST_*"]`.
- **Status:** Needs review

### Text files and snapshots go through custom codegen and duplicate checks

- **What:** three `well-known/` templates are escaped into template literals in a generated TypeScript module with a drift check. The OpenAPI and ES-query snapshots are checked both by Rust tests and by `--check` scripts that `cargo run` a dump binary.
- **Where:** `scripts/well-known/generate.ts`, `apps/web/src/lib/generated/well-known/`, `apps/web/src/pages/robots.txt.ts`, `apps/web/src/pages/llms.txt.ts`, `apps/web/src/pages/.well-known/security.txt.ts`, `scripts/openapi/snapshot.ts`, `scripts/es/query-snapshot.ts`, `scripts/ci.ts`
- **Why it's unusual:** Vite imports text files as strings with `?raw`, and the API side already uses `include_str!` for the same files. The contract phase recompiles and repeats a comparison that `cargo test` already makes.
- **Normal approach:** a `?raw` import through an alias, and one snapshot mechanism (typically `insta`).
- **Status:** Needs review

### A custom query serializer duplicates openapi-fetch's default

- **What:** `serializeQuery` is passed to both web API clients.
- **Where:** `apps/web/src/lib/api/query-string.ts`, `apps/web/src/lib/server/client.ts`, `apps/web/src/lib/api/browser-client.ts`
- **Why it's unusual:** openapi-fetch's default serializer already does form/explode arrays and skips `undefined`, `null`, and empty arrays. That is the encoding the API was moved to so that any generated client would work.
- **Normal approach:** drop `querySerializer` and use the default.
- **Status:** Needs review

### Poets are served from two stores with different count types

- **What:** `GET /poets` reads Elasticsearch, while `GET /poets/{slug}` and `/poets/slugs` read Postgres.
- **Where:** `apps/api/src/routes/poets.rs`, `apps/api/src/es/search.rs`, `apps/api/src/db/poets.rs`, `apps/api/generated/openapi/openapi.json`
- **Why it's unusual:** the two reads can disagree on counts until a reindex. The contract also shows the same `poemsCount` as `int64` in `PoetListItem`, and as `int32` in `PoetStats`.
- **Normal approach:** one source per resource, or at least one type for the shared field.
- **Status:** Needs review

### The footer is one React island

- **What:** `<Footer client:idle />` hydrates the whole footer, static links included, only to host `RandomPoemButton`.
- **Where:** `apps/web/src/layouts/layout.astro`, `apps/web/src/components/footer.tsx`
- **Why it's unusual:** static markup ships as JavaScript on every page. Because the footer wraps itself in `IslandErrorBoundary fallback={null}`, a crash in the button removes the entire footer.
- **Normal approach:** a `footer.astro` with one small island.
- **Status:** Needs review

### Poem result cards navigate with JavaScript

- **What:** `PoemCard` is a `div` whose `onClick` calls `window.location.assign(href)`, while `PoetCard` and `ListCard` are plain `<a>` elements.
- **Where:** `apps/web/src/components/search/result-cards.tsx`, `apps/web/src/components/ui-extended/list-card.tsx`
- **Why it's unusual:** middle-click, modifier-click, and link previews do not work on the card body. The handler must also handle text selection and nested links as special cases.
- **Normal approach:** the stretched-link pattern: the title `<a>` gets `after:absolute after:inset-0`, and nested links are raised with `relative z-10`.
- **Status:** Needs review

### The account control is mounted twice, and each copy fetches separately

- **What:** the nav renders `<AccountControl client:load compact />` and `<AccountControl client:load />` as separate React roots. Each fetches `/api/me` and may re-encode the avatar into localStorage.
- **Where:** `apps/web/src/components/layout/nav.astro`, `apps/web/src/components/layout/account-control.tsx`
- **Why it's unusual:** every page makes two identical session requests, and the two copies race on the same localStorage key. The approved viewer-init entry covers the pre-paint mechanism, not the double mount.
- **Normal approach:** one island styled responsively, or one shared fetch.
- **Status:** Needs review

### An explicit `?page=1` returns 404

- **What:** `parsePageQuery('1')` returns `null`, so `/poets?page=1` and every taxonomy term page with `?page=1` render the 404 page. A second parser, `parsePageParam`, accepts 1.
- **Where:** `apps/web/src/lib/pagination.ts`, `apps/web/src/pages/poets/`, `apps/web/src/pages/{meters,rhymes,themes,collections}/[slug].astro`
- **Why it's unusual:** users and crawlers expect a non-canonical URL to lead to the canonical one, not to disappear.
- **Normal approach:** a 301 to the bare URL, or accept it and rely on `rel=canonical`.
- **Status:** Needs review

### The home page carries text hidden from everyone but crawlers

- **What:** `SITE_DESCRIPTION` renders in a `<p>` that is `sr-only`, `opacity-0`, `aria-hidden="true"`, and `tabindex="-1"` at once.
- **Where:** `apps/web/src/pages/index.astro`
- **Why it's unusual:** `sr-only` plus `aria-hidden` hides the text from sighted users and assistive technology alike, which search engines treat as hidden-text spam. `text-opacity-0` is a Tailwind v3 class with no effect in v4.
- **Normal approach:** show the description visibly, or rely on the `<meta name="description">` that is already set.
- **Status:** Needs review

### Filter URL state re-implements nuqs's array parser

- **What:** the five filter params are stored as plain strings, then split and joined by hand with `splitCsvIds` and `createCsvFilterSetter`.
- **Where:** `apps/web/src/components/search/use-search.tsx`, `apps/web/src/lib/search/csv-filters.ts`
- **Why it's unusual:** nuqs ships a parser for exactly this.
- **Normal approach:** `useQueryStates` with `parseAsArrayOf(parseAsString).withDefault([])` per filter.
- **Status:** Needs review

### Search results use two load-more patterns and constant flags

- **What:** poets load more through a hand-rolled IntersectionObserver, in a horizontal row. Its two rows are built by a filter on `index % 2`. Poems load more through a button. `wantPoems` and `wantPoets` are hard-coded `true`, but are passed through three components.
- **Where:** `apps/web/src/components/search/sections.tsx`, `apps/web/src/components/search/use-search.tsx`, `apps/web/src/components/search/search-container.tsx`, `apps/web/src/components/search/filters.tsx`
- **Why it's unusual:** one results page has two load-more UIs, and the flags leave dead branches.
- **Normal approach:** one load-more pattern, CSS `grid-flow-col grid-rows-2` for two rows, and no constant flags.
- **Status:** Needs review

### `lib/seo` fetches data and holds page copy

- **What:** the SEO layer loads taxonomy data. It holds page headings and empty-state text in config tables whose values are functions. The sitemap imports its data loading from there.
- **Where:** `apps/web/src/lib/seo/taxonomy-pages.ts`, `apps/web/src/lib/seo/taxonomy-copy.ts`, `apps/web/src/pages/sitemap/taxonomies.xml.ts`, and the taxonomy `[slug].astro` pages
- **Why it's unusual:** metadata builders that reach into `lib/server` couple SEO to data access, and what a taxonomy page renders is spread over three files.
- **Normal approach:** pages (or `lib/server`) fetch the data, and `lib/seo` only turns the data it is given into a title, description, and JSON-LD.
- **Status:** Needs review

### `@qafiyah/config` is a path alias, not a workspace package

- **What:** root `config.ts` is reached through tsconfig `paths` in three tsconfigs, re-declared as a Vite alias for vitest, and listed as a turbo global dependency.
- **Where:** `config.ts`, `apps/web/tsconfig.json`, `apps/inspector/tsconfig.json`, `scripts/tsconfig.json`, `apps/web/vitest.config.ts`, `turbo.json`
- **Why it's unusual:** it looks like a package but is not one, so every consumer re-declares it. As a global dependency, any constant change invalidates every turbo task's cache.
- **Normal approach:** a `packages/config` workspace package with `exports`, depended on as `"@qafiyah/config": "workspace:*"`.
- **Status:** Needs review

### The inspector duplicates the smoke SEO suite

- **What:** a dev-only workspace app crawls the site, and parses HTML with the same regexes and thresholds as `scripts/smoke/suites/seo.ts`. It uses hard-coded samples that depend on the data, such as `/poets/GuRy`.
- **Where:** `apps/inspector/src/`, `scripts/smoke/suites/seo.ts`, `apps/inspector/AGENTS.md`
- **Why it's unusual:** the approved inspector entries cover its mechanics, not its existence or the duplication, which its `AGENTS.md` calls deliberate without giving a reason.
- **Normal approach:** Lighthouse or Unlighthouse SEO audits, or reporting the smoke suite's results. If a custom tool stays, use `HTMLRewriter` for parsing.
- **Status:** Needs review

### The database container is renamed after each restore

- **What:** after a restore, `tag_db_container` reads `COMMENT ON DATABASE` and runs `docker rename` on the Compose-managed container, giving it the name `<name>-<dump number>`.
- **Where:** `scripts/lib/tag-db-container.sh`, `scripts/db/init.sh`, `scripts/dev/run.ts`, `scripts/dev/db-up.sh`, `scripts/deploy/vps.sh`, `scripts/db/reseed.sh`, `docs/deployment/architecture.md`
- **Why it's unusual:** it renames a container that Compose owns, only to show metadata in `docker ps`. The helper is also shipped over SSH on every deploy.
- **Normal approach:** a label, a status command, or the init log.
- **Status:** Needs review

### Plan limits are hard-coded in the developer page

- **What:** `RateLimits` hard-codes 60 anonymous and 500 free requests per hour, values the API owns (`ANON_REQUESTS` and the `plans` rows).
- **Where:** `apps/web/src/components/developers/rate-limits.astro`, `apps/web/src/pages/account/index.astro`
- **Why it's unusual:** it creates two sources of truth that can drift. The account page already shows the real value from the API, right above the table.
- **Normal approach:** read the limits from the API, or share one constant.
- **Status:** Needs review

### The Rust lint wall enables clippy's restriction group

- **What:** the workspace denies `arithmetic_side_effects`, `as_conversions`, `indexing_slicing`, `string_slice`, `print_stdout`, `print_stderr`, `exit`, `wildcard_enum_match_arm`, and more.
- **Where:** `Cargo.toml` (`[workspace.lints]`), `docs/rust-conventions.md`, and the `#[expect(..)]` sites across `apps/api` and `crates/elasticsearch`
- **Why it's unusual:** clippy's documentation says that restriction lints are not meant to be enabled as a whole group. Here they cause 26 `#[expect]` sites, and repeated `match`, `eprintln!`, and `exit` blocks in `main.rs`. They also cause saturating math, which clamps without a message instead of showing an overflow. `overflow-checks` is also on in release.
- **Normal approach:** `clippy::all` plus selected pedantic lints, `unwrap_used` and `expect_used` if wanted, and `fn main() -> anyhow::Result<()>` for boot errors.
- **Status:** Needs review

### Operational runbooks live in agent skill files

- **What:** human-facing docs point to `.claude/skills/deploy/SKILL.md` and `.claude/skills/local-db-edit/SKILL.md` as the ordered runbook for deploy, rollback, reseed, and dump creation.
- **Where:** `.claude/skills/`, `data/db/MAINTAINERS_GUIDE.md`, `docs/deployment/README.md`, `docs/topology.md`
- **Why it's unusual:** a maintainer has to find procedures inside a tool-specific directory that `.gitignore` otherwise excludes.
- **Normal approach:** keep runbooks in `docs/deployment/` and have the skill point to them.
- **Status:** Needs review

### Scripts re-implement command-line basics, each in its own way

- **What:** about 19 scripts parse `process.argv` by hand (one defines its own `parseArgs`). There are four ANSI colour helpers, three ways to spawn processes, and three ways to list files.
- **Where:** `scripts/ci.ts`, `scripts/dev/run.ts`, `scripts/smoke/run.ts`, `scripts/smoke/surfaces.ts`, `scripts/dev/quota.ts`, `scripts/deploy/build-images.ts`, `scripts/lib/walk.ts`, `scripts/lib/tracked-files.ts`, `scripts/check/naming.ts`
- **Why it's unusual:** `ci.ts` and `run.ts` each carry copies of `paint`, `ensureDockerRunning`, the turbo and astro banner filters, and `formatMs`. The copies have drifted: the colours of `quota.ts` ignore `NO_COLOR` and TTY. Hand-written parsing caused the smoke `--suite` bug noted above.
- **Normal approach:** `parseArgs` and `styleText` from `node:util`, one spawn API, and `git ls-files` or `Bun.Glob` for file lists.
- **Status:** Needs review

### Next.js scaffolding is left in an Astro app

- **What:** 15 files start with `'use client'`. `components.json` declares `"rsc": true`, with `ui` and `hooks` aliases that point at directories that do not exist. `vitest.config.ts` declares the tsconfig aliases again by hand.
- **Where:** `apps/web/components.json`, `apps/web/vitest.config.ts`, and the `'use client'` files under `apps/web/src/`
- **Why it's unusual:** the directive means nothing in Astro, and suggests React Server Components to a new maintainer. `shadcn add` would write components to the wrong place, with RSC markers.
- **Normal approach:** `rsc: false` with aliases that match the tree, no directives, and Astro's `getViteConfig()` for vitest.
- **Status:** Needs review

### Two icon systems

- **What:** React code uses `lucide-react`, while Astro code uses `icon.astro`, a hand-kept map of raw SVG paths (copies of Lucide icons) injected with `set:html`.
- **Where:** `apps/web/src/components/ui/icon.astro`
- **Why it's unusual:** the copies can drift from the library, and the home page placeholder has to redraw the island's icons by hand.
- **Normal approach:** `@lucide/astro` (or `astro-icon` with the Lucide set) in `.astro` files.
- **Status:** Needs review

### Styling does the same things several ways

- **What:** inline `style` objects repeat Tailwind classes on the same element, and two scrollbar-hiding mechanisms coexist. `design-tokens.ts` maps names to identical class strings, with a test asserting the mapping. Footer links skip the `focus-ring` utility, and `dir="rtl"` is set on inner elements under an `<html dir="rtl">`.
- **Where:** `apps/web/src/components/ui-extended/select.tsx`, `apps/web/src/components/search/highlighted-text.tsx`, `apps/web/src/styles/globals.css` (`.search-results-scroll`), `apps/web/src/pages/poets/index.astro` (`scrollbar-none`), `apps/web/src/lib/constants/design-tokens.ts`, `apps/web/src/components/footer.tsx`, `apps/web/src/layouts/layout.astro`
- **Why it's unusual:** with several ways to do each of these things, a reader cannot tell which is the convention.
- **Normal approach:** Tailwind utilities and `@theme` tokens used directly, one scrollbar utility, and `dir` only on the root.
- **Status:** Needs review

### Not every island has the approved error boundary

- **What:** the approved entry says every React island renders inside `IslandErrorBoundary`, but `PoemDisplay` and `AccountControl` hydrate without one.
- **Where:** `apps/web/src/pages/poems/[slug].astro`, `apps/web/src/components/layout/nav.astro`, `apps/web/src/components/island-error-boundary.tsx`
- **Why it's unusual:** the approved rule and the code disagree, so one of them is wrong.
- **Normal approach:** wrap those islands, or narrow the approved entry.
- **Status:** Needs review

### Stale and dead root config

- **What:** several config entries refer to things that no longer exist.
- **Where:**
  - `.oxlintrc.json`: it bans an `@qafiyah/api` package "via oRPC over HTTP", and targets code in `packages/*`, which holds only tsconfig JSON.
  - `.dependency-cruiser.cjs`: the same `packages/*` rules.
  - `turbo.json`: `SMOKE_*` variables that no task uses.
  - `scripts/check/naming.ts`: it ignores a `tools` directory that does not exist, and most of `ALLOWED_BASENAMES` has no effect.
  - `.gitignore`: `packages/schemas/...`, `.next/`, `.vercel`, Python venv entries, `title-review.sqlite`, `.title-fix/`.
  - `.dockerignore`: `.next`, `.vercel`, `tools/**/venv`.
  - `.vscode/settings.json`: `editor.rulers: [80]`, against `printWidth: 100`.
- **Why it's unusual:** a reader has to confirm each rule is dead before trusting the rest.
- **Normal approach:** delete them.
- **Status:** Needs review

### Snippets highlight the whole poem and the API picks the verse

- **What:** the poem search asks Elasticsearch to highlight the whole `content` field (`number_of_fragments: 0`). Then `domain/search.rs` splits it into its stored rows (joined by a newline in the index). It balances the `<mark>` tags across hemistichs and rows again, and keeps the row with the longest marked run.
- **Where:** `apps/api/src/es/query.rs` (`poem_highlight`), `apps/api/src/domain/search.rs` (`poem_snippet`, `balanced_rows`), `docs/search.md` ("Snippets")
- **Why it's unusual:** the highlighter already picks and ranks fragments. Here it returns the full text, and Rust makes the choice again. It may well be justified:
  - Measured on 2026-10-02 over 180 queries on the dev index, whole-field highlighting costs the same as one fragment of 120 characters: a mean of 28.9 ms against 30.5 ms, and 5.9 ms with no highlighting.
  - While rows were joined by `*`, no fragmenter could cut on the row separator. Rows are now joined by a newline, which a sentence boundary scanner may cut on. So one fragment for each row is worth a measurement.
  - Since 2026-10-02, `content` indexes its offsets. That made the highlighting of a page that holds the longest poems eight to fourteen times faster.
  - What whole-field highlighting still costs is bytes. Such a page is an Elasticsearch response of 2 to 4 MB, with the poem sent once highlighted and once in `_source`, for an API response of about 9 KB.
  - Leaving `content` out of `_source` (with `no_match_size` for the fallback to the opening verse) halved those bytes, but saved under 2 ms on the heaviest pages. So it was not done.
- **Normal approach:** `number_of_fragments: 1` with a `fragment_size` and a boundary scanner, or one document per verse searched with `inner_hits`.
- **Status:** Needs review

## API (`apps/api`)

The API is not only a thin database connector. The crate has no doc comments beyond the field descriptions of its public response types, so these entries are its module-level intent. Read them before you assume that something is accidental.

### Client address comes from proxy headers only behind the web nginx

- **What:** `client_ip.rs` resolves the caller from `CF-Connecting-IP`, falling back to the last `X-Forwarded-For` hop, and to a single shared bucket when neither is present. For an internal-key request, `forwarded` reads only `CF-Connecting-IP`, and a missing header means no visitor limit rather than a shared bucket.
- **Where:** `apps/api/src/client_ip.rs`
- **Why:** the headers are honored only when the immediate peer of the connection is on the dedicated `backend` network. Only the API and the web container are on that network. The web nginx sets the header for `api.qafiyah.com`, and its Astro proxy sets it for website visitors. A caller outside that subnet is counted by its own peer address. So a lateral container on the default bridge cannot forge the header.
- **Normal approach:** trust the forwarding headers from any peer behind a single reverse proxy.
- **Date:** 2025-04-13

### API key lookups are cached, misses included

- **What:** `accounts/` resolves an `x-api-key` to a `Caller`. It joins `api_keys` to `users` to `plans` in the separate `qafiyah_accounts` database, behind a 60-second in-memory cache that also caches misses.
- **Where:** `apps/api/src/accounts/cache.rs`, `apps/api/src/accounts/keys.rs`
- **Why:** caching misses stops key spraying from becoming a database amplifier.
  - At its ceiling, the cache flushes completely instead of evicting. Entries rebuild cheaply, and the ceiling only limits memory under spraying.
  - Only a key with the exact shape that `keys::generate` emits (`qaf_` and 32 ASCII alphanumerics) reaches the cache or the database. Anything else is anonymous.
  - A 500 ms timeout limits a lookup that hangs or fails, and the cache keeps it as a 5-second miss. So an accounts outage costs one probe for each key in each TTL, not one stall for each request.
- **Normal approach:** query the database per request, or use an LRU cache crate with per-entry eviction.
- **Date:** 2026-09-21

### Every rate limit is checked in one pass

- **What:** `Limiter::check_all` evaluates every applicable limit together, and increments every bucket or none.
  - A refused request adds no bucket. So a caller who rotates addresses past a ceiling cannot grow the map toward its flush.
  - When the map still fills with live windows, the per-address buckets (`Ip`, `KeyedIp`, `Visitor`) are dropped first. So users' quotas and the /48 caps survive. Only a map that is still full after that is cleared.
  - IPv4 has no aggregate bucket, so its per-address counts do reset on such a drop. To force one takes the allowed requests of about 14 new /48s, and uses up those /48s for the rest of the hour.
- **Where:** `apps/api/src/rate_limit.rs`
- **Why:** a keyed caller has three limits:
  - the plan's `requests` for each `WINDOW_SECONDS`, counted on `users.id`
  - the plan's `burst` for each `BURST_WINDOW_SECONDS`, counted on the same user
  - the plan's `ip_ceiling`, counted on the client address. It is left out when the plan has no ceiling (only `free` has one). It is also left out when the address cannot be resolved, so that such callers do not all fall into one bucket.

  Three separate `check` calls would let a refusal by address use up the caller's own hourly allowance without a message. So a refused request uses none of the limits that allowed it. The response headers describe the hourly limit that is closest to running out. They leave out the one-second burst, so they always mean requests per hour. `Retry-After` on a 429 follows the limit that refused. The quota is counted on the user, never on the key. So rotating keys, or holding several, gives no extra allowance.

- **Normal approach:** one limiter middleware per limit, each checked in turn.
- **Date:** 2026-09-22

### The anonymous limit depends on the environment

- **What:** callers without a key share a bucket for each address: a /64 for an IPv6 caller, and its /48 shares ten times that. `ANON_REQUESTS` sets the size. It defaults to 60 under `ENVIRONMENT=production`, and to an unlimited value elsewhere.
- **Where:** `apps/api/src/config.rs::anon_requests`
- **Why:** outside production there is no Cloudflare in front, so every caller would share one bucket.
- **Normal approach:** one fixed default in every environment.
- **Date:** 2026-09-21

### Internal keys bypass rate limiting and the accounts database

- **What:** `API_KEY_INTERNAL` and `API_KEY_FULL` skip every limit, and never touch the accounts database. Every caller gets the same response body (no scope, no capping, no `Vary: x-api-key`).
  - One exception: an internal-key request that carries a visitor address, forwarded by a trusted peer, counts against that visitor. The website's search proxy sets that address.
  - The visitor limit is `VISITOR_REQUESTS` an hour for each /64 or IPv4 address, and ten times that for each /48, in buckets separate from those of anonymous callers.
- **Where:** `apps/api/src/auth.rs`, `apps/api/src/rate_limit.rs`, `apps/api/src/client_ip.rs`
- **Why:** an accounts outage harms the portal, not the website. During such an outage, a keyed caller falls back to the anonymous bucket, and is not refused. This matches the crawler policy that the API serves from `well-known/robots.api.txt`. The visitor limit exists because the proxy's only other limit for each visitor, nginx's `limit_req`, counts the exact address. So an IPv6 visitor could rotate inside a /64 without a limit. Only a forwarded address counts, never the connection's own. So the site's server-side renders stay unlimited, and crawlers are never limited.
- **Normal approach:** resolve every key, internal ones included, through the same accounts lookup and plan limits.
- **Date:** 2026-06-30, visitor limit 2026-09-28

### Key revocation, plan changes, and usage are eventually consistent

- **What:** resolved keys are kept for `API_KEY_CACHE_TTL_SECONDS` (up to 60 seconds), with no invalidation across processes. The `usage_hourly` counters are in memory. They flush once a minute, and drop that minute when a flush fails.
- **Where:** `apps/api/src/accounts/cache.rs`, `apps/api/src/accounts/usage.rs`
- **Why:** a revoked key keeps working until its entry expires in each replica; shortening the TTL trades a larger database load for a smaller window. Usage is a reporting number, not billing, so a failed flush is not retried.
- **Normal approach:** invalidate across processes through a shared cache, and write usage per request or retry failed writes.
- **Date:** 2026-09-21

### `/account/*` is protected three ways

- **What:** denied at the edge, guarded by `API_KEY_INTERNAL` alone, and merged outside both the `cached` and `limited` routers, with `no-store` on every response.
- **Where:** `apps/web/nginx.conf` (404 for `^~ /account` on `api.qafiyah.com`), `apps/api/src/routes/account.rs::guard`, `apps/api/src/auth.rs::Keys::is_internal`
- **Why:** `guard` accepts only what `Keys::is_internal` matches. So neither a user's own API key nor `API_KEY_FULL` opens it. This is true although `API_KEY_FULL` skips the limiter on `/v1`, and a user's key raises its quota there. Staying outside `cached` means that `cache::layer` never puts `private, max-age=300` on a session payload. Any one of the three would usually be enough. The point is that a mistake in one is survivable, because a shared-cache hit is served without ever reaching the origin.
- **Normal approach:** a single auth middleware on the routes.
- **Date:** 2026-09-21

### `users::upsert` trusts its caller about email verification

- **What:** `accounts/users.rs::upsert` does not check verification; `exchange.ts` in web requires Google's `email_verified` to be true and GitHub's email to be both primary and verified.
- **Where:** `apps/api/src/accounts/users.rs`, `apps/web/src/lib/server/oauth/exchange.ts`
- **Why:** web owns OAuth, so it is the side that sees the provider's claims. Linking accounts by email without the check is an account-takeover vector.
  - The upsert resolves by `(provider, provider_uid)` first, and updates the email of that account. So a change of email at the provider keeps the same account and its keys.
  - When the new email already belongs to a different account, the upsert is refused with a 409 `EMAIL_TAKEN`. It does not merge the accounts, and the keys of both accounts stay as they were.
- **Normal approach:** an auth library that verifies and links accounts where the user row is written.
- **Date:** 2026-09-21

### Shipped migrations are never edited, not even reformatted

- **What:** a fix is always a new migration. No formatter touches `.sql` files. A shipped migration that squawk flags is excluded in `.squawk.toml`, not edited.
- **Where:** `apps/api/migrations/`, `.squawk.toml`
- **Why:** a shipped migration has already run on prod, so an edit changes only fresh databases and the two drift apart. `bun run check:sql` runs squawk over new migrations (lock and timeout hazards, a `NOT NULL` column without a default, a blocking index build); `0001_accounts/up.sql` predates that and is excluded, and a squawk upgrade that flags a shipped migration gets the same exclusion. The one edit so far made `0001_accounts/up.sql` idempotent when the runner moved from sqlx to Diesel: Diesel keeps its own `__diesel_schema_migrations` table, so it applies `0001` once more on a database sqlx already migrated. `_sqlx_migrations` stays so an older build can still boot.
- **Normal approach:** fix lint and format findings in place.
- **Date:** 2026-09-23

### `js.rs` mirrors JavaScript runtime semantics

- **What:** ECMAScript's whitespace set and integer-safe number serialization, reimplemented in Rust.
- **Where:** `apps/api/src/js.rs`
- **Why:** the TS client must see exactly what JavaScript would produce. These are not general text utilities; don't reach for them outside that purpose.
- **Normal approach:** Rust's own `char::is_whitespace` and `serde_json` number output.
- **Date:** 2026-09-14

### The poem list picks its SQL shape for the planner

- **What:** the poem list has these special cases:
  - `resolve_ids` turns the slugs of every filter into ids, with one small query for each filter, pipelined on one connection. `list` returns an empty page without a query on `poems` when a filter matches no id.
  - `matching` builds `column = $n` for one id, and `column = ANY($n)` for several, always behind the shown-primary condition.
  - The ids of the page are picked in an `IN` subquery, ordered and limited, before poets and meters are joined.
  - The list rows, the count of several combined filters, and the grouped counts of the facets are sent unnamed through `db::Uncached`. Every other query is a cached prepared statement.
  - The total is the sum of the filter's `*_stats` rows (added up in Rust) when there is one filter, and of `meter_stats` when there is none. It is `COUNT(*)` only when several filters combine.
  - The facets group the matching poems by the counted column, and attach names and slugs from the taxonomy table, read in name order.
- **Where:** `apps/api/src/db/poems.rs`, `apps/api/src/db/mod.rs` (`Uncached`), `apps/api/src/db/taxonomy.rs`, `scripts/db/sql/refresh-taxonomy-stats.sql`
- **Why:** each special case is a measured planner win on the corpus.
  - Concrete ids let the planner read its statistics. With slug subqueries, a filter that matched no poem, or only a rare combination, walked every poem in id order: 113 to 145 ms before, 2 to 5 ms now (#127).
  - A scalar for one id lets Postgres walk the `(facet, id)` partial index in id order, and stop at the page. `= ANY` walks every poem, and `IN` joins and sorts every match. The last page of one theme takes 9.8 ms, against 44.9 ms with `= ANY` (#142). Its middle page takes 5.0 ms, against 22.7 ms with `IN`.
  - Picking the page by id before joining poets and meters keeps the skipped rows inside that index. This makes deep pages 8 to 14 times faster.
  - `Uncached` stops Postgres from choosing a generic plan that cannot see the ids. With every list query cached, a rare combination reached 76 ms after 400 common runs on the same connections, and 4.4 ms at most with `Uncached` (#138). It is Diesel's documented extension point: a `QueryFragment` wrapper (as in its `Paginated` guide example) that calls `unsafe_to_cache_prepared`. Diesel itself does the same for `sql_query`. Planning every query instead (`plan_cache_mode = force_custom_plan`) made the five cached lookups of the poem detail 0.9 ms slower.
  - The `*_stats` totals avoid a count of up to about 342,000 rows (the shown primaries in dump 0039) on every filtered or unfiltered page (#133, #135). A poem has exactly one value for each filter, so the rows add up exactly.
  - They agree with a live count only while `refresh_taxonomy_stats()` has run since the last change to `poems`. These tests guard that: `a_single_term_total_from_the_stats_table_equals_a_live_count_of_primaries`, `a_total_over_several_values_of_one_filter_equals_a_live_count_of_primaries`, and `the_unfiltered_total_and_the_poem_count_equal_a_live_count_of_primaries`. `a_planner_sensitive_list_is_never_kept_as_a_prepared_statement` guards `Uncached`.
- **Normal approach:** Diesel boxed queries, all cached, and a single `COUNT(*)` path.
- **Date:** 2026-09-24, updated 2026-10-03

### Famous lines rank by an era group with constant scores

- **What:** a poem search of three words or more puts the poems that hold the words as a phrase above every other result. It is a `dis_max`. Its verbatim side is a constant score for each era (101 to 109 million, with the oldest classical era highest). Its other side is the tier ladder. Exact search orders its hits in the same way.
- **Where:** `apps/api/src/es/query.rs` (`verbatim_by_era`, `verbatim_first`), `docs/search.md` ("Relevance")
- **Why:** many later poems quote a famous line, often as their title, and BM25 ranks the short later quotation first. The catalog has no popularity or citation data, and the production database stays read-only for the API. So the era of the poem is the only signal of where a line comes from. Measured on 76 famous lines, the source poem was first in 46 before, and in 67 after (exact search: 43 and 72). The constants are far above every tier score, so the group is a strict band. They also show in the `relevance` field.
- **Normal approach:** a popularity or canonical-source field fed to a `rank_feature` or a `function_score`.
- **Date:** 2026-10-02

## Web (`apps/web`)

Paths are relative to `apps/web/src/` unless they start at the repo root.

### Two API clients, and no key ever reaches the browser

- **What:** `apiServer` calls the internal API URL with `INTERNAL_API_KEY`, for server-side rendering. `apiBrowser` has no key, and calls `/api/v1` on the page's own origin. That is a proxy:
  - It forwards only the paths on the allowlist: `search`, `poems/random`, and the `poems` and `poems/facets` lists that the poet page fetches again in place. It forwards those two only for a request that names exactly one poet.
  - It attaches the internal key on the server.
  - It also attaches the visitor's address, from the `X-Real-IP` that nginx sets, and the API limits the rate of that address.
- **Where:** `lib/server/client.ts`, `lib/api/browser-client.ts`, `pages/api/v1/[...path].ts`, `lib/api/proxy-handler.ts`, `lib/api/proxy-allowlist.ts`
- **Why:** the allowlist is the security boundary: without it the route would be an unauthenticated tunnel to the whole corpus. Don't use one client from the other's context, and don't widen the allowlist without reading the API rate-limiting entries above.
- **Normal approach:** the browser calls the public API directly.
- **Date:** 2026-09-21

### Web owns identity; the API owns the accounts database

- **What:** OAuth, the `qaf_session` cookie, and every account screen are in web. Web never connects to Postgres. It calls `/account/*` on the API over the internal Docker network.
- **Where:** `lib/server/oauth/`, `lib/server/session.ts`, `pages/auth/`, `pages/account/`, `lib/server/account-client.ts`
- **Why:** `account-client.ts` is separate from `lib/server/client.ts` because that one bakes `/v1` into its base URL and the account routes are deliberately outside the public contract.
- **Normal approach:** the web server reads its own session and user tables through an auth library's database adapter.
- **Date:** 2026-09-21

### Authenticated pages opt out of the nginx cache, redirects included

- **What:** `/account`, `/api/me`, `/auth/`, and `/login` are in the `$skip_astro_cache` map. `@astro` passes that map to `proxy_no_cache` and `proxy_cache_bypass`. Every one of those responses sets `no-store`, the 302s included.
- **Where:** `apps/web/nginx.conf`
- **Why:** the astro cache zone keys on `"$host$uri$is_args$args"`, with no cookie. The skip must be read in `@astro`. Every location hands off with `try_files $uri @astro`. After that internal redirect, nginx applies only the directives of the named location. So a `proxy_no_cache` in the original location is ignored, with no message. A new authenticated route without both the map entry and `no-store` serves one reader's page to the next.
- **Normal approach:** bypass the cache whenever a session cookie is present.
- **Date:** 2026-09-21

### New server-side env vars go in `turbo.json`'s `dev.passThroughEnv`

- **What:** every server-side env var is listed in `dev.passThroughEnv` (today `INTERNAL_API_KEY`, the four OAuth variables, and `SESSION_STATE_SECRET`).
- **Where:** `turbo.json`, `lib/server/env.ts`
- **Why:** Turborepo gives the dev task an explicit allowlist. So anything missing from it reads as `''` in `env.ts`, and the feature silently behaves as if unconfigured.
- **Normal approach:** the dev server inherits the whole environment.
- **Date:** 2026-06-15

### Form POSTs rely on Astro's origin check instead of a CSRF token

- **What:** no CSRF token; Astro rejects a POST without a matching `Origin` header with `403 Cross-site POST form submissions are forbidden` before the route runs, on top of the `SameSite=Lax` session cookie.
- **Where:** `pages/account/`, `pages/auth/`, `apps/web/astro.config.mjs` (`security.allowedDomains`)
- **Why:** the framework check and `SameSite=Lax` already cover cross-site forms. The check compares `Origin` with the request URL. TLS ends before nginx, so Astro sees `https://qafiyah.com` only because `security.allowedDomains` lets it trust nginx's `X-Forwarded-Proto: https`. Without that, every production POST is a 403. To test these routes with curl, add an explicit `-H 'Origin: ...'`.
- **Normal approach:** a per-form CSRF token.
- **Date:** 2026-09-21

### A 400 from the API renders as a 404

- **What:** `isNotFoundStatus` treats the API's 400 (a page out of range, a bad slug) the same as its 404. So every `getX` or `getXPage` fetcher returns `null` for either one, and the page rewrites to `/404`.
- **Where:** `lib/server/api-error.ts`
- **Why:** to a reader, an out-of-range page or a bad slug is a page that does not exist.
- **Normal approach:** render a 400 as an error page.
- **Date:** 2026-09-13

### SSR retries transient network errors only

- **What:** `safeCall` backs off and retries only what `is-transient-network-error.ts` recognizes as a connection-level failure: aborts, `ECONNRESET`-style codes, and the browser's "failed to fetch" messages. It does not retry a real 4xx or 5xx from the API.
- **Where:** `lib/server/unwrap.ts`, `lib/observability/is-transient-network-error.ts`
- **Why:** a connection blip is worth a retry; an API error response is a real answer.
- **Normal approach:** the HTTP client's own retry option, or a retry library.
- **Date:** 2026-06-27

### The nav account control renders before hydration from client-side hints

- **What:** `viewer-init.astro` reads the `qaf_viewer=1` hint cookie (not HttpOnly) and the cached viewer in localStorage (`qafiyah-viewer`, which holds a 48px data-URL avatar). Then it sets `data-viewer` and `--viewer-avatar` on `<html>`, before the first paint. The server-rendered markup has the link for a known viewer and a skeleton, and CSS picks one.
- **Where:** `components/layout/viewer-init.astro`, `components/layout/account-control.tsx`
- **Why:** pages are cached without cookies, so the server render cannot know the viewer. With this, the control never flashes or moves the navigation.
  - The hint cookie is set at sign-in, cleared at sign-out, and synced again by `account-control.tsx` from `/api/me`. Keep those states rendering the same markup on the server and on hydration.
  - Anonymous visitors see no account control. The slot and its navigation `<li>` are hidden unless `data-viewer` is set, and `account-control.tsx` keeps that attribute in step with `/api/me`.
  - Accounts serve only API keys for now. So the only way in is `/developers`, linked from the footer and the about page.
- **Normal approach:** render the control on the client after fetching the session.
- **Date:** 2026-09-23

### A custom layout debugger (dev only)

- **What:** `layout-debug.astro` outlines and tints every wrapper with a unique color and a `D<n>` label on an overlay layer that never touches the page's own elements.
- **Where:** `components/layout/layout-debug.astro`
- **Why:** a user can point at a box by label. Toggle with Alt+Shift+D or `?debug=layout` (`?debug=off` to clear); it persists in localStorage. In the browser, `window.__qafDebug.el('D12')` returns the labelled element and `.describe('D12')` its tag and classes.
- **Normal approach:** browser devtools.
- **Date:** 2026-09-23

### `theme-init.astro` duplicates `parseSettings`

- **What:** the inline theme-read logic is a hand-kept-in-sync duplicate of `settings-schema.ts::parseSettings`; change one, mirror the other.
- **Where:** `components/layout/theme-init.astro`, `lib/settings/settings-schema.ts`
- **Why:** it runs `is:inline`, before hydration, so it can't import TS modules.
- **Normal approach:** import the shared function.
- **Date:** 2026-06-17

### `module-reload-recovery.astro` reloads on a failed dynamic import

- **What:** an `is:inline` script force-reloads the page once (rate-limited) when a dynamic import fails.
- **Where:** `components/layout/module-reload-recovery.astro`
- **Why:** a visitor's cached HTML can point at a JS chunk hash a newer deploy already evicted from nginx. It runs before hydration, so it can't import TS modules.
- **Normal approach:** keep old chunks available across deploys.
- **Date:** 2026-06-27

### HTML caching is asymmetric

- **What:** `htmlCacheControl` keeps the browser's cache short and nginx's edge cache long.
- **Where:** `lib/server/cache.ts`, `apps/web/astro.config.mjs`
- **Why:** a deploy can't purge browser caches but wipes nginx's and purges Cloudflare's. `build.inlineStylesheets: 'always'` means CSS ships inside that same HTML, so this cache also gates how fast style changes reach visitors.
- **Normal approach:** one `max-age` for browser and proxy.
- **Date:** 2026-09-15

### React islands wrap in `IslandErrorBoundary`

- **What:** every React island renders inside an error boundary with a fallback.
- **Where:** `components/island-error-boundary.tsx`
- **Why:** a client-side crash degrades to the fallback (and reports to Sentry) instead of taking the rest of the otherwise-static page down with it.
- **Normal approach:** one boundary at the app root.
- **Date:** 2026-06-20

### `vite.optimizeDeps.include` pre-bundles the search island's deps

- **What:** `@tanstack/react-query` and `nuqs` are listed in `vite.optimizeDeps.include`.
- **Where:** `apps/web/astro.config.mjs`
- **Why:** they are only discovered lazily. Without pre-bundling them, landing on a page without search and then navigating to one with it triggers a mid-session Vite re-optimize that silently breaks hydration. This entry is the only record of that; there is no comment in the config.
- **Normal approach:** let Vite discover dependencies.
- **Date:** 2026-05-09

### Taxonomy selects keep the API's order

- **What:** `Select` is given `sortOptions={false}` everywhere taxonomy options render.
- **Where:** `components/ui-extended/select.tsx`
- **Why:** each list's order (e.g. eras' chronological order) comes from the API's own `ORDER BY`, not alphabetical.
- **Normal approach:** the component's default sort.
- **Date:** 2026-06-18

### The bundled Amiri fonts are patched to draw `٬` as the ASCII comma

- **What:** both Amiri `.woff2` files map U+066C (the Arabic thousands separator) to the glyph of the ASCII comma `,` (U+002C). They do not use Amiri's own `٬` glyph, or the Arabic comma `،` (U+060C).
- **Where:** `apps/web/src/assets/fonts/Amiri-Regular-400.woff2`, `apps/web/src/assets/fonts/Amiri-Bold-700.woff2`
- **Why:** `Intl.NumberFormat('ar-SA')` groups with U+066C. Amiri draws it as a small raised mark that looks like an apostrophe between digits.
  - Amiri has no alternate glyph for it that CSS could select.
  - Swapping the character in code would put a Latin comma into the text that copy and paste and screen readers see.
  - Amiri is OFL 1.1 with no Reserved Font Name, so the patched files keep the name.
  - Replacing or downloading the fonts again drops the patch. Apply it again with fontTools: for each file, `f = TTFont(path)`. In every `f['cmap'].tables` entry that has U+066C, set `table.cmap[0x066C] = table.cmap[0x2C]`. Then `f.save(path)`.
- **Normal approach:** ship the font files unmodified.
- **Date:** 2026-09-24

### The poet page's poem list updates in place

- **What:** the `PoetPoems` island takes over the server-rendered list on a poet page.
  - When the filters or the page change, it fetches the poems and facets again through the `/api/v1` proxy.
  - It keeps them in the URL with nuqs (`history: 'push'`).
  - When a fetch fails, it falls back to a full page load of the same URL.
  - It sends a PostHog `$pageview` itself on each change, and on back or forward.
- **Where:** `components/poet-poems.tsx`, `components/poet-poem-filters.tsx`, `components/use-list-navigation.ts`, `lib/list-query-client.ts`, `lib/poet-search-params.ts`, `lib/api/proxy-allowlist.ts`, `pages/poets/[slug].astro`
- **Why:** a change of meter, rhyme, or theme keeps the reader's scroll position and focus, which a full page load would reset.
  - It needs the proxy to forward `poems` and `poems/facets`. The proxy does that only for a request that names exactly one poet. So the browser reaches one poet's poems at the visitor rate limit, and never the whole poem list.
  - Pageviews are sent by hand, because PostHog's `defaults: '2026-05-30'` turns on `capture_pageview: 'history_change'`. That compares only the path, so a change of only the query is not captured.
  - The object form `{ path: true, search: true }` (posthog-js, 2026-08-24) would capture it, but on the whole site. Then the URL updates of the home search would also count as pageviews.
- **Normal approach:** links and a form that load the filtered page from the server, as every other list page does.
- **Date:** 2026-10-01

### Sentry's trace tags are stripped from cached pages

- **What:** a middleware removes the `sentry-trace` and `baggage` meta tags from every HTML response with a `public` Cache-Control. Sentry's request handler is added by hand (`autoInstrumentation.requestHandler: false`), so the strip runs after it.
- **Where:** `middleware.ts`, `lib/observability/strip-cached-trace-tags.ts`, `astro.config.mjs`
- **Why:** nginx and Cloudflare cache pages for up to a day, and each cached copy carries the tags of the render that filled it. So every visitor of a copy continued one old trace, and the browser followed that render's sampling decision (#245). Sentry documents the fix: strip the tags before the response is cached. `@sentry/astro` has no option to leave them out, so we strip them. The browser then starts its own trace and samples 10% of page loads.
- **Normal approach:** let the integration inject the tags, on a site that does not cache its HTML.
- **Date:** 2026-10-08

## Search indexer (`apps/search-indexer`)

### A Compose init job that reindexes only empty aliases

- **What:** it runs with `restart: "no"`, `api` waits on `service_completed_successfully`, and it reindexes only when an alias is empty.
- **Where:** `docker-compose.yml`, `docker-compose.dev.yml`
- **Why:** the API starts only once search is populated, and an ordinary restart does not rebuild the indices. A data-only change (a new dump restored onto a live volume) therefore needs the force flag: `bun run reindex` (dev) or `bun run reindex:prod`.
- **Normal approach:** a long-running indexer service.
- **Date:** 2026-06-17

### A failed populate cleans up after itself

- **What:** a failed populate deletes the half-built index and leaves the live alias untouched; the previous versioned indices are deleted only after the swap succeeds.
- **Where:** `apps/search-indexer/src/`
- **Why:** there is nothing to roll back by hand.
- **Normal approach:** the standard Elasticsearch alias swap, listed here for its cleanup guarantees.
- **Date:** 2026-06-26

### It needs the Elasticsearch superuser URL

- **What:** the indexer connects as the Elasticsearch superuser.
- **Where:** `apps/search-indexer/AGENTS.md` ("Environment")
- **Why:** it provisions the reader role that `api` uses; `api` never does.
- **Normal approach:** a least-privilege service account.
- **Date:** 2026-09-23

## Inspector (`apps/inspector`)

### Reads web's pages with `node:fs`

- **What:** reads `apps/web/src/pages/` from disk, not through an `import`.
- **Where:** `apps/inspector/src/`
- **Why:** an import would trip `scripts/check/boundaries.ts`'s cross-app import rule.
- **Normal approach:** import the route list.
- **Date:** 2026-09-17

### Dynamic shape regexes exclude sibling static routes

- **What:** a dynamic shape's regex excludes any sibling static route at the same path depth (e.g. `poems/[slug]` excludes `poems/random`, which is really `poems/random.ts`, a redirect endpoint, not a poem).
- **Where:** `apps/inspector/src/`
- **Why:** without it the crawler can mistake a same-depth static route for an instance of the dynamic one.
- **Normal approach:** match a dynamic segment with a plain wildcard.
- **Date:** 2026-09-17

### The crawl budget counts only real fetches

- **What:** the fetch budget counts only `fetchHtml` calls, not every path popped off the internal queue.
- **Where:** `apps/inspector/src/`
- **Why:** a page can link many redundant candidates for a shape that is already resolved, for example an index page that lists dozens of items. Those must not use up the budget before the crawl reaches a candidate for a different shape that is still unresolved.
- **Normal approach:** cap the number of URLs visited.
- **Date:** 2026-09-17

## Corpus database (`scripts/db`)

### Restores apply schema SQL instead of migrations

- **What:** `scripts/db/init.sh` runs idempotent SQL files on every restore, beside the existing `refresh-poem-relations.sql` and `refresh-taxonomy-stats.sql`:
  - `poem-aliases.sql`, `merge-poem.sql`, `hidden-poets.sql`, `random-poem.sql`, `poem-recensions.sql`, `poet-aliases.sql`, `merge-poet.sql`, and `poem-tashkeel.sql`.
  - If they are missing, these create the `poem_aliases` and `poet_aliases` tables, the `poets.is_hidden`, `poems.is_hidden`, `poems.recension_of_id`, `poems.source`, and `poems.has_tashkeel` columns, their constraints, and the partial indexes on primaries only.
  - `poem-tashkeel.sql` fills `has_tashkeel` once, when it adds the column. A dump that has the column keeps its values, because the fill takes about two minutes.
  - They drop the indexes that those replace, including a primaries-only index that does not have the `is_hidden` predicate.
  - They create the `random_poem_pool` table, and fill it again after the restore.
  - They replace `random_poem_json()` and the maintenance functions.
- **Where:** `scripts/db/init.sh`, `scripts/db/sql/`
- **Why:** the corpus database ships as whole dumps, and has no migrations (`apps/api/AGENTS.md`). So a restore is the one moment when a schema change can meet an existing dump. Applying the files there lets current code run against an older dump. It also keeps the functions reviewable as files, not only inside dumps. A new dump already has the same schema, so on it every statement does nothing.
- **Normal approach:** versioned migrations run on deploy, the way `apps/api/migrations/` manages the accounts database.
- **Date:** 2026-09-26

### The excerpt merge crossed poets

- **What:** the excerpt merge of issue #277 merged short poems into the longer poems that hold their text, also when the two had different poets.
  - A named poet won over an anonymous record. Then the poet of the older era won, then the poet with more poems. On a tie, both poems stayed.
  - The longer text always survived. When the winning poet owned only the shorter poem, the longer poem moved to that poet only if the shorter one held 80% of it or more. Otherwise both poems stayed, because a short quotation does not make a long poem someone else's.
  - A poem in a collection never changed poet.
- **Where:** dump 0043; two SQL scripts that called `reattribute_poem` and then `merge_poem`, kept out of the repo
- **Why:** the maintainer decided that one text gets one page, even when the sources credit it to two poets.
- **Normal approach:** `merge_poem` refuses two poets, because that is a question of attribution. Both pages stay, and link to each other.
- **Date:** 2026-10-09

### The `nabati` poem type names a language, not a form

- **What:** the poem type `nabati` holds every poem whose register is `nabati`, whatever its form. The web lays out its two-part entries like `amudi`.
- **Where:** the `poem_types` row `nabati` in the dumps from 0044, `CLASSICAL_LAYOUT_POEM_TYPES` in `apps/web/src/lib/constants/taxonomy-data.ts`, and `docs/domain.md` ("Poem type")
- **Why:** the maintainer wants readers to filter Nabati poetry, and the poem type is the filter that the API, search, and site already have.
- **Normal approach:** keep the type as the form (`amudi`, `hurr`), and expose the existing `register_id` as its own filter in the API, the search index, and the site.
- **Date:** 2026-10-10

## Static checks (`docs/development.md`)

### Two formatters

- **What:** oxfmt formats everything it supports (TypeScript, JSON, CSS, Markdown, YAML, TOML), and `format` runs prettier with `prettier-plugin-astro` on `.astro` files.
- **Where:** `package.json` (`format`, `format:check`), `.oxfmtrc.json`, `.prettierrc.json`
- **Why:** oxfmt does not support `.astro`. Both formatters sort Tailwind classes against `apps/web/src/styles/globals.css`, so `.tsx` and `.astro` agree on order and the project's own utilities sort by their real layer.
- **Normal approach:** one formatter for everything.
- **Date:** 2026-09-12

### `prettier-plugin-astro` is pinned to 0.14.1

- **What:** the plugin stays on 0.14.1, and Dependabot ignores 1.x.
- **Where:** `package.json`, `.github/dependabot.yml`
- **Why:** 1.x silently stops `prettier-plugin-tailwindcss` from sorting classes in `.astro` files.
- **Normal approach:** track the latest version.
- **Date:** 2026-09-24

### TypeScript stays on 6.x

- **What:** `typescript` stays on 6.0.3 in `package.json` and `apps/web/package.json`, and Dependabot ignores 7.x.
- **Where:** `package.json`, `apps/web/package.json`, `.github/dependabot.yml`
- **Why:** TypeScript 7 is the native compiler and no longer ships the JavaScript API that `astro check` (`@astrojs/language-server` 2.16.10) calls, so `bun run types` crashes with `Cannot read properties of undefined (reading 'fileExists')`. Lift the ignore once Astro's checker supports 7.
- **Normal approach:** track the latest version.
- **Date:** 2026-09-25

### `prettier-plugin-astro` can add whitespace inside a nested element

- **What:** when it wraps a long line, an element nested in a `{...}` expression can gain whitespace. For example, `<span>text</span>` becomes the tag, the text, and the closing tag, on three lines.
- **Where:** `.astro` files under `apps/web/src/`
- **Why:** inside a flex or grid container, that whitespace is dropped, and this covers every case in the app today. In inline text, it renders as a space. Code that reads the text of an element reads it trimmed (the copy button for API keys does).
- **Normal approach:** formatter output that never changes rendering.
- **Date:** 2026-09-24

### The SQL syntax check substitutes psql variables

- **What:** `check:sql` replaces psql variables (`:name`) before parsing, the way psql does.
- **Where:** `scripts/check/sql.ts`
- **Why:** psql variables are client-side, so the PostgreSQL parser would reject them as-is.
- **Normal approach:** parse the files unchanged.
- **Date:** 2026-09-24

### hadolint ignores DL3018

- **What:** `apk add` versions are not pinned.
- **Where:** `.hadolint.yaml`
- **Why:** Alpine removes superseded package versions from its repositories, so a pinned `apk add` stops building; the image tag is the pin.
- **Normal approach:** pin every `apk` package version.
- **Date:** 2026-09-24

### `rust:lint` also fails on cargo's own warnings

- **What:** `rust:lint` fails on cargo's own warnings (such as an unused `Cargo.toml` key), and `check:rust-toolchain` rejects keys rustup would ignore in `rust-toolchain.toml`.
- **Where:** `scripts/check/rust-lint.ts`, `scripts/check/rust-toolchain.ts`
- **Why:** `-D warnings` does not reach cargo's warnings, and rustup ignores unknown keys silently. `clippy.toml` needs neither, since clippy errors on an unknown key.
- **Normal approach:** `cargo clippy -- -D warnings` alone.
- **Date:** 2026-09-24
