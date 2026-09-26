# Exceptions

Approved departures from the standard approach (see "Keep it boring" in the root `AGENTS.md`), grouped by component. Anything not listed here should be standard. Component `AGENTS.md` files describe shape; the non-obvious "why" lives here. Dates are when the behavior first appeared in the repo.

Each entry:

```md
### <short title>

- **What:** the unusual thing, in one line
- **Where:** file paths
- **Why:** the reason the standard approach did not work
- **Normal approach:** what we would usually do instead
- **Date:** YYYY-MM-DD
```

## Needs review

Departures not yet approved, found by a full scan on 2026-09-24 and ordered from most to least concerning. These entries use **Why it's unusual** and **Status** in place of **Why** and **Date**. Once reviewed, a justified entry moves into its component section below with a **Why** and **Date**, and any other gets a plan to replace it.

### The quality gate is a custom runner

- **What:** the gate is `scripts/ci.ts` (a 334-line orchestrator with phases, a worker pool, and output scrubbing). GitHub Actions runs it on every push and PR, the non-Docker phases in one job and each Docker phase in its own (`--phase db`, `origin`, `stack`).
- **Where:** `scripts/ci.ts`, `scripts/ci/phases.ts`, `.github/workflows/ci.yml`
- **Why it's unusual:** the runner re-does turbo's parallelism and log prefixing, then filters turbo's banners with a hard-coded prefix list.
- **Normal approach:** root tasks become turbo tasks, and the workflow calls them directly as separate jobs.
- **Status:** Needs review

### Full database dumps are committed to git under a homemade key scheme

- **What:** every Postgres dump is committed as openssl-encrypted 45 MB parts (162 parts, about 6.5 GB at HEAD, with `.git` at 9.6 GB), alongside a hand-built key manifest.
- **Where:** `data/README.md`, `data/db/`, `scripts/db/split-dump.sh`, `scripts/db/encrypt-dump.sh`, `scripts/db/resolve-dump.sh`, `scripts/dev/set-dump-key.sh`, `scripts/dev/check-dump-key.sh`
- **Why it's unusual:** encrypted blobs never delta-compress, so every clone and every VPS fetch carries every version forever, and pushes already need the `pack.useSparse=false` workaround. The manifest "hash" is `openssl enc -P` with a salt hard-coded in two scripts. Passphrases go on the command line as `-pass "pass:..."`, where `ps` can read them. The "newest dump" lookup is repeated in six scripts.
- **Normal approach:** dumps published as GitHub Release assets or R2 objects (R2 already holds avatars and backups) and downloaded by `db:up`. If encryption stays, use `age`, or at least `-pass env:VAR`.
- **Status:** Needs review

### Deploys build images on the production VPS over piped SSH

- **What:** `bun run deploy` concatenates shell files and pipes them into `ssh host 'bash -s'`, which runs `git reset --hard`, builds every image on the 4 GB production box, and rolls replicas with a custom `rollout()`.
- **Where:** `scripts/deploy/vps.sh`, `scripts/lib/remote.sh`, `scripts/db/reseed.sh`, `scripts/es/reindex-prod.sh`, `.github/workflows/images.yml`, `.claude/skills/deploy/SKILL.md`, `docs/deployment/architecture.md`
- **Why it's unusual:** release Rust builds (`lto = true`, one codegen unit) and the Astro build compete with the live stack for memory, which is why the docs keep swap. With no registry, rollback means reverting main and rebuilding. The Images workflow builds each image and throws it away. `rollout()` reimplements the docker-rollout plugin with fixed timings (a 150-second health deadline and a 7-second settle).
- **Normal approach:** CI builds on merge and pushes SHA-tagged images to GHCR, the VPS runs `docker compose pull` and `docker rollout` (or Kamal), and rollback redeploys the previous tag.
- **Status:** Needs review

### Dependabot never updates Docker images

- **What:** `.github/dependabot.yml` has `bun`, `cargo`, and `github-actions` entries but no `docker` or `docker-compose` entry.
- **Where:** `.github/dependabot.yml`
- **Why it's unusual:** base images (`rust`, `alpine`, `oven/bun`, `postgres`, and the exact Elasticsearch and CRS tags) never get update PRs.
- **Normal approach:** `docker` and `docker-compose` entries next to the existing ones.
- **Status:** Needs review

### Dev stacks are documented as running on the production VPS

- **What:** the dev override renames containers, ports, volumes, and subnets so a dev clone can run beside production on the same 4 GB host.
- **Where:** `docker-compose.dev.yml`, `docs/deployment/architecture.md` ("Prod vs dev isolation (both on one host)"), `docs/topology.md`
- **Why it's unusual:** production limits already add up to about 3 GB before a rollout doubles `api` and `web`, and the dev override alone gives Elasticsearch a 1500m heap and `mem_limit: 3g`, so a dev run can push production into OOM.
- **Normal approach:** develop on laptops or a separate staging VM, and keep the production host for production.
- **Status:** Needs review

### The web container runs two processes and is also the API's public ingress

- **What:** the web image runs nginx and Bun side by side under a shell loop that polls every 5 seconds. That nginx also carries the `api.qafiyah.com` server block (routing, the `/account` 404, and the API CSP).
- **Where:** `apps/web/Dockerfile`, `apps/web/docker-entrypoint.sh`, `apps/web/nginx.conf`, `apps/web/nginx-csp-api.conf`, `docker-compose.yml` (`web` networks and `depends_on`), `docs/deployment/architecture.md`, `docs/topology.md`
- **Why it's unusual:** requests pass through Cloudflare, the edge-gateway nginx, the web nginx, then Bun or the API. Changing the API's edge behavior means rebuilding the web image, and a crash of the Astro process takes `api.qafiyah.com` down too. The `while kill -0 ...; sleep 5` loop is a homemade supervisor. `web` also joins the `default` network and waits on `db`, although it never talks to Postgres and `docs/topology.md` says `default` carries only `api`, `db`, `es`, and the indexer.
- **Normal approach:** one process per container. Route each hostname to its own service at the existing edge-gateway (or with cloudflared ingress rules), and put `web` only on `edge` and `backend`.
- **Status:** Needs review

### A self-hosted WAF that has only ever logged

- **What:** edge-gateway runs ModSecurity with OWASP CRS behind Cloudflare, and it has been in `DetectionOnly` since 2026-06-17.
- **Where:** `docker-compose.yml` (`edge-gateway`, `MODSEC_RULE_ENGINE`), `apps/edge-gateway/`, `docs/deployment/services.md`, `.claude/skills/deploy/SKILL.md`
- **Why it's unusual:** it is an extra nginx hop that blocks nothing. `services.md` records enforcement as validated safe on 2026-06-18, yet it stays off. Each image bump needs the vendor template re-copied and re-patched by hand. The documented WAF rollback puts a host port on `web`, which conflicts with `rollout()` scaling `web` to two replicas and moves web's peer outside the trusted subnet. Its real-IP setting reads `CF-Connecting-IP`, which the Cloudflare tunnel never sends, so it scores and logs every request as the Docker gateway address.
- **Normal approach:** Cloudflare's managed WAF rules and rate limiting, which are already in the path, with one reverse proxy at the origin.
- **Status:** Needs review

### OAuth is a hand-written client with a redundant signed state

- **What:** the authorize URLs, token exchange, and userinfo calls are hand-written. The state is HMAC-signed with `SESSION_STATE_SECRET` on top of the cookie comparison, signatures are compared with a homemade constant-time loop, and there is no PKCE.
- **Where:** `apps/web/src/lib/server/oauth/` (`state.ts`, `providers.ts`, `exchange.ts`), `apps/web/src/pages/auth/[provider].ts`, `apps/web/src/pages/auth/callback/[provider].ts`, `apps/web/docker-entrypoint.sh`
- **Why it's unusual:** comparing a random state in an HttpOnly cookie with the query parameter is the whole standard CSRF defense. The signature binds nothing (no expiry, provider, or session), and anyone can get a validly signed state by visiting `/auth/google`, so the secret adds a required variable and no security. The name suggests sessions, but the secret only signs OAuth state. The approved "Web owns identity" entry covers where OAuth lives, not writing the client by hand.
- **Normal approach:** Arctic (`Google`, `GitHub`, `generateState`, `generateCodeVerifier`, `validateAuthorizationCode`) with the state and PKCE verifier in short-lived HttpOnly cookies, keeping the existing `email_verified` checks.
- **Status:** Needs review

### Cookies, redirects, and session lookup bypass Astro's built-ins

- **What:** the app uses no `Astro.cookies`, no `redirect()`, and no middleware or `Astro.locals`. Instead there are three copy-pasted cookie parsers, hand-built `Set-Cookie` strings in seven files, and ten hand-built 302 responses.
- **Where:** `apps/web/src/lib/server/session.ts`, `apps/web/src/lib/server/new-key-cookie.ts`, `apps/web/src/lib/server/oauth/redirect.ts`, `apps/web/src/lib/viewer-hint.ts`, `apps/web/src/pages/auth/`, `apps/web/src/pages/account/`, `apps/web/src/pages/login.astro`, `apps/web/src/pages/api/me.ts`, `apps/web/src/pages/poems/random.ts`, `apps/web/src/test/context.ts`
- **Why it's unusual:** Astro ships a cookie API and `context.redirect()`, and middleware is the usual place to resolve the session into `locals` and stamp `no-store` on auth prefixes. Encoding is inconsistent (the new-key cookie is URI-encoded, the others are not), `resolveViewer(request.headers.get('cookie'))` is repeated per route, and the test helper builds a `cookies` mock that production never uses.
- **Normal approach:** `context.cookies.get/set/delete`, `return context.redirect('/login')`, and a `src/middleware.ts` that sets `locals.viewer` and the `no-store` header for `/account`, `/auth`, and `/login`.
- **Status:** Needs review

### Web environment config is split four ways instead of `astro:env`

- **What:** `envin` validates one optional public variable, server variables are raw `process.env` reads that fall back to `''`, required-variable checks live in the shell entrypoint, and `astro.config.mjs` re-declares `PROD_SITE_URL` and `DEV_WEB_PORT`.
- **Where:** `apps/web/src/env.ts`, `apps/web/src/lib/server/env.ts`, `apps/web/docker-entrypoint.sh`, `apps/web/astro.config.mjs`, `apps/web/src/lib/constants/config.ts`
- **Why it's unusual:** Astro has a typed env schema (`envField`, `context: 'server'`, `access: 'secret'`) that fails at startup. The `''` fallbacks are why the approved `turbo.json` entry says a missing variable "silently behaves as if unconfigured", and a missing `INTERNAL_API_URL` in production silently falls back to localhost. Reading env at import time forces route tests to mutate `process.env`, reset modules, and import routes dynamically.
- **Normal approach:** declare every variable in `env.schema` in `astro.config.mjs` and import from `astro:env/server` or `astro:env/client`.
- **Status:** Needs review

### The search filters use a hand-built multi-select combobox

- **What:** `Select` is a 283-line homemade ARIA combobox, and its keyboard handling is a `window` `keydown` listener that stays active whenever the list is open.
- **Where:** `apps/web/src/components/ui-extended/select.tsx`, `apps/web/src/components/search/filters.tsx`
- **Why it's unusual:** Enter or Space toggles an option and also closes the list. After Tab moves focus away the list stays open, so the next Enter or Space anywhere on the page toggles an option. All five filters share `aria-label="اختيار متعدد"`, and their visible labels are `<p>` elements not tied to the control. Option ids (`option-${index}`) repeat across instances, and a clear `<button>` is nested inside the `role="combobox"` element. The single-select, `clearValue`, and `disabled` modes are never used.
- **Normal approach:** a maintained primitive (shadcn Popover with cmdk, Headless UI `Listbox multiple`, or React Aria), or a checkbox group in a `<fieldset>` with a `<legend>`.
- **Status:** Needs review

### The API's logging, request ids, and error reporting are hand-rolled

- **What:** request logs are `println!` of a `serde_json::Map` gathered through an `Arc<Mutex<Map>>` extension, request ids are a hand-formatted UUID v4, and errors go to `eprintln!` and manual Sentry capture.
- **Where:** `apps/api/src/log.rs`, `apps/api/src/error.rs`, `apps/api/src/sentry.rs`, `apps/api/src/accounts/usage.rs`, `apps/api/src/main.rs`, and every handler that takes `Extension<LogHandle>`
- **Why it's unusual:** the approved entries cover only the sampling rule and `civil_from_days`, and the rest is a homemade structured-logging framework. The request id is generated after the response exists and is never returned in a header, sent to Sentry, or attached to stage events, so it correlates nothing. Eleven `clippy::print_stdout`/`print_stderr` expectations exist only to allow it.
- **Normal approach:** `tracing` with `tracing-subscriber` JSON output, `tower_http::trace::TraceLayer`, `tower_http::request_id`, and `sentry-tower` or `sentry-tracing`. The sampling rule can stay as a custom layer.
- **Status:** Needs review

### `bun run dev` is a custom process supervisor that reads its children's output

- **What:** `scripts/dev/run.ts` (560 lines) starts cargo, turbo, astro, and the inspector. It decides readiness and status by regex-matching their stdout, the API's JSON log fields, and `resolve-dump.sh`'s stderr.
- **Where:** `scripts/dev/run.ts`, `scripts/dev/clean.sh`, `scripts/smoke/run.ts`, `apps/web/package.json` (`dev`)
- **Why it's unusual:** startup depends on the exact wording of other programs' output (Astro's "Local" banner, turbo's banners, shell log lines, and field names in `apps/api/src/log.rs`), so a reworded line silently breaks `dev`, the smoke phase, and the gate. Leftovers are killed with `pkill -f "${ROOT}.*astro"`. By code reading, a local `bun run ci` smoke run ends by stopping the shared `qafiyah-dev` project, which also takes down a dev stack the developer started themselves.
- **Normal approach:** a `dev` script in `apps/api/package.json` (`cargo watch -x run`) supervised by `turbo run dev`, or `docker compose watch`, with readiness checked through `/healthz`.
- **Status:** Needs review

### Worktree isolation is layered over Compose and applied inconsistently

- **What:** a `--worktree` flag hashes the worktree name into port offsets and a Compose project suffix across about eight scripts. `compose.sh` turns it on automatically in a linked worktree, while `run.ts`, `db-test.ts`, the smoke scripts, and `conformance.ts` honor only the explicit flag.
- **Where:** `scripts/dev/worktree.ts`, `scripts/dev/compose.sh`, `scripts/dev/run.ts`, `scripts/ci.ts`, `scripts/smoke/surfaces.ts`, `scripts/api/conformance.ts`, `scripts/dev/db-test.ts`, `docker-compose.dev.yml`, `docs/development.md`
- **Why it's unusual:** Compose already isolates per project. The repo overrides that with a fixed `COMPOSE_PROJECT_NAME`, `container_name`, and volume names, then rebuilds isolation on top. In a worktree without the flag, `compose.sh` starts db and Elasticsearch on offset ports while `run.ts` points the API at offset 0, the primary checkout's containers. `docs/development.md` says the flag is never implied, which `compose.sh` contradicts, and the fixed dev subnets stop two worktrees from running full stacks at once anyway.
- **Normal approach:** let Compose scope by project (no `container_name` or fixed volume names in dev), and keep each checkout's ports in its own `.env` or read them with `docker compose port`.
- **Status:** Needs review

### Smoke and contract tests run on a bespoke HTTP test framework

- **What:** `scripts/smoke/` (about 2,600 lines) is its own test runner, with a probe DSL, suites, surfaces, verdicts, a concurrency pool, retries, latency budgets, and a reporter. `scripts/api/conformance.ts` hand-rolls OpenAPI response validation with ajv.
- **Where:** `scripts/smoke/`, `scripts/api/conformance.ts`
- **Why it's unusual:** it gives up filtering, watch mode, standard reporters, and `test.each`, and the reinvented pieces drift. The "p95" over five samples is the maximum, `target.ts` can `process.exit` at import time, and `surfaces.ts` takes the first non-`--worktree` argument as the target, so by code reading `bun run smoke:dev --suite search` fails as an unknown target.
- **Normal approach:** `bun test` with `describe.each`/`test.each` over the probe tables (or Hurl for black-box HTTP), and Schemathesis for OpenAPI conformance.
- **Status:** Needs review

### Shared constants are hand-copied into Rust and infra files and synced by regex

- **What:** 29 `config.ts` values are copied into `apps/api/src/constants.rs`, and more are pinned as literals in nginx, Astro, Compose, the Elasticsearch schema, and `vps.sh`. `scripts/check/constants.ts` parses them all with regexes to catch drift.
- **Where:** `config.ts`, `apps/api/src/constants.rs`, `scripts/check/constants.ts`, root `AGENTS.md`
- **Why it's unusual:** there are two sources of truth plus a third file that encodes the mapping between them, down to Rust syntax. At least eight `config.ts` values (`X_PROFILE_URL`, `TELEGRAM_URL`, `GITHUB_URL`, `GITHUB_DB_DUMPS_URL`, `GITHUB_AVATARS_URL`, `RAAQIM_URL`, `X_INTENT_TWEET_URL`, `MAX_TWEET_LENGTH`) have no TypeScript consumer except the checker, which also hides them from knip.
- **Normal approach:** the API publishes its contract limits through the OpenAPI document, which already generates the TypeScript types. Anything else shared lives in one JSON or TOML file that Rust reads with `include_str!` and TypeScript imports.
- **Status:** Needs review

### API configuration is read three ways, and trusted proxy subnets are compiled in

- **What:** `Config::from_vars` is injected and tested, `sentry::Config::from_env` reads `std::env` directly, and `log::environment()` re-reads `ENVIRONMENT` into a `OnceLock` global. The trusted proxy networks are a compile-time constant parsed by a hand-written CIDR parser.
- **Where:** `apps/api/src/config.rs`, `apps/api/src/sentry.rs`, `apps/api/src/log.rs`, `apps/api/src/constants.rs::TRUSTED_PROXY_NETWORKS`, `apps/api/src/client_ip.rs`, `docker-compose.yml`, `docker-compose.dev.yml`, `apps/web/nginx.conf`, `scripts/dev/worktree.ts`
- **Why it's unusual:** it breaks the repo's own "no globals/singletons" rule. The `172.26` to `172.29` subnets appear in four places with no sync check, and production images also trust the dev subnets. If a Compose subnet changes, nginx becomes an untrusted peer and every public caller shares one anonymous bucket, with no error anywhere. The approved `client_ip` entry covers the trust policy, not compiling the ranges in.
- **Normal approach:** one `Config` passed into `AppState` and the layers, with the trusted proxies from an env var (for example `TRUSTED_PROXIES`) parsed by the `ipnet` crate.
- **Status:** Needs review

### The API parses query strings with a custom qs-style parser

- **What:** list and search handlers take `RawQuery` and parse it with a homemade `Query` type that understands PHP/qs bracket syntax (`era[]=`, `era[0]=`, and `era[x]`, which it rejects as an object). The OpenAPI params are separate hand-written tuples.
- **Where:** `apps/api/src/query.rs`, `apps/api/src/routes/poems.rs`, `apps/api/src/routes/search.rs`, `apps/api/src/routes/poets.rs`, `apps/api/src/routes/go.rs`
- **Why it's unusual:** no first-party client sends bracket forms: web appends repeated keys, and the contract documents only `?era=a&era=b`. Validation rules (the page pattern, `max_length = 50`, slug patterns, 100-item caps) are written once as doc strings and again in code, with nothing tying them together. There are also two scalar semantics: `first()` silently takes the first value, while `scalar()` answers 400 on repeats.
- **Normal approach:** `axum_extra::extract::Query<T>` (serde_html_form) on a `#[derive(Deserialize, IntoParams)]` struct per endpoint, so parsing and docs come from one type.
- **Status:** Needs review

### Elasticsearch responses are read as untyped JSON with silent defaults

- **What:** search hits are mapped by indexing into `serde_json::Value`, so a missing or renamed field becomes `""`, `0`, or `false`.
- **Where:** `apps/api/src/domain/search.rs`, `apps/api/src/es/client.rs`
- **Why it's unusual:** it contradicts the repo's own "validate at entry, fail loudly at boundaries" rule. A mapping drift would ship empty slugs and names with a 200, and a test pins the silent defaults. `total_hits` also accepts the pre-ES7 numeric `hits.total` shape, which this stack never returns.
- **Normal approach:** `#[derive(Deserialize)]` structs for the response and each hit's `_source`, with required fields non-optional.
- **Status:** Needs review

### JSON-LD builders validate their own output at runtime and throw

- **What:** each schema.org builder builds an object from typed data, runs it through a hand-written valibot schema, and throws if validation fails.
- **Where:** `apps/web/src/lib/seo/json-ld/` (`document.ts::parseNode` and every builder), `apps/web/src/lib/seo/serialize-json-ld.ts`
- **Why it's unusual:** this is the app's own output, not untrusted input, and it is typed twice (a valibot schema and a hand-written node type per builder). Any empty field becomes a 500 for the whole page, for example one poem with an empty title in a taxonomy list, or a title that `sanitizeMetaText` reduces to `''`.
- **Normal approach:** plain builder functions typed with `schema-dts`, a unit test per shape, and no runtime schema.
- **Status:** Needs review

### The random poem has its own client and retry loop, plus a client-side copy of the redirect

- **What:** `/poems/random` uses raw `fetch`, a neverthrow `Result`, and a second backoff loop, and 302s to `/500` on failure. `RandomPoemButton` then intercepts its `<a href="/poems/random">` to repeat the same lookup in the browser through the `/api/v1` proxy.
- **Where:** `apps/web/src/lib/api/random-poem.ts`, `apps/web/src/pages/poems/random.ts`, `apps/web/src/components/random-poem-button.tsx`, `apps/web/src/lib/api/proxy-allowlist.ts`, `apps/web/src/pages/500.astro`
- **Why it's unusual:** it is a third API client with a second retry policy, and unlike `safeCall` it retries every error, 429 included. neverthrow, which `docs/code-conventions.md` prescribes, is used only here, so the web app now has four failure conventions. The redirect to `/500` breaks that page's retry link, which rebuilds from the current URL (now `/500`). `poems/random` is in the proxy allowlist only for the button, and the button needs a `pageshow` handler to undo its own loading state after back navigation.
- **Normal approach:** add `/poems/random` to the OpenAPI contract and call it through `apiServer` with the existing retry, `return context.rewrite('/500')` on failure, and render a plain `<a href="/poems/random">`.
- **Status:** Needs review

### The home search submits on blur and moves focus itself

- **What:** the home search is a bare input with no `<form>`. It submits on Enter through a `keydown` handler, on the icon button, and on blur, and it places the caret and focus by hand.
- **Where:** `apps/web/src/components/search/use-search.tsx`, `apps/web/src/components/ui-extended/search-input.tsx`, `apps/web/src/components/search/search-container.tsx`
- **Why it's unusual:** tapping elsewhere or dismissing the mobile keyboard runs a search, and the clear button needs `onMouseDown={preventDefault}` to avoid it. The first click moves the caret to the end by hand, and focus jumps from the input to a results heading after every search. `tabIndex={-1}` on wrapper elements makes them take focus on click, which blurs the input and fires a search. nuqs's default `replace` means searches add no history entries. The poets page, by contrast, uses a plain `<form method="GET" role="search">`.
- **Normal approach:** `<form role="search" onSubmit>` around `<input type="search" enterKeyHint="search">`, submitting only on Enter or the button, an `aria-live` region to announce results, and `history: 'push'` for the committed query.
- **Status:** Needs review

### Arabic plurals and digits are hand-rolled instead of using `Intl`

- **What:** `formatArabicCount` hard-codes "0, 1, 2, then 3 to 10 plural, otherwise singular", and digits are converted both by a lookup table and by `Intl.NumberFormat('ar-SA')`.
- **Where:** `apps/web/src/lib/arabic.ts` (`formatArabicCount`, `toArabicDigits`), `apps/web/src/lib/pagination.ts`
- **Why it's unusual:** Arabic returns to the plural when the last two digits are 03 to 10, which `Intl.PluralRules('ar')` already encodes as `few`, so counts such as 103 and 105 render with the singular noun. Pagination shows ungrouped digits ("١٢٣٤") while counts show grouped ones ("١٬٣٣٨").
- **Normal approach:** map `new Intl.PluralRules('ar').select(n)` to the noun forms, and format every number with one `Intl.NumberFormat`.
- **Status:** Needs review

### JavaScript runtime semantics spread beyond `js.rs`

- **What:** the ETag is FNV-1a over UTF-16 code units, lengths are counted with `encode_utf16().count()`, `go.rs` hand-writes `encodeURIComponent`, and the log formats instants "the way javascript does".
- **Where:** `apps/api/src/cache.rs`, `apps/api/src/query.rs`, `apps/api/src/domain/poems.rs`, `apps/api/src/domain/search.rs`, `apps/api/src/routes/go.rs`, `apps/api/src/log.rs`
- **Why it's unusual:** the approved `js.rs` entry says not to reach for JavaScript semantics outside that module. No consumer recomputes the ETag, so hashing UTF-16 buys nothing. UTF-16 counting for `q` contradicts the published `maxLength: 50`, because JSON Schema counts code points.
- **Normal approach:** any stable hash over the bytes for the ETag (`sha2` is already a dependency), `chars().count()`, and `percent_encoding` or `url::Url::query_pairs_mut`.
- **Status:** Needs review

### `.env` is read by five different parsers

- **What:** the root `.env` is read by bash `source`, Docker Compose, Bun's auto-load, a grep regex, and a custom dotenv parser, and dev passwords are hard-coded in four places.
- **Where:** `scripts/db/resolve-dump.sh`, `scripts/lib/tag-db-container.sh`, `scripts/db/encrypt-dump.sh`, `scripts/dev/set-dump-key.sh`, `scripts/dev/compose.sh`, `scripts/secrets/dotenv.ts`, `scripts/secrets/schema.ts`, `scripts/dev/run.ts`, `scripts/dev/service-urls.ts`, `scripts/deploy/build-images.ts`, `scripts/smoke/surfaces.ts`
- **Why it's unusual:** the parsers disagree on quotes and `$`. A `DUMP_KEY__*` passphrase with a space or `$` works in Compose but is mangled by `source .env`, and the custom parser rejects `export FOO=` lines that `compose.sh` deliberately accepts. `schema.ts` adds yet another registry of variable names.
- **Normal approach:** one `.env.example` (or `${VAR:-default}` once in `docker-compose.dev.yml`) for dev defaults, and one loader instead of `source .env`.
- **Status:** Needs review

### Custom lint scripts overlap the installed linters

- **What:** `check:boundaries`, `check:no-parent-imports`, and `check:naming` scan sources and file names with regexes, while oxlint and dependency-cruiser already enforce overlapping rules.
- **Where:** `scripts/check/boundaries.ts`, `scripts/check/no-parent-imports.ts`, `scripts/check/naming.ts`, `scripts/lib/imports.ts`, `.oxlintrc.json`, `.dependency-cruiser.cjs`
- **Why it's unusual:** cross-app imports are checked twice, and so are cycles. The regex import extractor misses dynamic `import()` and can match text inside strings and comments. oxlint ships `import/no-relative-parent-imports`, which is unused, and `unicorn/filename-case`, which is explicitly turned off next to a 137-line naming checker.
- **Normal approach:** dependency-cruiser rules for cross-app and parent imports, and `unicorn/filename-case` or ls-lint for naming.
- **Status:** Needs review

### A search schema change is never reindexed by a deploy

- **What:** the indexer skips the rebuild whenever both aliases hold documents, whatever the mapping, and the deploy has no reindex step for `schema.json` changes.
- **Where:** `apps/search-indexer/src/main.rs`, `crates/elasticsearch/schema.json`, `crates/elasticsearch/AGENTS.md`, `scripts/deploy/vps.sh`, `.claude/skills/deploy/SKILL.md`
- **Why it's unusual:** this goes past the approved init-job entry, which covers data-only changes. A deploy ships an API built against the new mapping while the live index keeps the old one until someone remembers `reindex:prod`.
- **Normal approach:** put a schema version or hash in the index name, and reindex when the alias points at a different version.
- **Status:** Needs review

### The whole poem hydrates as a React island for font scale and a private `#h=` fragment

- **What:** `PoemDisplay` renders the title, byline, and every verse as a `client:idle` island. The only client-side needs are the font-scale setting and highlighting from a homemade `#h=term1,term2` fragment.
- **Where:** `apps/web/src/pages/poems/[slug].astro`, `apps/web/src/components/poem-display.tsx`, `apps/web/src/lib/urls.ts`, `apps/web/src/lib/highlight.ts`
- **Why it's unusual:** SSR renders at scale 1, so a saved scale visibly resizes the poem after idle hydration. Theme avoids this with a pre-paint script, and font scale does not. The verses ship twice (as HTML and as serialized props), and the scale is applied twice (an inline `fontSize` on every hemistich and a `--poem-scale` variable).
- **Normal approach:** render the poem statically in Astro, set `--poem-scale` on `<html>` in the existing pre-paint script, and highlight with Text Fragments (`#:~:text=`) or a small script using the CSS Custom Highlight API.
- **Status:** Needs review

### The home page hand-copies the search UI as a placeholder and deletes it by DOM query

- **What:** `index.astro` duplicates the search UI as a static, inert `#search-shell` overlay, and the `client:only` island removes it on mount with `document.querySelector('#search-shell')?.remove()`.
- **Where:** `apps/web/src/pages/index.astro`, `apps/web/src/components/search/search-with-providers.tsx`
- **Why it's unusual:** the shell's markup, classes, and icons (redrawn with `icon.astro` instead of lucide) must be kept in sync with `search-container.tsx` and `search-input.tsx` by hand. It is also unusual for an island to reach outside its own root to delete page DOM.
- **Normal approach:** Astro's `<div slot="fallback">` inside the `client:only` component, which Astro swaps out on load, or server-render the island with `client:load`.
- **Status:** Needs review

### The mobile menu is a hand-rolled drawer

- **What:** the mobile nav is a fixed `div` opened by a script that toggles classes, `inert`, `aria-hidden`, each link's `tabIndex`, and `body.style.overflow`, plus a document-level Escape listener.
- **Where:** `apps/web/src/components/layout/nav.astro`
- **Why it's unusual:** focus never moves into the menu on open or back to the button on close. The manual `tabIndex` and `aria-hidden` repeat what `inert` already does. The settings panel in the same app uses a native `<dialog>`, so there are two overlay implementations.
- **Normal approach:** a `<dialog>` opened with `showModal()`, which provides the focus trap, Escape, an inert background, and focus return. A shadcn Sheet also works.
- **Status:** Needs review

### `issue-key` bypasses the library's key invariants

- **What:** the CLI inlines its own `INSERT INTO users` and `INSERT INTO api_keys` instead of calling `users::upsert` and `keys::create_for`.
- **Where:** `apps/api/src/bin/issue-key.rs`, `apps/api/src/accounts/keys.rs`, `apps/api/src/accounts/users.rs`
- **Why it's unusual:** it skips `MAX_ACTIVE_KEYS_PER_USER` and stores the raw email instead of `normalize_email`. Its `ON CONFLICT (lower(email)) DO UPDATE SET email = EXCLUDED.email` also rewrites an existing user's stored email casing.
- **Normal approach:** the CLI calls the same library functions the HTTP routes use.
- **Status:** Needs review

### Account timestamps are hand-formatted with a literal `Z`

- **What:** timestamps are rendered in SQL with `to_char(.., 'YYYY-MM-DD"T"HH24:MI:SS"Z"')` into `String`, and usage hours are epoch-hour integers computed in Rust.
- **Where:** `apps/api/src/accounts/keys.rs`, `apps/api/src/accounts/usage.rs`, `apps/api/src/rate_limit.rs`, `apps/api/src/main.rs` (connect options)
- **Why it's unusual:** `to_char` on a `timestamptz` follows the session `TimeZone`, so the literal `Z` is correct only while the server default happens to be UTC, and no `TimeZone` is set on connect. The `civil_from_days` exception covers only `log.rs`.
- **Normal approach:** sqlx's `time` or `chrono` feature with RFC 3339 serialization, or at least `TimeZone=UTC` in the connect options.
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

### The settings store is built for more than two values

- **What:** a module-singleton store behind `useSyncExternalStore` also broadcasts a `window` CustomEvent that only the same module listens to, and it persists through a versioned, forward-compatible schema with legacy-key migration. All of this holds `theme` and `poemFontScale`.
- **Where:** `apps/web/src/lib/settings/settings-store.ts`, `apps/web/src/lib/settings/settings-storage.ts`, `apps/web/src/lib/settings/settings-schema.ts`
- **Why it's unusual:** `updateSettings` calls `notify()` and then dispatches the event, whose listener calls `notify()` again, so every update notifies subscribers twice. `applyTheme` also repeats `theme-init.astro`'s apply logic, while the approved entry covers only the parse duplication.
- **Normal approach:** nanostores `persistentAtom`, or the same `useSyncExternalStore` store without the window event and the version field.
- **Status:** Needs review

### Outbound links go through the API's `/v1/go/*` redirector

- **What:** About page and footer links point to `api.qafiyah.com/v1/go/{x,github,telegram,db,avatars,raaqim}`, while the same destinations exist as web constants for JSON-LD `sameAs`.
- **Where:** `apps/web/src/lib/urls.ts`, `apps/web/src/pages/about.astro`, `apps/web/src/components/footer.tsx`, `apps/web/src/lib/constants/site-meta.ts`, `apps/api/src/routes/go.rs`, `apps/api/src/lib.rs`
- **Why it's unusual:** each click is a round trip through the data API that counts against the visitor's anonymous quota, so a spent quota yields a 429 instead of a redirect. There are also two sources of truth for the URLs.
- **Normal approach:** link to the external URL directly from one web constant.
- **Status:** Needs review

### Browser telemetry goes through a separate Worker and domain

- **What:** the browser DSN host is rewritten to `t.qafiyah.com`, where a Cloudflare Worker with a hard-coded ingest host and project id, its own CORS, and its own deploy forwards events to Sentry.
- **Where:** `apps/telemetry-proxy/`, `apps/web/sentry.client.config.js`, `apps/web/sentry.server.config.js`, `apps/web/nginx-csp.conf`, `apps/web/nginx-csp-api.conf`, `docs/topology.md`
- **Why it's unusual:** changing the Sentry project touches three files and two deploys. `docs/topology.md` says API and web telemetry go through `t.qafiyah.com`, but `sentry.server.config.js` posts straight to the ingest host.
- **Normal approach:** `Sentry.init({ tunnel: '/monitoring' })` with a same-origin route shipped with web.
- **Status:** Needs review

### The search indexer diverges from the API's stack

- **What:** the indexer uses tokio-postgres where the API uses sqlx, and `Result<_, String>` throughout where conventions say `thiserror`. It also has a second hand-rolled `civil_from_days`, used only for a `lastReindexAt` field nobody reads, next to a `lastError` that is always null.
- **Where:** `apps/search-indexer/Cargo.toml`, `apps/search-indexer/src/main.rs`, `apps/search-indexer/src/pg.rs`
- **Why it's unusual:** one workspace ends up with two Postgres drivers and two error styles, and the approved `civil_from_days` entry lists only `apps/api/src/log.rs`. The pg tests assert on the SQL text instead of running it.
- **Normal approach:** one driver per workspace, `thiserror` or `anyhow`, and SQL tested against the database with the existing `rust:test:db`.
- **Status:** Needs review

### Branded slug types are never validated

- **What:** seven valibot brand schemas exist, six are used only to derive types, and routes cast `Astro.params` to the brand.
- **Where:** `apps/web/src/lib/api/brands.ts`, `apps/web/src/pages/poems/[slug].astro`, `apps/web/src/pages/poets/[slug].astro`, `apps/web/src/pages/poets/index.astro`, `apps/web/src/lib/seo/taxonomy-copy.ts`
- **Why it's unusual:** the brands suggest validation that never happens. A lint-disable in `taxonomy-copy.ts` claims "the route validates each slug before the lookup builds its brand", but the taxonomy routes pass `Astro.params.slug` straight through, and casts in `.astro` files escape oxlint's no-cast rule.
- **Normal approach:** `v.safeParse` at the route with a 404 on failure, or plain `string`, since the API already validates and its 400 becomes a 404.
- **Status:** Needs review

### The API's tests use homemade snapshot, randomness, and skip tooling

- **What:** ES query snapshots go through a dedicated `es-query-dump` binary, a Bun script, and a committed vectors file. An xorshift `Rng` drives the "never panics" loops, and DB tests return early, and so pass, when `QAFIYAH_TEST_*` is unset.
- **Where:** `apps/api/src/bin/es-query-dump.rs`, `scripts/es/query-snapshot.ts`, `apps/api/generated/es/query.vectors.json`, `apps/api/src/test_support.rs`, `apps/api/tests/db.rs`, `apps/api/src/es/query.rs`
- **Why it's unusual:** each piece is a homemade version of a standard crate. The dump binary ships in the release build, `rand` is already a dependency, and a skipped DB test shows up as a pass. `PoetSearchParams::default()` also enables highlighting that both production callers turn off, so only the dump and the tests use it.
- **Normal approach:** `insta::assert_json_snapshot!`, `proptest` (or a seeded `rand` RNG), and `#[ignore = "needs QAFIYAH_TEST_*"]` or `#[sqlx::test]`.
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
- **Where:** `apps/api/src/routes/poets.rs`, `apps/api/src/domain/search.rs`, `apps/api/src/domain/poets.rs`, `apps/api/generated/openapi/openapi.json`
- **Why it's unusual:** the two reads can disagree on counts until a reindex, and the contract shows the same `poemsCount` as `int64` in `PoetListItem` and `int32` in `PoetStats`.
- **Normal approach:** one source per resource, or at least one type for the shared field.
- **Status:** Needs review

### The footer is one React island

- **What:** `<Footer client:idle />` hydrates the whole footer, static links included, only to host `SettingsDialog` and `RandomPoemButton`.
- **Where:** `apps/web/src/layouts/layout.astro`, `apps/web/src/components/footer.tsx`
- **Why it's unusual:** static markup ships as JavaScript on every page. Because the footer wraps itself in `IslandErrorBoundary fallback={null}`, a crash in either widget removes the entire footer.
- **Normal approach:** a `footer.astro` with two small islands.
- **Status:** Needs review

### Poem result cards navigate with JavaScript

- **What:** `PoemCard` is a `div` whose `onClick` calls `window.location.assign(href)`, while `PoetCard` and `ListCard` are plain `<a>` elements.
- **Where:** `apps/web/src/components/search/result-cards.tsx`, `apps/web/src/components/ui-extended/list-card.tsx`
- **Why it's unusual:** middle-click, modifier-click, and link previews do not work on the card body, and the handler has to special-case text selection and nested links.
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

- **What:** poets load more through a hand-rolled IntersectionObserver in a horizontal row, with two rows built by filtering on `index % 2`, while poems load more through a button. `wantPoems` and `wantPoets` are hard-coded `true` but threaded through three components.
- **Where:** `apps/web/src/components/search/sections.tsx`, `apps/web/src/components/search/use-search.tsx`, `apps/web/src/components/search/search-container.tsx`, `apps/web/src/components/search/filters.tsx`
- **Why it's unusual:** one results page has two load-more UIs, and the flags leave dead branches.
- **Normal approach:** one load-more pattern, CSS `grid-flow-col grid-rows-2` for two rows, and no constant flags.
- **Status:** Needs review

### `lib/seo` fetches data and holds page copy

- **What:** the SEO layer loads taxonomy data and holds page headings and empty-state text in function-valued config tables, and the sitemap imports its data loading from there.
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

- **What:** a dev-only workspace app crawls the site and parses HTML with the same regexes and thresholds as `scripts/smoke/suites/seo.ts`, using hard-coded, data-dependent samples such as `/poets/GuRy`.
- **Where:** `apps/inspector/src/`, `scripts/smoke/suites/seo.ts`, `apps/inspector/AGENTS.md`
- **Why it's unusual:** the approved inspector entries cover its mechanics, not its existence or the duplication, which its `AGENTS.md` calls deliberate without giving a reason.
- **Normal approach:** Lighthouse or Unlighthouse SEO audits, or reporting the smoke suite's results. If a custom tool stays, use `HTMLRewriter` for parsing.
- **Status:** Needs review

### The database container is renamed after each restore

- **What:** after a restore, `tag_db_container` reads `COMMENT ON DATABASE` and runs `docker rename` on the Compose-managed container, giving it the name `<name>-<dump number>`.
- **Where:** `scripts/lib/tag-db-container.sh`, `scripts/db/init.sh`, `scripts/dev/run.ts`, `scripts/dev/db-up.sh`, `scripts/deploy/vps.sh`, `scripts/db/reseed.sh`, `docs/deployment/architecture.md`
- **Why it's unusual:** it renames a container that Compose owns just to show metadata in `docker ps`, and the helper is shipped over SSH on every deploy.
- **Normal approach:** a label, a status command, or the init log.
- **Status:** Needs review

### Plan limits are hard-coded in the developer page

- **What:** `RateLimits` hard-codes 60 anonymous and 500 free requests per hour, values the API owns (`ANON_REQUESTS` and the `plans` rows).
- **Where:** `apps/web/src/components/developers/rate-limits.astro`, `apps/web/src/pages/account/index.astro`
- **Why it's unusual:** it creates two sources of truth that can drift, and the account page already shows the real value from the API right above the table.
- **Normal approach:** read the limits from the API, or share one constant.
- **Status:** Needs review

### The Rust lint wall enables clippy's restriction group

- **What:** the workspace denies `arithmetic_side_effects`, `as_conversions`, `indexing_slicing`, `string_slice`, `print_stdout`, `print_stderr`, `exit`, `wildcard_enum_match_arm`, and more.
- **Where:** `Cargo.toml` (`[workspace.lints]`), `docs/rust-conventions.md`, and the `#[expect(..)]` sites across `apps/api` and `crates/elasticsearch`
- **Why it's unusual:** clippy's documentation says restriction lints are not meant to be enabled wholesale. Here they drive 20 `#[expect]` sites, repeated `match`/`eprintln!`/`exit` blocks in `main.rs`, and saturating math that silently clamps instead of surfacing overflow, while `overflow-checks` is also on in release.
- **Normal approach:** `clippy::all` plus selected pedantic lints, `unwrap_used` and `expect_used` if wanted, and `fn main() -> anyhow::Result<()>` for boot errors.
- **Status:** Needs review

### Operational runbooks live in agent skill files

- **What:** human-facing docs point to `.claude/skills/deploy/SKILL.md` and `.claude/skills/local-db-edit/SKILL.md` as the ordered runbook for deploy, rollback, reseed, and dump creation.
- **Where:** `.claude/skills/`, `data/db/MAINTAINERS_GUIDE.md`, `docs/deployment/README.md`, `docs/topology.md`, `apps/telemetry-proxy/AGENTS.md`
- **Why it's unusual:** a maintainer has to find procedures inside a tool-specific directory that `.gitignore` otherwise excludes.
- **Normal approach:** keep runbooks in `docs/deployment/` and have the skill point to them.
- **Status:** Needs review

### Scripts re-implement command-line basics, each in its own way

- **What:** about 19 scripts parse `process.argv` by hand (one defines its own `parseArgs`). There are four ANSI colour helpers, three ways to spawn processes, and three ways to list files.
- **Where:** `scripts/ci.ts`, `scripts/dev/run.ts`, `scripts/smoke/run.ts`, `scripts/smoke/surfaces.ts`, `scripts/dev/quota.ts`, `scripts/deploy/build-images.ts`, `scripts/lib/walk.ts`, `scripts/lib/tracked-files.ts`, `scripts/check/naming.ts`
- **Why it's unusual:** `ci.ts` and `run.ts` each carry copies of `paint`, `ensureDockerRunning`, the turbo and astro banner filters, and `formatMs`, and the copies have drifted (`quota.ts`'s colours ignore `NO_COLOR` and TTY). Hand-written parsing produced the smoke `--suite` bug noted above.
- **Normal approach:** `parseArgs` and `styleText` from `node:util`, one spawn API, and `git ls-files` or `Bun.Glob` for file lists.
- **Status:** Needs review

### Next.js scaffolding is left in an Astro app

- **What:** 15 files start with `'use client'`, `components.json` declares `"rsc": true` with `ui` and `hooks` aliases pointing at directories that do not exist, and `vitest.config.ts` re-declares the tsconfig aliases by hand.
- **Where:** `apps/web/components.json`, `apps/web/vitest.config.ts`, and the `'use client'` files under `apps/web/src/`
- **Why it's unusual:** the directive means nothing in Astro and suggests React Server Components to a new maintainer, and `shadcn add` would write components to the wrong place with RSC markers.
- **Normal approach:** `rsc: false` with aliases that match the tree, no directives, and Astro's `getViteConfig()` for vitest.
- **Status:** Needs review

### The poet bio toggle uses the checkbox hack

- **What:** a `peer sr-only` checkbox and two `<label>`s toggle the rest of the bio.
- **Where:** `apps/web/src/pages/poets/[slug].astro`
- **Why it's unusual:** a screen reader announces the toggle as a checkbox, and a native disclosure element exists for this.
- **Normal approach:** `<details><summary>عرض المزيد</summary>...</details>`.
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
- **Where:** `.oxlintrc.json` (it bans an `@qafiyah/api` package "via oRPC over HTTP" and targets code in `packages/*`, which holds only tsconfig JSON), `.dependency-cruiser.cjs` (the same `packages/*` rules), `turbo.json` (`SMOKE_*` variables no task uses), `scripts/check/naming.ts` (it ignores a nonexistent `tools` directory, and most of `ALLOWED_BASENAMES` has no effect), `.gitignore` (`packages/schemas/...`, `.next/`, `.vercel`, Python venv entries, `title-review.sqlite`, `.title-fix/`), `.dockerignore` (`.next`, `.vercel`, `tools/**/venv`), `.vscode/settings.json` (`editor.rulers: [80]` against `printWidth: 100`)
- **Why it's unusual:** a reader has to confirm each rule is dead before trusting the rest.
- **Normal approach:** delete them.
- **Status:** Needs review

## API (`apps/api`)

The API is not just a thin DB connector, and the crate carries no doc comments: these entries are its module-level intent. Read them before assuming something is incidental.

### Client address comes from proxy headers only behind the web nginx

- **What:** `client_ip.rs` resolves the caller from `CF-Connecting-IP`, falling back to the last `X-Forwarded-For` hop, and to a single shared bucket when neither is present.
- **Where:** `apps/api/src/client_ip.rs`
- **Why:** the headers are honored only when the connection's immediate peer is the web nginx on the dedicated `backend` network. A caller outside that subnet is bucketed on its own peer address, so a lateral container on the default bridge cannot spoof the header.
- **Normal approach:** trust the forwarding headers from any peer behind a single reverse proxy.
- **Date:** 2025-04-13

### API key lookups are cached, misses included

- **What:** `accounts/` resolves an `x-api-key` to a `Caller` by joining `api_keys` to `users` to `plans` in the separate `qafiyah_accounts` database, behind a 60-second in-memory cache that also caches misses.
- **Where:** `apps/api/src/accounts/cache.rs`, `apps/api/src/accounts/keys.rs`, `apps/api/src/bin/issue-key.rs`
- **Why:** caching misses stops key spraying from becoming a database amplifier. The cache flushes entirely at its ceiling rather than evicting, because entries rebuild cheaply and the ceiling only bounds memory under spraying. Only a key with the exact shape `keys::generate` and `bin/issue-key` emit (`qaf_` plus 32 ASCII alphanumerics) reaches the cache or the database; anything else is anonymous. A lookup that hangs or fails is bounded by a 500 ms timeout and cached as a 5-second miss, so an accounts outage costs one probe per key per TTL instead of one stall per request.
- **Normal approach:** query the database per request, or use an LRU cache crate with per-entry eviction.
- **Date:** 2026-09-21

### Every rate limit is checked in one pass

- **What:** `Limiter::check_all` evaluates every applicable limit together and increments every bucket or none.
- **Where:** `apps/api/src/rate_limit.rs`
- **Why:** a keyed caller faces three limits: the plan's `requests` per `WINDOW_SECONDS` bucketed on `users.id`, the plan's `burst` per `BURST_WINDOW_SECONDS` bucketed on the same user, and the plan's `ip_ceiling` bucketed on the client address (omitted when the plan carries no ceiling, only `free` does, and omitted when the address cannot be resolved rather than collapsing every such caller into one bucket). Three sequential `check` calls would let an address refusal silently burn the caller's own hourly allowance, so a refused request consumes none of the limits that allowed it. Response headers always describe the sustained user limit, while `Retry-After` on a 429 follows whichever limit actually refused. Quota is bucketed on the user, never the key, so rotating keys or holding several grants no extra allowance.
- **Normal approach:** one limiter middleware per limit, each checked in turn.
- **Date:** 2026-09-22

### The anonymous limit depends on the environment

- **What:** callers without a key share a per-address bucket sized by `ANON_REQUESTS`, which defaults to 60 under `ENVIRONMENT=production` and to effectively unlimited elsewhere.
- **Where:** `apps/api/src/config.rs::anon_requests`
- **Why:** outside production there is no Cloudflare in front, so every caller would share one bucket.
- **Normal approach:** one fixed default in every environment.
- **Date:** 2026-09-21

### Internal keys bypass rate limiting and the accounts database

- **What:** `API_KEY_INTERNAL` and `API_KEY_FULL` bypass every limit and never touch the accounts database, and every caller receives identical response bodies (no scope, no capping, no `Vary: x-api-key`).
- **Where:** `apps/api/src/auth.rs`, `apps/api/src/rate_limit.rs`
- **Why:** an accounts outage degrades the portal and not the website; a keyed caller during such an outage falls back to the anonymous bucket rather than being refused. Matches the crawler policy the API serves from `well-known/robots.api.txt`.
- **Normal approach:** resolve every key, internal ones included, through the same accounts lookup and plan limits.
- **Date:** 2026-06-30

### Key revocation, plan changes, and usage are eventually consistent

- **What:** resolved keys are held for `API_KEY_CACHE_TTL_SECONDS` (up to 60 seconds) with no cross-process invalidation, and `usage_hourly` counters live in memory, flush once a minute, and drop that minute when a flush fails.
- **Where:** `apps/api/src/accounts/cache.rs`, `apps/api/src/accounts/usage.rs`
- **Why:** a revoked key keeps working until its entry expires in each replica; shortening the TTL trades a larger database load for a smaller window. Usage is a reporting number, not billing, so a failed flush is not retried.
- **Normal approach:** invalidate across processes through a shared cache, and write usage per request or retry failed writes.
- **Date:** 2026-09-21

### `/account/*` is protected three ways

- **What:** denied at the edge, guarded by `API_KEY_INTERNAL` alone, and merged outside both the `cached` and `limited` routers, with `no-store` on every response.
- **Where:** `apps/web/nginx.conf` (404 for `^~ /account` on `api.qafiyah.com`), `apps/api/src/routes/account.rs::guard`, `apps/api/src/auth.rs::Keys::is_internal`
- **Why:** `guard` accepts only what `Keys::is_internal` matches, so neither a user's own API key nor `API_KEY_FULL` opens it, even though both bypass the limiter on `/v1`. Staying outside `cached` means `cache::layer` never stamps `private, max-age=300` on a session payload. Any one of the three would usually be enough; the point is that a mistake in one is survivable, because a shared-cache hit is served without ever reaching the origin.
- **Normal approach:** a single auth middleware on the routes.
- **Date:** 2026-09-21

### `users::upsert` trusts its caller about email verification

- **What:** `accounts/users.rs::upsert` does not check verification; `exchange.ts` in web requires Google's `email_verified` to be true and GitHub's email to be both primary and verified.
- **Where:** `apps/api/src/accounts/users.rs`, `apps/web/src/lib/server/oauth/exchange.ts`
- **Why:** web owns OAuth, so it is the side that sees the provider's claims. Linking accounts by email is an account-takeover vector without the check. Upsert resolves by `(provider, provider_uid)` first and updates that account's email, so a provider email change keeps the same account and its keys; when the new email already belongs to a different account the upsert is refused with a 409 `EMAIL_TAKEN` rather than silently merging, leaving both accounts' keys untouched.
- **Normal approach:** an auth library that verifies and links accounts where the user row is written.
- **Date:** 2026-09-21

### `build.rs` exists only to watch `migrations/`

- **What:** `apps/api/build.rs` does nothing but emit `cargo:rerun-if-changed=migrations`.
- **Where:** `apps/api/build.rs`
- **Why:** `sqlx::migrate!` embeds the SQL at compile time and cargo does not watch `migrations/` on its own, so a new migration file is silently ignored until something else triggers a rebuild.
- **Normal approach:** no build script.
- **Date:** 2026-09-21

### Shipped migrations are never edited, not even reformatted

- **What:** a fix is always a new migration, no formatter touches `.sql` files, and a shipped migration that squawk flags is excluded in `.squawk.toml` rather than edited.
- **Where:** `apps/api/migrations/`, `.squawk.toml`
- **Why:** sqlx checksums every applied migration and API startup refuses one whose bytes changed. `bun run check:sql` runs squawk over new migrations (lock and timeout hazards, a `NOT NULL` column without a default, a blocking index build); `0001_accounts.sql` predates that and is excluded, and a squawk upgrade that flags a shipped migration gets the same exclusion.
- **Normal approach:** fix lint and format findings in place.
- **Date:** 2026-09-23

### `js.rs` mirrors JavaScript runtime semantics

- **What:** ECMAScript's whitespace set and integer-safe number serialization, reimplemented in Rust.
- **Where:** `apps/api/src/js.rs`
- **Why:** the TS client must see exactly what JavaScript would produce. These are not general text utilities; don't reach for them outside that purpose.
- **Normal approach:** Rust's own `char::is_whitespace` and `serde_json` number output.
- **Date:** 2026-09-14

### Production drops most successful-request logs

- **What:** `should_emit` drops about 95% of ordinary successful-request logs in production; errors, slow requests, and empty results are always kept.
- **Where:** `apps/api/src/log.rs`
- **Why:** keeps the log to the requests worth reading.
- **Normal approach:** log every request and filter by level.
- **Date:** 2026-09-12

### `civil_from_days` hand-rolls date math

- **What:** days since the epoch are converted to a civil date by hand.
- **Where:** `apps/api/src/log.rs::civil_from_days`
- **Why:** avoids a date-crate dependency. It is not a general calendar utility.
- **Normal approach:** the `time` or `chrono` crate.
- **Date:** 2026-09-12

### `/poems` SQL is assembled by hand with planner special cases

- **What:** `clauses` builds the list's `WHERE` with `format!` and `AssertSqlSafe`: a scalar `= (SELECT id ...)` lookup for one slug and `IN (SELECT id ... ANY)` for several, always behind `p.recension_of_id IS NULL`. The page is picked by id first and joined afterwards, and the total comes from the facet's `*_stats` row when the filter is one value of one facet, from `COUNT(*)` otherwise.
- **Where:** `apps/api/src/domain/poems.rs`, `apps/api/src/domain/taxonomy.rs`, `scripts/db/sql/refresh-taxonomy-stats.sql`
- **Why:** each special case is a measured planner win on the 346k-poem corpus (pgbench, prepared statements, 2026-09-26). A scalar lookup lets Postgres walk the `(facet, id)` partial index in id order and stop at the page, where `IN` joins and sorts every match: one theme's middle page 5.0 ms against 22.7 ms, its last page 9.8 against 24.8. Picking the page by id before joining poets and meters keeps the skipped rows inside that index, making deep pages 8 to 14 times faster. The `*_stats` total spares a count of up to about 200k rows on every single-term page, taking page 1 of a large facet from about 10 ms to 1 ms. The two count paths agree only while `refresh_taxonomy_stats()` has run since the last change to `poems`; `a_single_term_total_from_the_stats_table_equals_a_live_count_of_primaries` guards that.
- **Normal approach:** `sqlx::QueryBuilder` with `push_bind` and a single `COUNT(*)` path.
- **Date:** 2026-09-24

## Web (`apps/web`)

Paths are relative to `apps/web/src/` unless they start at the repo root.

### Two API clients, and no key ever reaches the browser

- **What:** `apiServer` calls the internal API URL with `INTERNAL_API_KEY` for SSR; `apiBrowser` is keyless and calls `/api/v1` on the page's own origin, a proxy that forwards only the two paths in the allowlist (`search`, `poems/random`) and attaches the internal key server-side.
- **Where:** `lib/server/client.ts`, `lib/api/browser-client.ts`, `pages/api/v1/[...path].ts`, `lib/api/proxy-allowlist.ts`
- **Why:** the allowlist is the security boundary: without it the route would be an unauthenticated tunnel to the whole corpus. Don't use one client from the other's context, and don't widen the allowlist without reading the API rate-limiting entries above.
- **Normal approach:** the browser calls the public API directly.
- **Date:** 2026-09-21

### Web owns identity; the API owns the accounts database

- **What:** OAuth, the `qaf_session` cookie, and every account screen live in web, which never connects to Postgres and calls `/account/*` on the API over the internal Docker network.
- **Where:** `lib/server/oauth/`, `lib/server/session.ts`, `pages/auth/`, `pages/account/`, `lib/server/account-client.ts`
- **Why:** `account-client.ts` is separate from `lib/server/client.ts` because that one bakes `/v1` into its base URL and the account routes are deliberately outside the public contract.
- **Normal approach:** the web server reads its own session and user tables through an auth library's database adapter.
- **Date:** 2026-09-21

### Authenticated pages opt out of the nginx cache, redirects included

- **What:** `/account`, `/api/me`, `/auth/` and `/login` are listed in the `$skip_astro_cache` map, which `@astro` passes to `proxy_no_cache` and `proxy_cache_bypass`, and every one of those responses sets `no-store`, the 302s included.
- **Where:** `apps/web/nginx.conf`
- **Why:** the astro cache zone keys on `"$host$uri$is_args$args"` with no cookie. The skip has to be read in `@astro`: every location hands off with `try_files $uri @astro`, and after that internal redirect nginx applies only the named location's directives, so a `proxy_no_cache` in the original location is silently ignored. Adding an authenticated route without both the map entry and `no-store` serves one reader's page to the next.
- **Normal approach:** bypass the cache whenever a session cookie is present.
- **Date:** 2026-09-21

### New server-side env vars go in `turbo.json`'s `dev.passThroughEnv`

- **What:** every server-side env var is listed in `dev.passThroughEnv` (today `INTERNAL_API_KEY`, the four OAuth variables, and `SESSION_STATE_SECRET`).
- **Where:** `turbo.json`, `lib/server/env.ts`
- **Why:** Turborepo passes an explicit allowlist to the dev task, so anything missing from it reads as `''` in `env.ts` and the feature silently behaves as if unconfigured.
- **Normal approach:** the dev server inherits the whole environment.
- **Date:** 2026-06-15

### Form POSTs rely on Astro's origin check instead of a CSRF token

- **What:** no CSRF token; Astro rejects a POST without a matching `Origin` header with `403 Cross-site POST form submissions are forbidden` before the route runs, on top of the `SameSite=Lax` session cookie.
- **Where:** `pages/account/`, `pages/auth/`, `apps/web/astro.config.mjs` (`security.allowedDomains`)
- **Why:** the framework check plus `SameSite=Lax` already covers cross-site forms. The check compares `Origin` with the request URL, and TLS ends before nginx, so Astro only sees `https://qafiyah.com` because `security.allowedDomains` lets it trust nginx's `X-Forwarded-Proto: https`; without that every production POST is a 403. Testing these routes with curl requires an explicit `-H 'Origin: ...'`.
- **Normal approach:** a per-form CSRF token.
- **Date:** 2026-09-21

### A 400 from the API renders as a 404

- **What:** `isNotFoundStatus` treats the API's 400 (out-of-range page, bad slug) the same as its 404, so every `getX`/`getXPage` fetcher returns `null` for either and the page rewrites to `/404`.
- **Where:** `lib/server/api-error.ts`
- **Why:** to a reader, an out-of-range page or a bad slug is a page that does not exist.
- **Normal approach:** render a 400 as an error page.
- **Date:** 2026-09-13

### SSR retries transient network errors only

- **What:** `safeCall` backs off and retries only what `is-transient-network-error.ts` recognizes as connection-level (aborts, `ECONNRESET`-style codes, browser "failed to fetch" messages); a real 4xx or 5xx from the API is not retried.
- **Where:** `lib/server/unwrap.ts`, `lib/observability/is-transient-network-error.ts`
- **Why:** a connection blip is worth a retry; an API error response is a real answer.
- **Normal approach:** the HTTP client's own retry option, or a retry library.
- **Date:** 2026-06-27

### The nav account control renders before hydration from client-side hints

- **What:** `viewer-init.astro` reads the non-HttpOnly `qaf_viewer=1` hint cookie and the cached viewer in localStorage (`qafiyah-viewer`, holding a 48px data-URL avatar), then sets `data-viewer` and `--viewer-avatar` on `<html>` before first paint; the SSR markup carries the known-viewer link and a skeleton, and CSS picks one.
- **Where:** `components/layout/viewer-init.astro`, `components/layout/account-control.tsx`
- **Why:** pages are cached without cookies, so SSR can't know the viewer, and this way the control never flashes or shifts the nav. The hint cookie is set at login, cleared at logout, and resynced by `account-control.tsx` from `/api/me`. Keep those states rendering the same markup SSR and on hydration. Anonymous visitors see no account control at all (the slot and its nav `<li>` are hidden unless `data-viewer` is set, and `account-control.tsx` keeps that attribute in sync with `/api/me`): accounts only serve API keys for now, so the only way in is `/developers`, linked from the footer and about page.
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

### Taxonomy options are regenerated before every deploy

- **What:** unlike the other generated files, `taxonomy-options.gen.ts` is regenerated and committed by the deploy skill before each deploy, not just when its generator script changes.
- **Where:** `lib/generated/taxonomy/taxonomy-options.gen.ts`, `.claude/skills/deploy/SKILL.md`
- **Why:** eras, meters, rhymes, themes, and collections rows change independently of any code change, so the file drifts from the database on its own.
- **Normal approach:** regenerate only when the generator changes, or fetch the options at runtime.
- **Date:** 2026-09-16

### Taxonomy selects keep the API's order

- **What:** `Select` is given `sortOptions={false}` everywhere taxonomy options render.
- **Where:** `components/ui-extended/select.tsx`
- **Why:** each list's order (e.g. eras' chronological order) comes from the API's own `ORDER BY`, not alphabetical.
- **Normal approach:** the component's default sort.
- **Date:** 2026-06-18

### The bundled Amiri fonts are patched to draw `٬` as the ASCII comma

- **What:** both Amiri `.woff2` files map U+066C (Arabic thousands separator) to the glyph of the ASCII comma `,` (U+002C), not Amiri's own `٬` glyph and not the Arabic comma `،` (U+060C).
- **Where:** `apps/web/src/assets/fonts/Amiri-Regular-400.woff2`, `apps/web/src/assets/fonts/Amiri-Bold-700.woff2`
- **Why:** `Intl.NumberFormat('ar-SA')` groups with U+066C, and Amiri draws it as a small raised mark that reads like an apostrophe between digits. Amiri has no alternate glyph for it that CSS could select, and swapping the character in code would put a Latin comma into the text that copy/paste and screen readers see. Amiri is OFL 1.1 with no Reserved Font Name, so the patched files keep the name. Replacing or re-downloading the fonts drops the patch; reapply it with fontTools: for each file, `f = TTFont(path)`, set `table.cmap[0x066C] = table.cmap[0x2C]` in every `f['cmap'].tables` entry that has U+066C, then `f.save(path)`.
- **Normal approach:** ship the font files unmodified.
- **Date:** 2026-09-24

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
- **Why:** a page linking many redundant candidates for an already-resolved shape (e.g. an index page listing dozens of items) must not exhaust the budget before a different, still-unresolved shape's legitimately needed candidate is reached.
- **Normal approach:** cap the number of URLs visited.
- **Date:** 2026-09-17

## Corpus database (`scripts/db`)

### Restores apply schema SQL instead of migrations

- **What:** `scripts/db/init.sh` runs idempotent SQL files on every restore: `poem-aliases.sql`, `merge-poem.sql`, `poem-recensions.sql`, `poet-aliases.sql` and `merge-poet.sql` create the `poem_aliases` and `poet_aliases` tables, the `poems.recension_of_id` column, their constraints and the primaries-only partial indexes if missing, drop the indexes those replace, and replace the maintenance functions, beside the existing `refresh-poem-relations.sql` and `refresh-taxonomy-stats.sql`.
- **Where:** `scripts/db/init.sh`, `scripts/db/sql/`
- **Why:** the corpus database is shipped as whole dumps and has no migrations (`apps/api/CLAUDE.md`), so a restore is the one moment a schema change can meet an existing dump. Applying the files there lets current code run against an older dump, and keeps the functions reviewable as files instead of living only inside dumps. A new dump already carries the same schema, so on it every statement is a no-op.
- **Normal approach:** versioned migrations run on deploy, the way `apps/api/migrations/` manages the accounts database.
- **Date:** 2026-09-26

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

### `posthog.astro` stays minified

- **What:** the vendor snippet is kept minified; `.prettierignore` skips it and oxlint ignores it too.
- **Where:** `apps/web/src/components/layout/posthog.astro`, `.prettierignore`
- **Why:** it is a vendor snippet, kept as shipped.
- **Normal approach:** format and lint it like the rest of the code.
- **Date:** 2026-09-24

### `prettier-plugin-astro` can add whitespace inside a nested element

- **What:** when it wraps a long line, an element nested in a `{...}` expression can gain whitespace (`<span>text</span>` becomes the tag, the text, and the closing tag on three lines).
- **Where:** `.astro` files under `apps/web/src/`
- **Why:** inside a flex or grid container that whitespace is dropped, which covers every case in the app today; in inline text it renders as a space. Code that reads an element's text reads it trimmed (the API key copy button does).
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
