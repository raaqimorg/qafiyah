# Per-Service Deployment Reference

## API (`apps/api`)

The API is Rust and axum (`apps/api/Dockerfile`). It is a static musl binary in an alpine image of about 15 MB. It runs as a **non-root** user (`api`), with `tini` as PID 1.

- Its environment is the `api` service block in `docker-compose.yml`. `apps/api/src/config.rs` reads it, and refuses to start in production without the two `API_KEY_*` values.
- The healthcheck calls `GET /healthz`. This is a 200 that never touches the database and is outside the `/v1` rate limiter, so the probe never limits itself.
- The web container reaches the API over the dedicated `backend` network, as `api-backend:8787`. This network is also the only source that the API trusts for `CF-Connecting-IP`.

**Logs.** The API logs JSON through `tracing`, one line for each event. Each line carries its request span: the method, the route, the path, `x-request-id`, and the fields that the handler recorded.

- A request ends with a `request` line. Its level is `error` for a 5xx, `warn` for a request over two seconds, and `debug` for the rest. A search or a list that found nothing adds an `info` line.
- With `ENVIRONMENT=production`, the filter is `warn,qafiyah_api=info`. So production keeps every error, slow request, failure, and empty result, and nothing else. In other environments, the filter is `warn,qafiyah_api=debug`. `RUST_LOG` overrides either one.
- `/healthz`, `/v1/openapi.json`, and `/v1/docs` are never logged.
- Every response carries `x-request-id`: the caller's value, or a new UUID.
- Docker's log files and Loki (`apps/observability`) keep the lines.

**Errors.** The API reports server errors to Sentry, with tags for the contract code, the method, and the path. A panic hook is installed, so a handler that dies before the error path is also reported.

- It reports only 5xx. A 404 for a slug that does not exist means that the API works, and reports of those would hide the important ones.
- Reporting needs `SENTRY_DSN` **and** `ENVIRONMENT=production`. So a developer who holds production credentials cannot fill the project from a laptop.

The metrics in `apps/observability` hold the latency and the request count of every request, for each route, with percentiles. The log line adds the details of a slow or failed request.

**Checking a deploy.** `bun run api:conformance` (or `api:conformance prod`) runs Schemathesis in Docker against a running API. It sends every documented operation its documented examples. Each one must answer 2xx, with a status, content type, headers, and body that match the committed OpenAPI document. This is the check after a deploy in `.claude/skills/deploy/SKILL.md`.

## Postgres connections

Postgres allows 100 connections, and keeps 3 of them for superusers. Every client sets its own limit:

- The API holds at most 20 connections to the corpus and 5 to `qafiyah_accounts` (`PG_POOL_MAX_CONNECTIONS`, `PG_ACCOUNTS_POOL_MAX_CONNECTIONS`).
  - The pool opens connections when it needs them, and uses them again.
  - It runs `SELECT 1` before it gives out a connection, and replaces the connection if that fails.
  - A request waits at most 2 seconds for a free connection. After that, it gets 503 with `Retry-After: 2`.
  - A whole request gets 6 seconds (`REQUEST_DEADLINE_SECONDS`). After that, it gets the same 503.
  - Each connection sets `statement_timeout = 5s` when it opens. The accounts connections also set `lock_timeout = 2s`.
- The search indexer holds 1 connection while it reindexes. `pg_dump` (for backups and dump scripts) holds 1 while it runs.

The worst case is about 30 connections. So no external pooler is necessary while one API process is the only long-lived client. When a client process stops, its sockets close, and Postgres ends its sessions at once.

For what a client leaves behind, the `db` command in `docker-compose.yml` sets these values:

- `idle_in_transaction_session_timeout=30s`: Postgres ends a transaction that stays open between statements after 30 seconds, so it does not keep its locks. An example is a forgotten transaction in TablePlus. The app's transactions last milliseconds. `pg_dump` and restores turn the setting off for their own sessions.
- `tcp_keepalives_idle=60`, `tcp_keepalives_interval=10`, `tcp_keepalives_count=3`: Postgres finds a client that disappeared without closing its socket within 90 seconds, not after the kernel's 2 hours.
- `client_connection_check_interval=10s`: Postgres cancels a running query within 10 seconds after its client disconnects.
- `max_parallel_workers_per_gather=0`: no parallel query plans. The container has one CPU (`cpus: 1.0`), so a leader and its workers would share it. A parallel hash aggregate over `poem_verses` ran for minutes, and the same query alone takes less than 2 seconds (#205).

`idle_session_timeout` stays off, because it would close the API's pooled connections in quiet periods. To see who holds connections, run this against production:

```sql
SELECT usename, state, count(*)
FROM pg_stat_activity
WHERE backend_type = 'client backend'
GROUP BY usename, state
ORDER BY count(*) DESC;
```

## Web (`apps/web`)

The web app is Astro with server-side rendering. Every route renders when it is requested, by calling the internal `api` container over the contract. It uses `INTERNAL_API_URL=http://api-backend:8787`, through `openapi-fetch` with the generated schema types. A bundled nginx caches the rendered HTML, and serves the built static assets.

- `PUBLIC_API_URL` is not set, so the browser islands use the production API.
- The build runs only `astro build`. It needs no `DATABASE_URL`.
- The serve image is **fully non-root**: nginx and the Bun server both run as `nginx`, under `tini`.
- nginx listens on `8080`, and the container has no host port. Only `edge-gateway` binds `127.0.0.1:80`, and it proxies to `web-edge:8080`.
- The entrypoint runs both processes, and stops if either one dies. So `restart: unless-stopped` recovers a crash.
- To build only this image, run `docker compose build web`.

Browser and server errors go to Sentry only from builds that carry a release. `bun run deploy` builds with `SENTRY_RELEASE` set to the commit. The Dockerfile gives it to Astro as `PUBLIC_SENTRY_RELEASE`, which turns reporting on. Dev servers and local Docker builds have no release, so they never report to the production project.

Sentry has one uptime monitor, which is all that the plan includes. It requests `https://qafiyah.com/api/v1/poems/random?option=slug` every minute from several regions.

- The response is `no-store`. So every check goes through Cloudflare, the tunnel, the edge gateway, the web nginx and its API proxy, the API, and Postgres.
- The proxy answers 502 when the API is down.
- Three failures in a row open an uptime issue in the `javascript-astro` project.
- The home page is the wrong target, because Cloudflare and nginx keep serving it from cache while the origin is down.

### Caching and freshness

Each route sets `Cache-Control` from `apps/web/src/lib/server/cache.ts`:

- HTML: one day at nginx (`s-maxage`), but only one minute in the browser. A deploy clears the nginx cache, but it cannot clear a browser's cache.
- Sitemaps: five minutes.
- Well-known files: one day.
- 404 pages and pages for signed-in users: `no-store`.

nginx (`proxy_cache`) follows these values. It combines concurrent misses into one request (`proxy_cache_lock`). It serves a stale copy on upstream errors and during a background refresh. New or edited poems appear within the cache time, with no rebuild.

nginx also does these things:

- It sends every URL to its canonical form on the https apex: www goes to the apex, and trailing slashes are removed.
- It sets the baseline security headers.
- It compresses text with gzip.
- It serves `/_astro/` as immutable.

**The nginx cache key** keeps the query string only where the page reads one:

- the API proxy under `/api/`
- the taxonomy term pages (`/collections`, `/meters`, `/rhymes`, and `/themes`, which read `page`)
- `/poets` (`page`, `era`, `q`)
- `/poets/{slug}` (`page`, `meter`, `rhyme`, `theme`)

Every other page uses only its path as the key. This includes the home page, whose search state is only in the browser. So a stray query string gets the one cached entry, and never forces a render.

The pages that read a query string accept only one spelling of it. They answer every other spelling with a 301 to their canonical URL, before they call the API. Examples are an unknown parameter, `page=02`, or filters out of order or repeated. `poetUrl` writes poet filters sorted and without repeats. So each page has one URL.

A poet name search on `/poets?q=` is the one input with an unlimited number of values. So while `q` is present, nginx limits it for each visitor: 60 a minute with a burst of 30, then the 429 page. The stack smoke run checks all three behaviors (`scripts/smoke/probes/page-cache.ts` and the caching suite).

**Cloudflare caches pages too**, through a Cache Rule in the dashboard:

- It applies to the host `qafiyah.com`, on paths that do not start with `/api/`.
- It makes them eligible for cache, and takes the Edge TTL from the origin's `Cache-Control`. When there is no `Cache-Control`, it does not cache.
- So `no-store` pages are never stored there.

Cached pages refer to the build's hashed `/_astro/` scripts, and the next deploy removes those scripts. So `bun run deploy` purges the whole Cloudflare cache after the new containers pass the edge check. It uses `CLOUDFLARE_ZONE_ID` and `CLOUDFLARE_CACHE_PURGE_TOKEN` from the production secrets. That token can only purge the cache of `qafiyah.com`. If the purge fails, the deploy exits with an error and shows Cloudflare's response. Then purge everything in the dashboard by hand, before you do anything else.

**The zone's Managed Transforms** (dashboard, Rules > Settings) are set for this origin:

- **Remove visitor IP headers** is off, so `CF-Connecting-IP` reaches the edge gateway (see the real IP item in "nginx and TLS" below).
- **Add security headers** is off. It replaced the origin's `X-Frame-Options: DENY` and `Referrer-Policy: strict-origin-when-cross-origin` with weaker values. It also added the deprecated `X-XSS-Protection` and `Expect-CT`.
- **Add TLS client auth headers** is off, because nothing uses mTLS.
- **Remove "X-Powered-By" headers** is on.
- **Add visitor location headers** is on, but nothing in the code reads the `cf-ip*` headers that it adds.

### Sitemap

`/sitemap-index.xml` is generated when it is requested, and cached like any route. It lists these things:

- the poems, in shards of `SITEMAP_POEMS_PER_SHARD` (from `config.ts`)
- the poets. A poet with no poems keeps their page, but the sitemap leaves them out, because `GET /v1/poets/slugs` lists only poets whose `poet_stats` count is above zero.
- the collection landing pages

`pages/robots.txt.ts` (rendered from `well-known/robots.web.txt`) refers to it.

### nginx and TLS

The full configuration is `apps/web/nginx.conf`. The image has it at `/etc/nginx/nginx.conf`. nginx proxies HTML to the Astro server on `127.0.0.1:4321`, serves `/app/apps/web/dist/client` from disk, and answers a `/healthz` inside the container.

TLS ends at Cloudflare. So the configuration is plain HTTP by design, and `$scheme` is always `http`:

- The redirect from www to the apex is its own `server` block, with `https://qafiyah.com` written into it.
- Redirects on the same host (trailing slashes) stay relative (`absolute_redirect off`). So the browser keeps https, and never sees an `http://` Location or the internal port `:8080`.
- The baseline security headers are `X-Content-Type-Options: nosniff`, `Referrer-Policy: strict-origin-when-cross-origin`, and `X-Frame-Options: DENY`.
  - They are in `apps/web/nginx-security-headers.conf`. They reach browsers unchanged only while Cloudflare's **Add security headers** Managed Transform stays off (above).
  - The file is included at server scope. It is included again in every `location` that sets its own headers, because nginx then drops the inherited `add_header`.
- The Content-Security-Policy is different for each host.
  - `apps/web/nginx-csp.conf` is for qafiyah.com and www. Its `connect-src` also lists the GitHub and Google avatar hosts (`avatars.githubusercontent.com`, `lh3.googleusercontent.com`). The account control in the navigation fetches the avatar of the signed-in user, to cache it. A new sign-in provider needs its avatar host there.
  - `apps/web/nginx-csp-api.conf` is for api.qafiyah.com. Only this policy allows cdn.jsdelivr.net in `script-src`, for the Scalar docs UI.
- The real visitor address comes from `X-Forwarded-For`.
  - Cloudflare adds the visitor as its last entry, after anything that the client sent. It also sends `CF-Connecting-IP` with the same address. This was verified on the wire on 2026-09-28: the two matched on every request, and Cloudflare refuses a client-sent `CF-Connecting-IP` with a 403.
  - The zone's **Remove visitor IP headers** Managed Transform is off. If it is turned on, it removes `CF-Connecting-IP`, and leaves the visitor as the only `X-Forwarded-For` entry. The rule below also handles that case.
  - The web nginx trusts that header only from the dedicated `edge` network (`172.28.0.0/24` in production, `172.29.0.0/24` in dev). Only the edge gateway is on that network.
  - It takes the rightmost address that it does not trust (`real_ip_recursive on`). So it ignores entries that a client adds on the left, and a peer on the default bridge cannot forge the address.
  - Every limit for each address uses this address: the web's `limit_req`, and the API's anonymous, IP-ceiling, and website-visitor buckets. So after any change to the proxy chain, make sure that `docker compose logs web` shows real client addresses, not the `172.28.x` or `172.29.x` gateway.
  - nginx gives the address to Astro as `X-Real-IP`, and overwrites any value from the client. The search proxy sends it on to the API.
- The access log is one JSON line for each request (`log_format json`, `"source":"nginx"`). It has the visitor address, host, method, URI, status, bytes, `request_time`, `upstream_time`, and the page cache status. Alloy ships it to Loki, and the Edge dashboard reads the cache-hit ratio and the latency from it.

## Web Application Firewall (`edge-gateway`)

The edge gateway is the [OWASP ModSecurity CRS image](https://github.com/coreruleset/modsecurity-crs-docker) (`owasp/modsecurity-crs:*-nginx-alpine`). `docker-compose.yml` pins it by its exact CRS and date tag. It is ModSecurity v3 with the **OWASP Core Rule Set**, which has signatures for SQL injection, XSS, remote code execution, scanners, file inclusion, and more.

It runs as a non-root reverse proxy on `127.0.0.1:80`. It inspects every request, and forwards it to `web-edge:8080` with the original `Host`. Environment variables in `docker-compose.yml` set all of its configuration; nothing is in the image:

- `BACKEND=http://web-edge:8080` (the web service's alias on the dedicated `edge` network), `PORT=8080`, `SERVER_NAME=qafiyah.com`.
- `NGINX_ALWAYS_TLS_REDIRECT=off`: TLS ends at Cloudflare, so the gateway never redirects to https itself.
- `REAL_IP_HEADER=CF-Connecting-IP` and `SET_REAL_IP_FROM=172.28.0.1` (dev: `172.29.0.1`): these give CRS the real client address for its scores.
  - The single address is the gateway of the `edge` network. The host's `cloudflared` appears to the container from that address, through docker-proxy on the published `127.0.0.1:80`.
  - Cloudflare sends that header, so the WAF scores each request against the real visitor. This was verified on 2026-09-28: the address that Cloudflare adds to `X-Forwarded-For` for the web nginx is the visitor's.
  - The gateway's access log still shows `172.28.0.1`. The image's `main` log format prints `$realip_remote_addr`, which is the connecting peer.
  - If the **Remove visitor IP headers** Managed Transform is turned on, the header disappears. Then the WAF scores every request as `172.28.0.1`. The web nginx does not depend on the header, because it reads `X-Forwarded-For` itself.
- `BLOCKING_PARANOIA=1` (the paranoia level), `ANOMALY_INBOUND=5` (the block threshold).

### Backend proxy override (`apps/edge-gateway/proxy_backend.conf.template`)

`apps/edge-gateway/proxy_backend.conf.template` is the image's own template, with a variable in `proxy_pass`. So nginx looks up `web-edge:8080` for each request, and does not keep one IP. This lets the zero-downtime web rollout in `docs/deployment/architecture.md` work. The file, the reason for it, and how to update it when the image changes are in `apps/edge-gateway/AGENTS.md`.

### Rollout: DetectionOnly to On

CRS can block **Arabic search input** by mistake, because unusual encodings or punctuation can trip the SQL injection or XSS rules. So the WAF ships with **`MODSEC_RULE_ENGINE=DetectionOnly`**. In this mode, it evaluates the rules and **logs** the requests that it would block, but lets every request through. Run it this way against real traffic first, then enforce it. `.claude/skills/deploy/SKILL.md` has the ordered steps to change to `On`, and the bypass and rollback.

> **Check before enforcement (2026-06-18): `On` is considered safe.**
>
> - A [Schemathesis](https://schemathesis.readthedocs.io) schema fuzz of every operation in the OpenAPI document ran through the full edge chain (`edge-gateway`, then the web nginx, then `api`). It used `MODSEC_RULE_ENGINE=On` with the production settings (`BLOCKING_PARANOIA=1`, `ANOMALY_INBOUND=5`).
> - Of 1737 cases, CRS blocked exactly **2**. Both were blocked by rule `949110` (inbound anomaly score 10, over the threshold of 5). Both were extreme fuzzer payloads, with NUL bytes and control characters in the query parameters of `/v1/poems`.
> - **CRS did not block legitimate Arabic `/v1/search` queries.** They reached the API and its normal input validation. This answers the main concern about false positives.
> - Synthetic fuzzing cannot model real traffic perfectly. So after the change, still follow `docker compose logs -f edge-gateway` for the first day. The result stays a go-ahead.

## Elasticsearch and the indexer (`apps/search-indexer`, Rust)

Search is served from Elasticsearch. `apps/search-indexer` fills it from Postgres. Both are in the same Compose stack, and bind only to loopback.

The indexer is a one-shot init job, and `api` waits for it. What it does and when it reindexes are in `apps/search-indexer/AGENTS.md`. To fix differences on a populated index, run `bun run reindex:prod`. It does a full rebuild and an alias swap. The bare `bun run reindex` acts on dev. `.claude/skills/deploy/SKILL.md` has the ordered command.

Elasticsearch has one CPU (`cpus: 1.0`). So its search thread pool has 2 threads, and concurrent searches wait in a queue. These are measurements on the dev stack with the full index:

- A burst of 16 searches takes 1.3 to 1.9 seconds when the node is warm.
- The same burst takes 2.8 to 3.5 seconds the first time after a restart.
- A single search right after a restart takes about 0.45 seconds.

So the node serves normal traffic normally after a restart. Only a burst on a cold node comes near the API's search deadline of 5 seconds (#208).

## Observability (`apps/observability`)

Prometheus, Loki, Alloy, Grafana, postgres_exporter, elasticsearch_exporter, blackbox_exporter, and the one-shot `db-monitor-role` job run in the same Compose stack. `bun run deploy` updates them together with `db` and `elasticsearch`. What each dashboard shows, and how to read it, is in `apps/observability/AGENTS.md`.

- **To open it:** run `bun run observe` on your laptop. It forwards `127.0.0.1:3301` to Grafana's `127.0.0.1:3000` on the VPS. Sign in as `admin` with `GRAFANA_ADMIN_PASSWORD`; the script prints the `sops` command that reads it. Grafana keeps no volume. So a deploy that recreates it (a new image or a changed configuration) asks for the sign-in again. Other deploys leave it running.
- **Alerts:** Grafana sends alerts to the Telegram group "Qafiyah Alerts". `apps/observability/AGENTS.md` ("Alerts") lists them, and says how to silence them before a reseed.
- **To restart it:** run `docker compose restart prometheus loki alloy grafana`. Neither the API nor the website waits for them. If Prometheus or Loki is down, there are only gaps in the graphs and the log views. Docker keeps writing its own rotated log files in either case.
- **Monitor credentials:**
  - `db-monitor-role` sets `PG_MONITOR_PASSWORD` on the `qafiyah_monitor` Postgres role on every `up`. `bun run db:reseed` also sets it right after its restore, which recreates the `public` schema and its grants.
  - The search indexer sets `ES_MONITOR_PASSWORD` on the Elasticsearch monitor user on every run.
  - A change to either one in `secrets/prod.enc.env` takes effect on the next deploy.
- **Disk:**
  - Prometheus keeps 15 days or 1 GB in the `qafiyah-prometheus-data` volume, whichever is smaller. A wipe loses only history. Do it only when someone asks: `docker compose rm -sf prometheus && docker volume rm qafiyah-prometheus-data`, then `docker compose up -d prometheus`.
  - Loki keeps 7 days of logs in `qafiyah-loki-data`. Alloy limits each service to 10 lines a second, beyond a burst of 2,000. So a flood of requests keeps Loki under about 1 GB.
  - Alloy keeps only its read positions, in `qafiyah-alloy-data`.

### Accounts database backups

`qafiyah_accounts` is a separate database in the same Postgres container. It holds the user records and the hashed API keys. It is outside the `data/db/` pipeline on purpose, because that pipeline gives the corpus to anyone who asks for a passphrase. So the accounts database has its own backup path:

- `scripts/accounts/backup.sh` runs every day at 03:17 UTC, from a systemd timer on the VPS.
  - It runs `pg_dump` in the `db` container, and encrypts the dump with `age` to `ACCOUNTS_BACKUP_RECIPIENT`.
  - It uploads the result with rclone (run from its image) to a private R2 bucket, under `accounts/`.
  - A failed dump uploads nothing.
- The recipient is an age public key whose private half is never on the VPS. It is a maintainer's key, never the VPS's own sops key. So the VPS can write backups, but cannot read them.
- An R2 lifecycle rule on the bucket controls retention (it deletes `accounts/` objects after 30 days). The script does not.
- The bucket is **not** `qafiyah-assets`, which is public at `cdn.qafiyah.com`. The backup bucket has no public binding and no custom domain.
- The upload uses an R2 API token with object read and write on the backup bucket only. It is not the deploy token, and it cannot reach `qafiyah-assets`. This limit keeps a compromised VPS away from the public asset bucket.
- The required environment is `ACCOUNTS_BACKUP_BUCKET`, `ACCOUNTS_BACKUP_RECIPIENT`, `ACCOUNTS_BACKUP_R2_ENDPOINT` (`https://<account-id>.r2.cloudflarestorage.com`), `ACCOUNTS_BACKUP_R2_ACCESS_KEY_ID`, and `ACCOUNTS_BACKUP_R2_SECRET_ACCESS_KEY`. The host also needs `age`, which is installed with sops (see `docs/deployment/secrets.md`).

Install the timer once, as root on the VPS:

```bash
cp /opt/qafiyah/scripts/accounts/qafiyah-accounts-backup.{service,timer} /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now qafiyah-accounts-backup.timer
```

- To check it, run `systemctl list-timers qafiyah-accounts-backup.timer`.
- To run one backup now, run `systemctl start qafiyah-accounts-backup.service`.
- To read its output, run `journalctl -u qafiyah-accounts-backup.service`.
- A failed run shows in `systemctl --failed`.

To restore, download the backup (from the Cloudflare dashboard, or with `rclone copyto`). Then decrypt it on the machine that holds the private key, and stream it to the VPS:

```bash
age -d -i ~/.config/sops/age/keys.txt qafiyah_accounts_<stamp>.dump.age \
  | ssh qafiyah 'cd /opt/qafiyah && docker compose exec -T db pg_restore -U qafiyah_accounts -d qafiyah_accounts --clean --if-exists --no-owner'
```

What a lost volume really costs:

- `users` and `identities` come back when each person signs in again.
- A lost `sessions` table only makes people sign in again.
- `usage_hourly` is only for display.
- Only `api_keys` hurts. Even then, qafiyah.com keeps working, because the site authenticates with `INTERNAL_API_KEY` from the environment and never reads the accounts database. Only third-party integrations stop working, until their owners make new keys.
