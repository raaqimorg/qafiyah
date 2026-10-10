# Environments and Configuration

## Prerequisites (VPS)

1. **Docker and Docker Compose**, with the repo at `/opt/qafiyah` in a sparse checkout (see "The server checkout" below).
2. **sops, age, and the VPS age key.** See `docs/deployment/secrets.md`. Every deploy generates the root `.env` from `secrets/prod.enc.env`. Compose loads that file automatically for `${VAR}`. Never edit it on the server.
3. **A Cloudflare Tunnel** (or an equivalent) for ingress and TLS. It reaches the gateway over loopback: both hosts go to `127.0.0.1:80`, and nginx routes by `Host`.

The API's `DATABASE_URL` uses the read-only role `qafiyah_api`, with `PG_READER_PASSWORD` and `POSTGRES_DB`. Its `DATABASE_URL_ACCOUNTS` uses the role `qafiyah_accounts`, with `PG_ACCOUNTS_PASSWORD`. Local development needs no `.env`: `scripts/dev/compose.sh` sets default dev passwords (see `docs/development.md`).

## Starting the stack and seeding

```bash
docker compose up -d --build   # or, from a dev machine: bun run deploy
```

This builds and starts every service. Dependency conditions set the order:

1. `db` and `elasticsearch` become healthy.
2. `search-indexer` runs to completion.
3. `api` starts after the indexer exits with 0, and becomes healthy.
4. `web` starts and becomes healthy.
5. `edge-gateway` starts.

The observability containers follow their own conditions. `db-monitor-role` runs once after `db` is healthy, and `postgres-exporter` waits for it. `elasticsearch-exporter` waits for the indexer. `grafana` waits for a healthy `prometheus`, and `alloy` waits for `loki`.

**First boot only:** on an empty data volume, Postgres restores the newest dump from `data/db/` through `scripts/db/init.sh`. This takes a few minutes. The `start_period` of the `db` healthcheck is 300 seconds. The `db` container is then renamed to `<container>-<dump-number>` (for example, `qafiyah-db-0019`), so `docker ps` shows which dump it runs. Later starts use the same volume, so the name stays until the next real restore. To wipe and seed again locally, run `bun run db:reset`.

To ship a new Postgres or Elasticsearch dump to production, follow the ordered steps (`bun run db:reseed`) in `.claude/skills/deploy/SKILL.md`. A major version upgrade of `postgres` or `elasticsearch` needs a volume wipe; see `docs/deployment/troubleshooting.md`.

## The server checkout

The checkout at `/opt/qafiyah` leaves out the old dumps and the avatar snapshots. Every dump adds about 0.4 GB, and the server only restores the newest one. The site reads avatars from R2, never from `data/avatars/`. Without this, the disk fills up: during the version 6 release, Elasticsearch passed its 90% and 95% disk watermarks, and the reseed and the first deploy failed.

It is a non-cone sparse checkout. Show the current patterns on the server:

```bash
git -C /opt/qafiyah sparse-checkout list
```

The patterns keep everything (`/*`), then leave out `/data/avatars/` and each old folder of `data/db/`. A new dump folder is not in the list, so a deploy checks it out by itself. `git reset --hard` keeps the sparse checkout.

At each release that ships a new dump, do these steps:

1. Run `df -h /` on the server before `bun run db:reseed`. Elasticsearch builds the new index (about 1.1 GB) beside the old one, and refuses new shards above 90% disk.
2. After the release, add the dump folders older than the live one to the list, so they leave the disk.

Add a folder to the list (here `0044_10_10_2026`, after a later dump is live):

```bash
cd /opt/qafiyah
git sparse-checkout add '!/data/db/0044_10_10_2026/'
```

Set it up on a new server, after the first clone (list every dump folder except the newest):

```bash
cd /opt/qafiyah
git sparse-checkout set --no-cone '/*' '!/data/avatars/' '!/data/db/0031_23_09_2026/'
```

To undo it, run `git sparse-checkout disable`. All files come back on the next checkout.

## Secrets

The production secrets are encrypted in `secrets/prod.enc.env`. They reach the VPS as a generated `/opt/qafiyah/.env`, with mode `600` and ignored by git. To change them, run `bun run secrets:edit prod`. Never change them on the server, and never paste values into a doc. See `docs/deployment/secrets.md`.

## Rate limiting and API keys

The API serves **the same response body to every caller**. There is no capped data, no scope, and no `Vary: x-api-key`. Only the number of requests an hour changes from caller to caller, and each response's `X-RateLimit-*` headers report it.

That is why JSON reads are `Cache-Control: private, max-age=300`. The caller's own browser can use a response again for five minutes. No shared cache (Cloudflare, nginx) can store one, because it would give one caller's counters to the next caller, and cached hits would skip the count.

The website's `/api/v1/search` proxy removes those headers, so every visitor gets the same response. The proxy sets its own `public` policy on successful searches (`max-age=300, stale-while-revalidate=86400`). The website's nginx serves a cached search as fresh for five minutes. After that it serves the stored answer and refreshes it in the background, so an entry lives until a day passes without a request for it (`inactive=24h`). A deploy clears the cache. The stack smoke run checks that a repeated search is a cache hit. Cloudflare also caches a successful search in each data center, fresh for the same five minutes (`services.md`, "Cloudflare caches pages too").

Anonymous callers share an hourly bucket for each address:

- An IPv6 caller is counted by its /64, which is the block that one subscriber usually gets. So rotating addresses inside it gives no extra allowance.
- Its /48 shares a second bucket of ten times the anonymous limit. So rotating across /64s stops there.

Website visitors never reach those buckets, because the site calls the API with `API_KEY_INTERNAL`. A visitor's search through the site gets its own buckets instead (below). Each caller with a key gets its own bucket and its own number. A caller over the limit gets `429` as `application/problem+json`, with `Retry-After`. Every response carries `x-ratelimit-limit`, `x-ratelimit-remaining`, and `x-ratelimit-reset`.

### Where requests are limited

There are three layers, outermost first.

**Cloudflare** has three custom rules and one rate limiting rule, set in the dashboard. The Free plan allows five custom rules and one rate limiting rule.

The custom rules block a request with Cloudflare's own `403`, so it never reaches the tunnel. In order:

1. **Scanner tools and paths:**
   - a path segment that starts with a dot, except `/.well-known/`
   - `/wp-` anywhere in the path
   - script, configuration, backup, archive, and key file extensions (`php`, `env`, `sql`, `bak`, `zip`, `pem`, and others)
   - well-known admin and product paths (`/admin`, `/phpmyadmin`, `/cgi-bin/`, `/actuator`, and others)
   - the default user agents of scanners (`sqlmap`, `nikto`, `nuclei`, `nmap`, and others)
2. **Attack payloads in the URL:** the decoded, lowercased URL contains a common injection marker (`<script`, `union select`, `/etc/passwd`, and others).
3. **Methods the site never uses:** any method but `GET`, `HEAD`, and `OPTIONS` on the four site hosts. `POST` to `/account/keys` and `/auth/logout` is allowed, and so is `/cdn-cgi/*`, where a Cloudflare challenge posts.

The edge gateway's WAF still runs in `DetectionOnly` (`services.md`). So these rules are the only layer that refuses such probes. They stop a tool with its default settings, not a person who changes them.

- Before you add a route under a blocked path or extension, or a new `POST` route, change the rule in the dashboard.
- Keep one custom rule free for an incident.
- `bun run smoke:prod` sends a few requests that these rules block. Those probes expect only "no 5xx", so a `403` passes.

The rate limiting rule is a flood guard far above what a visitor sends:

- A client address that sends more than 100 counted requests in 10 seconds is blocked for 10 seconds with Cloudflare's own `429` (error 1015).
- It does not count `/_astro/*`, `/cdn-cgi/*`, or `/poets/*/avatar.webp`, which Cloudflare serves from its cache. So a page costs about 3 counted requests: the document, and `/api/me` twice.
- On 2026-10-09, a first visit sent 26 requests, and 20 of them were `/_astro/` files. Before the exclusion, four visitors behind one address could trip the rule.
- 100 is about twice what a fast reader sends, and equals the free API plan's burst for 10 seconds. A premium or enterprise key at its full burst can trip it.
- On the Free plan, the rule can match only the path and the verified bot flag. So it cannot tell `qafiyah.com` from `api.qafiyah.com`.
- Verified bots are exempt, and each Cloudflare data center keeps its own count. A request that a custom rule blocked does not count.
- A full `bun run smoke:prod` from one address can trip it.
- It counts each IPv6 address alone, not by /64, so it does nothing against rotation inside a block. On 2026-09-28, 182 requests from two addresses in one /64 through one data center passed. 180 requests from one address were blocked after about 110.
- A client's new connections can also reach different data centers, which splits its count.

**The web nginx** applies `limit_req` from `apps/web/nginx.conf`:

- 60 requests a minute for each address, with a burst of 30, on `/api/v1/`, `/account`, `/api/me`, and `/auth/`.
- It counts the exact address, so an IPv6 client that rotates inside its /64 gets a new allowance each time.
- It also holds each address to 10 requests in flight at once (`limit_conn`), on `/api/v1/` and on `api.qafiyah.com` (which has no `limit_req`). So one address cannot hold more than half of the API's 20 Postgres connections. The extra requests get the same 429.

**The API** applies the hourly buckets above to callers of `api.qafiyah.com`:

- The website's server-side calls carry `API_KEY_INTERNAL` and skip the buckets.
- The website's browser proxy also sends the visitor's address as `CF-Connecting-IP`, from the `X-Real-IP` that nginx sets. This covers `/api/v1/search`, `/api/v1/poems/random`, and, for a request that names exactly one poet, `/api/v1/poems` and `/api/v1/poems/facets`.
- The API counts those requests for each visitor: `VISITOR_REQUESTS` (3,600, nginx's steady rate) an hour for each /64 or IPv4 address, and ten times that for each /48. These buckets are separate from those of anonymous callers.
- Only an address that the web container forwards counts. Page renders forward none, so they stay unlimited.
- A search that Cloudflare answers from its cache reaches neither nginx nor the API, so no limit counts it.
- Locally, `bun run dev` has no nginx to set the address, so its proxy stays unlimited. In the Docker stack, every local request reaches nginx as one address. nginx already holds that address to 60 a minute, which is the same 3,600 an hour.
- A refused search is a `429` marked `no-store`. The proxy passes it through, and nginx does not cache it.

### Environment keys (they skip the limiter)

The API's environment holds two keys, each generated with `openssl rand -hex 32`. The API checks them before it reads the accounts database:

- `API_KEY_INTERNAL` (secret): unlimited, and the only key that opens `/account`. The web server-side renderer and the same-origin search proxy send it. A proxied request that carries a visitor's address counts against that visitor (above).
- `API_KEY_FULL` (secret): unlimited on `/v1` only. It is for trusted server clients, smoke tests, and admin tooling. It does **not** open `/account`, so a leak costs corpus reads, never account administration.

These keys do not touch `qafiyah_accounts` on purpose. So an outage of the accounts database harms only the developer portal, never the website.

**The web service's `INTERNAL_API_KEY` is `API_KEY_INTERNAL`.** Compose and `bun run dev` both take it from that one variable, so the two cannot differ. Never add `INTERNAL_API_KEY` to a secrets file; `secrets:check` refuses it.

The key is server-only: the renderer reads it, and it never reaches the browser. No key of any kind ships in the browser bundle. The browser calls `/api/v1/search`, `/api/v1/poems/random`, `/api/v1/poems`, and `/api/v1/poems/facets` on its own origin. `apps/web/src/lib/api/proxy-allowlist.ts` refuses every other path, and every `poems` or `poems/facets` request that does not name exactly one poet.

### The anonymous limit

`ANON_REQUESTS` sets the bucket for callers without a key, for each address, over `WINDOW_SECONDS` (one hour). **Leave it unset in production.** When it is unset, the API uses 60 if `ENVIRONMENT=production`, and an unlimited value everywhere else (`apps/api/src/config.rs::anon_requests`). Outside production, there is no Cloudflare in front, and every caller falls into the same bucket.

### Plans

Callers with a key get their limits from the `plans` table in `qafiyah_accounts`, not from the environment. Three limits apply, and a request must pass all three:

- the plan's `requests` for each `WINDOW_SECONDS`, counted for each user
- the plan's `burst` for each `BURST_WINDOW_SECONDS`, counted for each user
- the plan's `ip_ceiling`, counted for each client address

Only a plan with an `ip_ceiling` counts by address, and only `free` has one. So a paying customer is never limited for sharing an office address with free accounts.

To change a plan, run `update plans set requests = 1000 where slug = 'free';`. To move a user, run `update users set plan = 'premium' where email = '...';`. Each change takes up to 60 seconds to apply, because `accounts/cache.rs` keeps resolved keys for `API_KEY_CACHE_TTL_SECONDS`.

`ENVIRONMENT` matters more than it seems. If it is ever not exactly `production`, the limiter stops working without a message. So `secrets:check` requires it to be exactly `production` in `secrets/prod.enc.env`. An empty internal key would put the site's own server-side renderer in the anonymous bucket of 60 an hour. So in production, the API does not start without `API_KEY_INTERNAL` and `API_KEY_FULL`, and the web container does not start without `INTERNAL_API_KEY` and `SESSION_STATE_SECRET`.

The client address comes from `X-Forwarded-For`, where Cloudflare adds the visitor as the last entry (see `services.md`). The web container's nginx reads it and sends it again as `CF-Connecting-IP` over the dedicated `backend` network. The API accepts that header only from the `backend` subnet (`client_ip.rs`). So a container on the default bridge is counted by its own address, not by a forged header.

### Issued keys

Every other key is a row in the `qafiyah_accounts` database: a SHA-256 hash of the key, and a `prefix` for display. A key has no allowance of its own. The limits come from the owner's plan. So to raise one developer's limit, move them to another plan. Do not edit their key.

To issue a key, run:

```bash
DATABASE_URL_ACCOUNTS=... cargo run -p qafiyah-api --bin issue-key -- <email> [label]
```

It uses the same library calls as the account API:

- It normalizes the email, and creates the user if the user is new.
- It refuses a user who already holds `MAX_ACTIVE_KEYS_PER_USER` active keys, until one is revoked.
- It prints the raw key once. Only its hash is stored, so nobody can recover the key later.

Two delays are deliberate. Know them before you debug either one:

- A revoked key keeps working for up to 60 seconds (`API_KEY_CACHE_TTL_SECONDS`), because other processes get no signal to clear their cache.
- `usage_hourly` is up to a minute behind, because the counters are written in batches, not on the request path.

### OAuth for the account portal

Sign-in is on `qafiyah.com`, not on the API. So these five variables belong to the **web** service:

```
OAUTH_GOOGLE_CLIENT_ID
OAUTH_GOOGLE_CLIENT_SECRET
OAUTH_GITHUB_CLIENT_ID
OAUTH_GITHUB_CLIENT_SECRET
SESSION_STATE_SECRET
```

Register these exact redirect URIs with each provider. Otherwise, the provider refuses the callback before it reaches us:

```
https://qafiyah.com/auth/callback/google
https://qafiyah.com/auth/callback/github
```

`apps/web/src/lib/server/oauth/providers.ts` fixes the scopes: `openid email profile` for Google, and `read:user user:email` for GitHub. GitHub's `user:email` is required. Without it, `/user/emails` returns 403, and the verified-email check that prevents account takeover cannot run.

`SESSION_STATE_SECRET` is `openssl rand -hex 32`. It protects only the short-lived OAuth state cookie. So a new value cancels the sign-ins in progress, and nothing else. Sessions are rows in `qafiyah_accounts.sessions`, and a new value does not affect them.

The account portal also depends on `INTERNAL_API_KEY`: `account-client.ts` sends it to reach `/account/*`. If it is empty, every visitor appears signed out.

For local development, these variables and `INTERNAL_API_KEY` must also be in `turbo.json` under `dev.passThroughEnv`. Otherwise, Turborepo hides them from `astro dev`, and the portal acts as if no provider were set up.

## The accounts database

On an empty data volume, `scripts/db/accounts-init.sh` creates the `qafiyah_accounts` role and database on the first boot. The API then runs its Diesel migrations (`apps/api/migrations/`) against that database at startup, so the tables create themselves.

These steps set up its password and backups on a new server:

1. **Set `PG_ACCOUNTS_PASSWORD` in `secrets/prod.enc.env`.** Generate it with `openssl rand -hex 32`. Both the `db` and `api` services guard it with `:?`, so `docker compose` does not start without it. It must not equal `PG_READER_PASSWORD`.
2. **Create a private R2 bucket for backups, and a token that only it accepts.**
   - Do not use `qafiyah-assets`, which is public at `cdn.qafiyah.com`. The new bucket gets no public binding and no custom domain.
   - Add a lifecycle rule that deletes `accounts/` objects after 30 days.
   - Create an R2 API token with object read and write on that bucket only, not the deploy token. Then a compromised VPS cannot reach the public asset bucket. The token gives an access key ID, a secret access key, and the S3 endpoint.
3. **Set the `ACCOUNTS_BACKUP_*` values in `secrets/prod.enc.env`, then install the timer.** See "Accounts database backups" in `docs/deployment/services.md`. It lists the variables, the two install commands, and how to restore.

If a volume already holds data but has no accounts database, the initdb script never runs. This happens because Postgres runs `/docker-entrypoint-initdb.d` only on an **empty** volume. In that case, run its SQL once against the live container:

```bash
docker compose exec db psql -v ON_ERROR_STOP=1 -v accounts_pw="$PG_ACCOUNTS_PASSWORD" \
  -U "$POSTGRES_USER" -d "$POSTGRES_DB" <<'SQL'
SELECT 'CREATE ROLE qafiyah_accounts LOGIN'
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'qafiyah_accounts')
\gexec
ALTER ROLE qafiyah_accounts WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS PASSWORD :'accounts_pw';
SQL

docker compose exec db psql -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" <<'SQL'
SELECT 'CREATE DATABASE qafiyah_accounts OWNER qafiyah_accounts'
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = 'qafiyah_accounts')
\gexec
SQL
```

To check the tables after the next API start, run `docker compose exec db psql -U "$POSTGRES_USER" -d qafiyah_accounts -c '\dt'`.

To check that the whole path works after a deploy, run:

```bash
curl -si https://api.qafiyah.com/v1/meters | grep -i x-ratelimit
```

Expect `x-ratelimit-limit: 60`. If it shows a very large number, `ENVIRONMENT` is not `production`.
