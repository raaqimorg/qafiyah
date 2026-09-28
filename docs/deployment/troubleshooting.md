# Deployment Troubleshooting & Operations

## Common operations (on the host, from `/opt/qafiyah`)

```bash
docker compose ps                       # status of all containers
docker compose logs -f web              # tail logs (web | api | search-indexer | db | elasticsearch)
docker compose restart web              # restart one service
docker compose down                     # stop the stack (keeps data volumes)
docker compose up -d                    # start the stack

# Updates/rebuilds: re-run the push-button deploy from a dev machine (`bun run deploy`);
# the zero-downtime roll lives in scripts/deploy/vps.sh, there is no separate on-host command.

# Cloudflare Tunnel:
systemctl status cloudflared
cloudflared tunnel list                 # name / id / health

# search-indexer (one-shot init job, no liveness endpoint):
docker compose logs search-indexer              # last run: provision + reindex outcome
docker compose ps -a search-indexer             # should show Exited (0)

# Rebuild the ES index from Postgres (full reindex + alias swap), from /opt/qafiyah:
docker compose run --rm -e SEARCH_INDEXER_FORCE=true search-indexer
# Or from your laptop over SSH: bun run reindex:prod

# DANGER: wipes DB + ES volumes and re-seeds on next boot
# docker compose down -v && docker compose up -d
```

## Major-version bumps of `postgres`/`elasticsearch` need a volume wipe

A new major image refuses to boot on the old volume's data, so `db`/`es` crash-loop, `--wait` never completes, and nothing else starts. Confirm with `docker compose logs db` / `logs elasticsearch`. Since the dataset is read-only/dump-shipped, wipe the affected volume and let it re-seed (DB from `data/db/`, ES rebuilt by the search-indexer):

```bash
docker compose down                               # no -v
docker volume rm qafiyah-db-data qafiyah-es-data  # only the bumped store(s) strictly need it
docker compose up -d --build --wait
```

Safe only while production stays read-only. Once it takes real writes, use `pg_upgrade` instead of wiping `qafiyah-db-data`.

## Gotchas

- **Loopback binds are deliberate**, see `docs/deployment/architecture.md#security-posture`.
- **Stale browser cache after deploy.** HTML carries a one-minute browser `max-age` (`apps/web/src/lib/server/cache.ts`), so a returning visitor sees the old build for at most a minute; nginx's own cache dies with the replaced container. Assets are content-hashed and never collide.
- **Cloudflare is the DNS authority**, DNS changes are instant, no registrar propagation wait.
- **Major-version bumps need a volume wipe**, see above.
- **Website searches answering 429.** Either nginx's `limit_req` (60 a minute per address, an HTML 429) or the API's website-visitor limit (3,600 an hour per /64 or IPv4 address, 36,000 per /48, a `problem+json` 429). The API's log line for a proxied `/v1/search` carries `rate_limit_remaining`. See `docs/deployment/environments.md` ("Where requests are limited").
- **`/healthz` never reaches the app through the edge.** The ModSecurity image answers `/healthz` with `OK` itself on every host, so check routing with `/api/v1/poems/random?option=slug` (site) or `/v1/poems/random?option=slug` (api), as the deploy does.
