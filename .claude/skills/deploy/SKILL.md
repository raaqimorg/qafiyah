---
name: deploy
description: Deploy, release data/index changes to, or roll back the qafiyah production VPS stack or the telemetry-proxy Cloudflare Worker. Use when the user asks to deploy, ship to prod, release, or roll back.
disable-model-invocation: true
---

Full architecture and "why" live in `docs/deployment/`. This is only the ordered steps. Link out rather than re-explaining.

## 1. Deploy the VPS stack (api, web, search-indexer, db, es, edge-gateway)

From a dev machine with the repo checked out and SSH access:

```bash
bun run deploy        # scripts/deploy/vps.sh
```

It first confirms the GitHub CI run for `origin/main` passed and refuses otherwise (still running, failed, or never ran); `bun run deploy --skip-ci-check` skips that in an emergency. This builds, then does a zero-downtime rolling replace of `api`/`web`, then smoke-gates, purges the Cloudflare cache (a failed purge exits non-zero: purge everything in the Cloudflare dashboard right away), prunes build cache, and finally checks `https://qafiyah.com/healthz` and `https://api.qafiyah.com/healthz` from your machine through Cloudflare, exiting non-zero if either does not answer 200. See `docs/deployment/architecture.md` for what it does internally. `bun` is not installed on the host; this always runs from your dev machine.

**Verify after it finishes:**

```bash
bun run api:conformance prod   # replays every documented API operation against the live schema
```

`docker compose ps` on the host should show all 6 containers healthy/exited-0.

## 2. Rolling back a bad deploy

**There is no separate rollback script.** `bun run deploy` always syncs the host to the current tip of `origin/main` (`git reset --hard FETCH_HEAD`). A bad build is already fail-safe (the old replica keeps serving until the new one passes its healthcheck, so it can't half-ship). To undo a deploy that _did_ complete: revert or reset `main` to the last-good commit, then run `bun run deploy` again the same way, once CI passes on that commit (or with `--skip-ci-check` if it can't wait). There is no faster path in this repo today.

## 3. Ship a new Postgres/Elasticsearch dump to prod (release data)

A normal deploy preserves the data volume, so a new dump in `data/db/` is ignored until it is restored.

```bash
bun run db:reseed     # push-button: syncs to origin/main, builds api and search-indexer, restores the newest dump, starts the new api, rebuilds es
```

Replaces the corpus database (`qafiyah`) only; `qafiyah_accounts` is untouched. It builds `api` and `search-indexer` from `origin/main` while the old stack serves, stops the API for the restore (a few minutes, nginx keeps serving cached pages), starts the freshly built API on the restored data, then rebuilds Elasticsearch with the freshly built indexer and an alias swap while the API serves. It ends with the same public health check as the deploy. It needs the newest dump's `DUMP_KEY__<dir>` in `secrets/prod.enc.env` and refuses to start without it. Prompts for confirmation unless run with `-y`. Details: `docs/deployment/environments.md`.

**A new dump together with code changes**, in this order:

1. Push the snapshot and the code, and let CI pass.
2. `bun run db:reseed`. The API it starts is already the new build, so a schema change the old API cannot read, or a new API that needs the new schema, is safe.
3. `bun run deploy` to roll out `web` (it also replaces `api` with the same build). No separate reindex: the reseed already rebuilt search with the new indexer.
4. Verify: `bun run api:conformance prod`. The home page's search filters read their options and counts from the API, so they follow the new dump on their own.

## 4. Rebuild the search index only (fix ES drift, no data change)

```bash
bun run reindex:prod
# equivalent, run on the host from /opt/qafiyah:
docker compose run --rm -e SEARCH_INDEXER_FORCE=true search-indexer
```

## 5. Major Postgres/Elasticsearch version bump

Not a normal deploy: a major image needs a volume wipe or it crash-loops. Ordered steps: `docs/deployment/troubleshooting.md`.

## 6. Enforce the WAF (flip ModSecurity from DetectionOnly to On)

Only when logs are clean against real Arabic-query traffic (background/validation history: `docs/deployment/services.md`).

```bash
# 1. Watch for would-be blocks first (audit-only in DetectionOnly mode):
docker compose logs -f edge-gateway | grep -i modsecurity   # esp. /v1/search and Arabic query strings

# 2. Sanity-check routing still works:
curl -s -H 'Host: qafiyah.com'     'http://127.0.0.1:80/api/v1/poems/random?option=slug' -o /dev/null -w 'apex %{http_code}\n'
curl -s -H 'Host: api.qafiyah.com' http://127.0.0.1:80/v1/docs -o /dev/null -w 'api  %{http_code}\n'

# 3. When clean, flip it:
#    set MODSEC_RULE_ENGINE: On in docker-compose.yml, commit, then run step 1 (bun run deploy).
```

If a legitimate request keeps tripping a rule afterward, exclude that rule id (CRS tuning), do **not** drop `BLOCKING_PARANOIA` to 0.

**Rollback (bypass the WAF entirely):** move the host bind back to `web` (`ports: ["127.0.0.1:80:8080"]` on the `web` service), drop the `edge-gateway` service, redeploy. The tunnel config needs no change.

## 7. Deploy the telemetry proxy (Cloudflare Worker, separate from the VPS stack)

After `bunx wrangler login` or with `CLOUDFLARE_API_TOKEN` set:

```bash
cd apps/telemetry-proxy
bun run deploy        # wrangler deploy
```

Independent of step 1: a change to the Worker itself (`apps/telemetry-proxy/`) needs this; a change to how the web app _calls_ it needs step 1 instead. Details: `apps/telemetry-proxy/AGENTS.md`.
