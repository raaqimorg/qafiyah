# Per-Service Deployment Reference

## API (`apps/api`)

Rust + axum (`apps/api/Dockerfile`): a static musl binary in a ~15MB alpine image running **non-root** (`api` user) with `tini` as PID 1. Its environment is the `api` service block in `docker-compose.yml`; `apps/api/src/config.rs` is what reads it and what refuses to start in production without the two `API_KEY_*` values. Healthcheck hits `GET /healthz` (a 200 that never touches the DB and sits outside the `/v1` rate limiter, so the probe never throttles itself). Reached by the web container over the dedicated `backend` network (`api-backend:8787`), which is also the only source the API trusts for `CF-Connecting-IP`.

`ENVIRONMENT` decides how much it logs: outside production every request emits a structured line, in production the line is sampled at 5% unless the request errored, took over two seconds, or found nothing.

Server errors are reported to Sentry, tagged with the contract code, the method and the path, and with the panic hook installed so a handler that dies without reaching the error path is reported too. Only 5xx: a 404 for a slug that does not exist is the API working, and reporting those would bury what matters. Reporting needs `SENTRY_DSN` **and** `ENVIRONMENT=production`, so a developer holding production credentials cannot fill the project from a laptop.

There is no performance tracing; the per-request log line (with `duration_ms`, always kept for a slow request) covers that ground.

**Checking a deploy.** `bun run api:conformance` (or `api:conformance prod`) replays every documented operation against a running API and validates each response body against the committed OpenAPI schema. This is the post-deploy verification step in `.claude/skills/deploy/SKILL.md`.

## Postgres connections

Postgres allows 100 connections, 3 of them reserved for superusers. Every client caps its own:

- The API holds at most 20 connections to the corpus and 5 to `qafiyah_accounts` (`PG_POOL_MAX_CONNECTIONS`, `PG_ACCOUNTS_POOL_MAX_CONNECTIONS`). The pool opens them on demand and reuses them, runs `SELECT 1` before handing one out and replaces it if that fails, and makes a request wait at most 2 s for a free one before answering 500. Each connection sets `statement_timeout = 5s` (and `lock_timeout = 2s` for accounts) when it opens.
- The search indexer holds 1 while it reindexes, and `pg_dump` (backups, dump scripts) 1 each while it runs.

The worst case is about 30, so no external pooler is needed while one API process is the only long-lived client. When a client process exits, its sockets close and Postgres ends its sessions at once. For what a client leaves behind, the `db` command in `docker-compose.yml` sets:

- `idle_in_transaction_session_timeout=30s`: a transaction left open between statements (a forgotten one in TablePlus, say) is ended after 30 s instead of holding its locks. The app's transactions last milliseconds, and `pg_dump` and restores turn it off for their own sessions.
- `tcp_keepalives_idle=60`, `tcp_keepalives_interval=10`, `tcp_keepalives_count=3`: a client that vanished without closing its socket is noticed within 90 s, not the kernel's 2 hours.
- `client_connection_check_interval=10s`: a running query whose client disconnected is cancelled within 10 s.

`idle_session_timeout` stays off, because it would close the API's pooled connections in quiet periods. To see who holds connections, run against production:

```sql
SELECT usename, state, count(*)
FROM pg_stat_activity
WHERE backend_type = 'client backend'
GROUP BY usename, state
ORDER BY count(*) DESC;
```

## Web (`apps/web`)

Astro SSR: every route renders on demand by calling the internal `api` container over the contract (`INTERNAL_API_URL=http://api-backend:8787`, via `openapi-fetch` bound to the generated schema types); bundled nginx caches the rendered HTML and serves built static assets. `PUBLIC_API_URL` is unset, so browser islands fall back to the production API. The build runs `astro build` only (no `DATABASE_URL` at build time). The serve image is **fully non-root**: nginx and the Bun SSR origin both run as `nginx` under `tini`. nginx listens on `8080` and the container has no host port; only `edge-gateway` binds `127.0.0.1:80` and proxies to `web-edge:8080`. The entrypoint runs both processes and exits if either dies, so `restart: unless-stopped` recovers a crash. Build alone with `docker compose build web`.

Browser and server errors go to Sentry only from builds that carry a release: `bun run deploy` builds with `SENTRY_RELEASE` set to the commit, and the Dockerfile passes it to Astro as `PUBLIC_SENTRY_RELEASE`, which switches reporting on. Dev servers and local Docker builds have no release, so they never report to the production project.

Sentry's one uptime monitor (the plan includes one) requests `https://qafiyah.com/api/v1/poems/random?option=slug` every minute from several regions. The response is `no-store`, so every check passes Cloudflare, the tunnel, the edge gateway, the web nginx and its API proxy, the API and Postgres, and the proxy answers 502 when the API is down. Three failures in a row open an uptime issue in the `javascript-astro` project. The homepage is the wrong target: Cloudflare and nginx keep serving it from cache while the origin is down.

### Caching & freshness

Each route sets `Cache-Control` from `apps/web/src/lib/server/cache.ts`: HTML is held one day at nginx (`s-maxage`) but only one minute in the browser (a deploy wipes nginx's cache, it cannot wipe a browser's), sitemaps five minutes, well-known files one day, and 404 and authenticated pages `no-store`. nginx (`proxy_cache`) honors it, collapses concurrent misses (`proxy_cache_lock`), and serves stale on upstream errors or during background refresh. New/edited poems appear within the TTL, no rebuild. nginx also canonicalizes URLs to the https apex (www→apex + trailing slashes), sets baseline security headers, gzips text, and serves `/_astro/` immutably.

Cloudflare caches pages too, through a Cache Rule set in the dashboard (host `qafiyah.com`, path not starting with `/api/`, eligible for cache, Edge TTL from the origin's `Cache-Control`, bypassed when there is none), so `no-store` pages are never stored there. Cached pages reference the build's hashed `/_astro/` scripts, which the next deploy removes, so `bun run deploy` purges the whole Cloudflare cache once the new containers pass the edge check, using `CLOUDFLARE_ZONE_ID` and `CLOUDFLARE_CACHE_PURGE_TOKEN` from the prod secrets (a token that can only purge `qafiyah.com`'s cache). If the purge fails, the deploy exits non-zero with Cloudflare's response; purge everything in the dashboard by hand before anything else.

The zone's Managed Transforms (dashboard, Rules > Settings) are set for this origin: **Remove visitor IP headers** is off, so `CF-Connecting-IP` reaches the edge gateway (see the real-IP bullet below); **Add security headers** is off, because it overwrote the origin's `X-Frame-Options: DENY` and `Referrer-Policy: strict-origin-when-cross-origin` with weaker values and added the deprecated `X-XSS-Protection` and `Expect-CT`; **Add TLS client auth headers** is off, since nothing uses mTLS; **Remove "X-Powered-By" headers** is on. **Add visitor location headers** is on, though nothing in the code reads the `cf-ip*` headers it adds.

### Sitemap

`/sitemap-index.xml` is generated on demand (poems sharded per `SITEMAP_POEMS_PER_SHARD` from `config.ts`, plus poets and collection landing pages; a poet with no poems keeps their page but is left out, since `GET /v1/poets/slugs` lists only poets whose `poet_stats` count is above zero) and cached like any route. `pages/robots.txt.ts` (rendered from `well-known/robots.web.txt`) references it.

### nginx & TLS

Full config at `apps/web/nginx.conf`, baked into the image at `/etc/nginx/nginx.conf`. It proxies HTML to the Astro origin on `127.0.0.1:4321`, serves `/app/apps/web/dist/client` from disk, and answers a container-local `/healthz`. TLS terminates at Cloudflare, so the config is plain HTTP by design, and `$scheme` is always `http`:

- The www→apex redirect is its own `server` block hardcoding `https://qafiyah.com`.
- Same-host (trailing-slash) redirects stay relative (`absolute_redirect off`) so the browser keeps https, never an `http://` Location or a leaked `:8080`.
- Baseline security headers (`X-Content-Type-Options: nosniff`, `Referrer-Policy: strict-origin-when-cross-origin`, `X-Frame-Options: DENY`) live in `apps/web/nginx-security-headers.conf`, and reach browsers unchanged only while Cloudflare's **Add security headers** Managed Transform stays off (above), included at server scope and re-included in every `location` that sets its own headers (nginx drops inherited `add_header`).
- The Content-Security-Policy is host-specific and split across `apps/web/nginx-csp.conf` (qafiyah.com and www) and `apps/web/nginx-csp-api.conf` (api.qafiyah.com, which alone allows cdn.jsdelivr.net in script-src for the Scalar docs UI). The qafiyah.com `connect-src` also lists the GitHub and Google avatar hosts (`avatars.githubusercontent.com`, `lh3.googleusercontent.com`), because the nav account control fetches the signed-in avatar to cache it; a new sign-in provider needs its avatar host added there.
- Real visitor IP is restored from `X-Forwarded-For`. Cloudflare appends the visitor as its last entry, after anything the client sent, and also sends `CF-Connecting-IP` with the same address (verified on the wire on 2026-09-28: they matched on every request, and a client-sent `CF-Connecting-IP` is refused with a 403). The zone's **Remove visitor IP headers** Managed Transform (dashboard, Rules > Settings) is off; turned on, it strips `CF-Connecting-IP` and leaves the visitor as the only `X-Forwarded-For` entry, which the rule below also resolves. The web nginx trusts that header only from the dedicated `edge` network (`172.28.0.0/24` in prod, `172.29.0.0/24` in dev), which carries only the edge-gateway, and takes the rightmost address it does not trust (`real_ip_recursive on`), so entries a client adds on the left are ignored. A peer on the default bridge cannot spoof it. After any change to the proxy chain, confirm `docker compose logs web` shows real client IPs, not the `172.28.x`/`172.29.x` gateway: every per-address limit (the web's `limit_req`, and the API's anonymous, IP-ceiling, and website-visitor buckets) keys on it. nginx passes it to Astro as `X-Real-IP`, overwriting any client value, and the search proxy forwards that to the API.

## Telemetry proxy (`apps/telemetry-proxy`, Cloudflare Worker)

A first-party Worker on `t.qafiyah.com` forwarding browser Sentry envelopes, so telemetry survives tracker-blockers and never touches the VPS or the WAF. Routes, CORS, and the deploy command: `apps/telemetry-proxy/AGENTS.md`. It runs on Cloudflare's edge and is **not** part of `docker compose`; its deploy is step 7 of `.claude/skills/deploy/SKILL.md`, independent of the VPS deploy. Do not confuse it with `apps/search-indexer`, the Postgres-to-Elasticsearch job that does run on the box.

## Web Application Firewall (`edge-gateway`)

The edge gateway is the [OWASP ModSecurity CRS image](https://github.com/coreruleset/modsecurity-crs-docker) (`owasp/modsecurity-crs:*-nginx-alpine`, pinned by exact CRS+date tag in `docker-compose.yml`): ModSecurity v3 running the **OWASP Core Rule Set** (signatures for SQLi, XSS, RCE, scanners, LFI/RFI, etc.). It runs as a non-root reverse proxy on `127.0.0.1:80`, inspects every request, and forwards to `web-edge:8080` preserving the original `Host`. Configured entirely by env vars (none baked in), in `docker-compose.yml`:

- `BACKEND=http://web-edge:8080` (the web service's alias on the dedicated `edge` network), `PORT=8080`, `SERVER_NAME=qafiyah.com`.
- `NGINX_ALWAYS_TLS_REDIRECT=off`: TLS terminates at Cloudflare; never self-redirect to https.
- `REAL_IP_HEADER=CF-Connecting-IP` + `SET_REAL_IP_FROM=172.28.0.1` (dev: `172.29.0.1`): restores the real client IP for CRS scoring. The single address is the `edge` network gateway, which is how the host's `cloudflared` (through docker-proxy on the published `127.0.0.1:80`) appears to the container. Cloudflare sends that header, so the WAF scores each request against the real visitor (verified 2026-09-28: the address it appends to `X-Forwarded-For` for the web nginx is the visitor's). Its access log still shows `172.28.0.1`, since the image's `main` log format prints `$realip_remote_addr`, the connecting peer. If the Remove visitor IP headers Managed Transform is turned on, the header disappears and the WAF falls back to scoring everything as `172.28.0.1`. The web nginx does not depend on it, since it reads `X-Forwarded-For` itself.
- `BLOCKING_PARANOIA=1` (paranoia level), `ANOMALY_INBOUND=5` (block threshold).

### Backend proxy override (`apps/edge-gateway/proxy_backend.conf.template`)

`apps/edge-gateway/proxy_backend.conf.template` is the image's own template, with a variable `proxy_pass` so nginx re-resolves `web-edge:8080` per request instead of pinning an IP; that is what lets the zero-downtime web roll in `docs/deployment/architecture.md` work. The file, the reason, and the refresh procedure for an image bump: `apps/edge-gateway/AGENTS.md`.

### Rollout: DetectionOnly → On

CRS can false-positive on **Arabic search input** (unusual encodings/punctuation can trip SQLi/XSS rules), so the WAF ships in **`MODSEC_RULE_ENGINE=DetectionOnly`**: it evaluates rules and **logs** would-be blocks but lets every request through. Run this way against real traffic first, then enforce. The ordered flip-to-`On` steps and the bypass/rollback are in `.claude/skills/deploy/SKILL.md`.

> **Pre-flight validation (2026-06-18): enabling `On` is considered safe.** A [Schemathesis](https://schemathesis.readthedocs.io) schema-fuzz of every operation in the OpenAPI document ran through the full edge chain (`edge-gateway` → web nginx → `api`) with `MODSEC_RULE_ENGINE=On` at production settings (`BLOCKING_PARANOIA=1`, `ANOMALY_INBOUND=5`). Of 1737 cases, CRS blocked exactly **2**, both via rule `949110` (inbound anomaly score 10 ≥ 5) on pathological fuzzer payloads carrying NUL bytes and control characters in `/v1/poems` query params. Crucially, **legitimate Arabic `/v1/search` queries were not blocked** (they reach the API and hit normal input validation), clearing the headline false-positive concern. Synthetic fuzzing can't perfectly model real traffic, so still tail `docker compose logs -f edge-gateway` for the first day after flipping, but the green light stands.

## Elasticsearch & indexer (`apps/search-indexer`, Rust)

Search is served from Elasticsearch, populated from Postgres by `apps/search-indexer`, both in the same Compose stack, loopback-only. The indexer is a one-shot init job that `api` waits on; what it does and when it reindexes: `apps/search-indexer/AGENTS.md`. Fix drift on a populated index with `bun run reindex:prod` (full rebuild + alias swap; bare `bun run reindex` targets dev). The ordered command is in `.claude/skills/deploy/SKILL.md`.

### Accounts database backups

`qafiyah_accounts` is a separate database in the same Postgres container, holding
user records and hashed API keys. It is deliberately outside the `data/db/`
pipeline, which publishes the corpus to anyone who requests a passphrase, so it
has its own backup path:

- `scripts/accounts/backup.sh` runs daily at 03:17 UTC from a systemd timer on the
  VPS. It runs `pg_dump` inside the `db` container, encrypts the dump with `age`
  to `ACCOUNTS_BACKUP_RECIPIENT`, and uploads it with rclone (run from its image)
  to a private R2 bucket under `accounts/`. A failed dump uploads nothing.
- The recipient is an age public key whose private half never touches the VPS (a
  maintainer's, never the VPS's own sops key), so the VPS can write backups but
  cannot read them.
- Retention is an R2 lifecycle rule on the bucket (delete `accounts/` objects
  after 30 days), not the script.
- The bucket is **not** `qafiyah-assets`. That one is served publicly at
  `cdn.qafiyah.com`. The backup bucket has no public binding and no custom domain.
- It uploads with an R2 API token scoped to object read and write on the backup
  bucket only, not the deploy token and not one that can reach `qafiyah-assets`.
  That scoping is what keeps a compromised VPS from reaching the public asset
  bucket.
- Required environment: `ACCOUNTS_BACKUP_BUCKET`, `ACCOUNTS_BACKUP_RECIPIENT`,
  `ACCOUNTS_BACKUP_R2_ENDPOINT` (`https://<account-id>.r2.cloudflarestorage.com`),
  `ACCOUNTS_BACKUP_R2_ACCESS_KEY_ID`, `ACCOUNTS_BACKUP_R2_SECRET_ACCESS_KEY`, and
  `age` on the host (installed with sops, see `docs/deployment/secrets.md`).

Install the timer once, as root on the VPS:

```bash
cp /opt/qafiyah/scripts/accounts/qafiyah-accounts-backup.{service,timer} /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now qafiyah-accounts-backup.timer
```

Check it with `systemctl list-timers qafiyah-accounts-backup.timer`, run one now
with `systemctl start qafiyah-accounts-backup.service`, and read its output with
`journalctl -u qafiyah-accounts-backup.service`. A failed run shows in
`systemctl --failed`.

To restore, download the backup (Cloudflare dashboard, or `rclone copyto`), then
decrypt it on the machine that holds the private key and stream it to the VPS:

```bash
age -d -i ~/.config/sops/age/keys.txt qafiyah_accounts_<stamp>.dump.age \
  | ssh qafiyah 'cd /opt/qafiyah && docker compose exec -T db pg_restore -U qafiyah_accounts -d qafiyah_accounts --clean --if-exists --no-owner'
```

What a lost volume actually costs: `users` and `identities` are recreated the
moment someone signs in again, `sessions` just forces a re-login, and
`usage_hourly` is cosmetic. Only `api_keys` hurts, and it degrades gracefully:
qafiyah.com keeps working throughout, because the site authenticates with
`INTERNAL_API_KEY` from the environment and never reads the accounts database.
Only third-party integrations break, until their owners regenerate.
