# Environments & Configuration

## Prerequisites (VPS)

1. **Docker + Docker Compose**, repo at `/opt/qafiyah`.
2. **sops + age and the VPS age key**, see `docs/deployment/secrets.md`. The root `.env` (which Compose auto-loads for `${VAR}`) is generated from `secrets/prod.enc.env` on every deploy; never edit it on the box.

`DATABASE_URL` is composed from `POSTGRES_USER` / `POSTGRES_PASSWORD` / `POSTGRES_DB`. (Local dev needs no `.env`; `scripts/dev/compose.sh` defaults the dev passwords, see `docs/development.md`.)

3. **Cloudflare Tunnel** (or equivalent) for ingress/TLS, reaching the gateway over loopback: both hosts → `127.0.0.1:80` (nginx routes by `Host`).

## Bring up the stack & seeding

```bash
docker compose up -d --build   # or, from a dev machine: bun run deploy
```

Builds and starts all six, gated by dependency conditions: `db`/`es` healthy → `search-indexer` runs to completion → `api` (waits for search-indexer exit 0) → `api` healthy → `web` healthy → `edge-gateway`.

**First boot only:** on an empty data volume, Postgres auto-restores the newest dump from `data/db/` via `scripts/db/init.sh` (a few minutes; `db` healthcheck `start_period` is 300s), and the `db` container gets renamed to `<container>-<dump-number>` (e.g. `qafiyah-db-0019`) so `docker ps` shows which dump it's running. Later boots reuse the volume, so the name doesn't change again until the next real restore. Wipe and re-seed locally with `bun run db:reset`.

Shipping a new Postgres/ES dump to production is an ordered action (`bun run db:reseed`), see `.claude/skills/deploy/SKILL.md`. A major-version bump of `postgres`/`elasticsearch` needs a volume wipe, see `docs/deployment/troubleshooting.md`.

## Secrets

Production secrets live encrypted in `secrets/prod.enc.env` and reach the VPS as a generated `/opt/qafiyah/.env` (mode `600`, gitignored). Change them with `bun run secrets:edit prod`, never on the box and never by pasting values into a doc. See `docs/deployment/secrets.md`.

## Rate limiting and API keys

The API serves **identical response bodies to every caller**. There is no capped
data, no scope, and no `Vary: x-api-key`. The only thing that varies is how many
requests per hour a caller gets, carried in each response's `X-RateLimit-*`
headers. That is why JSON reads are `Cache-Control: private, max-age=300`: the
caller's own browser may reuse a response for five minutes, but no shared cache
(Cloudflare, nginx) may store one, since it would hand one caller's counters to
the next and let cached hits skip the count.
The website's `/api/v1/search` proxy drops those headers, so every visitor gets
the same response; it sets its own `public` policy on successful searches, and
the website's nginx caches them for five minutes.

Anonymous callers share a per-IP hourly bucket. An IPv6 caller is bucketed by
its /64, the block one subscriber is usually handed, so rotating addresses
inside it earns no extra allowance. Keyed callers get their own bucket and
their own number. Exceeding either returns `429` as
`application/problem+json` with `Retry-After`, and every response carries
`x-ratelimit-limit`, `x-ratelimit-remaining`, and `x-ratelimit-reset`.

### Environment keys (bypass the limiter)

Two values, each generated with `openssl rand -hex 32`, live in the API's
environment and are checked before the accounts database is consulted:

- `API_KEY_INTERNAL` (secret): unlimited, and the only key that opens
  `/account`. The web SSR server and the same-origin search proxy present it.
- `API_KEY_FULL` (secret): unlimited on `/v1` only, for trusted server clients,
  smoke tests, and admin tooling. It does **not** open `/account`, so a leak
  costs corpus reads and never account administration.

They deliberately do not touch `qafiyah_accounts`, so an accounts-database
outage degrades the developer portal and never the website.

**The web service's `INTERNAL_API_KEY` is `API_KEY_INTERNAL`.** Compose (and
`bun run dev`) feed it from that one variable, so the two cannot drift; never
add `INTERNAL_API_KEY` to a secrets file (`secrets:check` rejects it). It is
server-only, read at SSR time, and never reaches the
browser. No key of any kind ships in the browser bundle: the browser calls
`/api/v1/search` and `/api/v1/poems/random` on its own origin, and
`apps/web/src/lib/api/proxy-allowlist.ts` refuses every other path.

### The anonymous limit

`ANON_REQUESTS` sets the per-address bucket for unkeyed callers, over
`WINDOW_SECONDS` (one hour). **Leave it unset in production.** Unset, the API
picks 60 when `ENVIRONMENT=production` and an effectively unlimited value
everywhere else (`apps/api/src/config.rs::anon_requests`), because outside
production there is no Cloudflare in front and every caller resolves to the same
bucket.

### Plans

Keyed callers get their ceilings from the `plans` table in `qafiyah_accounts`,
not from the environment. Three limits apply and all must pass: the plan's
`requests` per `WINDOW_SECONDS` bucketed on the user, the plan's `burst` per
`BURST_WINDOW_SECONDS` bucketed on the user, and the plan's `ip_ceiling`
bucketed on the client address. Only plans carrying an `ip_ceiling` are address
bucketed, and only `free` does, so a paying customer is never throttled for
sharing an office address with free accounts.

Change a plan with `update plans set requests = 1000 where slug = 'free';` and
move a user with `update users set plan = 'premium' where email = '...';`.
Either takes up to 60 seconds to apply, because `accounts/cache.rs` holds
resolved keys for `API_KEY_CACHE_TTL_SECONDS`.

This is why `ENVIRONMENT` matters more than it looks: if it is ever not exactly
`production`, the limiter silently becomes a no-op. `secrets:check` requires it
to be exactly `production` in `secrets/prod.enc.env`. An empty internal key
would drop the site's own SSR into the 60/hour anonymous bucket, so in
production the API refuses to start without `API_KEY_INTERNAL` and
`API_KEY_FULL`, and the web container refuses to start without
`INTERNAL_API_KEY` and `SESSION_STATE_SECRET`.

Client IP comes from `X-Forwarded-For` (the Cloudflare tunnel does not send
`CF-Connecting-IP`), which the web container's nginx resolves and re-sends as
`CF-Connecting-IP` over the dedicated `backend` network. The API honors that
header only from the `backend` subnet (`client_ip.rs`), so a lateral container on
the default bridge is bucketed on its own address rather than a spoofed header.

### Issued keys

Everything else is a row in the `qafiyah_accounts` database: a SHA-256 hash of
the key and a `prefix` for display. The key carries no allowance of its own; the
ceilings come from the owner's plan, so raising one developer's limit means
moving them to another plan rather than editing their key.

Issue a key with:

```bash
DATABASE_URL_ACCOUNTS=... cargo run -p qafiyah-api --bin issue-key -- <email> [label]
```

The raw key is printed once and never recoverable; only its hash is stored.

Two consistency windows are deliberate and worth knowing before debugging
either: a revoked key keeps working for up to 60 seconds
(`API_KEY_CACHE_TTL_SECONDS`, no cross-process invalidation), and `usage_hourly`
lags by up to a minute because counters flush in batches rather than on the
request path.

### OAuth for the account portal

Sign-in lives on `qafiyah.com`, not on the API, so these five belong to the
**web** service:

```
OAUTH_GOOGLE_CLIENT_ID
OAUTH_GOOGLE_CLIENT_SECRET
OAUTH_GITHUB_CLIENT_ID
OAUTH_GITHUB_CLIENT_SECRET
SESSION_STATE_SECRET
```

Register these exact redirect URIs with each provider, or the callback is
rejected before it reaches us:

```
https://qafiyah.com/auth/callback/google
https://qafiyah.com/auth/callback/github
```

Scopes are fixed in `apps/web/src/lib/server/oauth/providers.ts`:
`openid email profile` for Google, `read:user user:email` for GitHub. GitHub's
`user:email` is not optional; without it `/user/emails` returns 403 and the
verified-email check that prevents account takeover cannot run.

`SESSION_STATE_SECRET` is `openssl rand -hex 32`. It protects only the
short-lived OAuth state cookie, so rotating it invalidates in-flight logins and
nothing else. Sessions themselves are rows in `qafiyah_accounts.sessions` and
are unaffected.

The web service's `INTERNAL_API_KEY` comes from `API_KEY_INTERNAL`. Beyond SSR, the account portal now depends on it: `account-client.ts`
presents it to reach `/account/*`, and if it is empty every visitor appears
signed out no matter what.

For local development these, plus `INTERNAL_API_KEY`, must also be listed in `turbo.json` under
`dev.passThroughEnv`, or Turborepo withholds them from `astro dev` and the
portal behaves as if no provider were configured.

## Accounts database on an existing production volume (one-time)

The accounts work shipped with everything the code needs and nothing the host
needs. These steps are **manual, once, on the VPS**, and the deploy will fail
without the first two.

1. **Set `PG_ACCOUNTS_PASSWORD` in `secrets/prod.enc.env`.** Generate with
   `openssl rand -hex 32`. Both the `db` and `api` services guard it with `:?`,
   so `docker compose` refuses to start without it. It must not equal
   `PG_READER_PASSWORD`.

2. **Create the database and role by hand.** `scripts/db/accounts-init.sh` is
   mounted into the Postgres initdb directory, which only runs on an **empty**
   data volume. Production's volume is not empty, so the script will never fire
   there. Run its SQL once against the live container:

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

   The API runs `sqlx::migrate!` against that database at startup, so the tables
   create themselves on the first boot after this. Verify with
   `docker compose exec db psql -U "$POSTGRES_USER" -d qafiyah_accounts -c '\dt'`.

3. **Create a private R2 bucket for backups and a token scoped to it.** Not
   `qafiyah-assets`, which is served publicly at `cdn.qafiyah.com`. The new
   bucket gets no public binding and no custom domain. Add a lifecycle rule that
   deletes `accounts/` objects after 30 days. Then create an R2 API token with
   object read and write on that bucket only, not the deploy token, so a
   compromised VPS cannot reach the public asset bucket. It gives an access key
   ID, a secret access key, and the S3 endpoint.

4. **Set the `ACCOUNTS_BACKUP_*` values in `secrets/prod.enc.env`, then install
   the timer.** See the accounts backup section in `docs/deployment/services.md`
   for the variables, the two install commands, how to restore, and what a lost
   volume actually costs.

A quick check that the whole path works, after deploying:

```bash
curl -si https://api.qafiyah.com/v1/meters | grep -i x-ratelimit
```

Expect `x-ratelimit-limit: 60`. If it shows something enormous, `ENVIRONMENT` is
not `production`.
