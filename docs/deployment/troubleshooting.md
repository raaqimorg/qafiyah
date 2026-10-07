# Deployment Troubleshooting and Operations

## Common operations (on the host, from `/opt/qafiyah`)

```bash
docker compose ps                       # the status of every container
docker compose logs -f web              # follow the logs of one service (web, api, search-indexer, db, elasticsearch, ...)
docker compose restart web              # restart one service
docker compose down                     # stop the stack (the data volumes stay)
docker compose up -d                    # start the stack

# To update or rebuild, run the deploy again from a dev machine (`bun run deploy`).
# The zero-downtime replace is in scripts/deploy/vps.sh; there is no separate command on the host.

# Cloudflare Tunnel:
systemctl status cloudflared
cloudflared tunnel list                 # name, id, and health

# search-indexer (a one-shot init job, with no liveness endpoint):
docker compose logs search-indexer              # the last run: the provision and reindex result
docker compose ps -a search-indexer             # must show Exited (0)

# Rebuild the Elasticsearch index from Postgres (a full reindex and an alias swap), from /opt/qafiyah:
docker compose run --rm -e SEARCH_INDEXER_FORCE=true search-indexer
# Or from your laptop, over SSH: bun run reindex:prod

# DANGER: this deletes the database and Elasticsearch volumes, including the accounts database.
# docker compose down -v && docker compose up -d
```

## A major version upgrade of `postgres` or `elasticsearch` needs a volume wipe

A new major image refuses to start on the data of the old volume. So `db` or `elasticsearch` restarts in a loop, `--wait` never finishes, and nothing else starts. To confirm, run `docker compose logs db` or `docker compose logs elasticsearch`.

The corpus comes from the dumps, so you can wipe its volume and let it seed again. On a new volume, Postgres restores the newest dump from `data/db/`, and the search indexer rebuilds Elasticsearch.

**The Postgres volume also holds the accounts database** (`qafiyah_accounts`, with the users and their API keys). A wipe deletes it. So for a Postgres upgrade, do these steps:

1. Take a new accounts backup: `systemctl start qafiyah-accounts-backup.service`. Check that it succeeded: `systemctl show qafiyah-accounts-backup.service -p Result`.
2. Wipe and seed again:

   ```bash
   docker compose down                               # no -v
   docker volume rm qafiyah-db-data qafiyah-es-data  # only the upgraded store needs it
   docker compose up -d --build --wait
   ```

3. Restore the accounts backup. The steps are in `docs/deployment/services.md` ("Accounts database backups").

For an Elasticsearch upgrade alone, remove only `qafiyah-es-data`. The accounts are not in it. As an alternative for Postgres, `pg_upgrade` keeps the volume.

## Known problems

- **The loopback binds are deliberate.** See `docs/deployment/architecture.md#security-posture`.
- **Old pages in the browser after a deploy.** HTML has a one-minute browser `max-age` (`apps/web/src/lib/server/cache.ts`). So a returning visitor sees the old build for at most a minute. The nginx cache goes away with the replaced container. Assets have content-hashed names, so old and new assets never collide.
- **Cloudflare is the DNS authority.** DNS changes take effect at once, with no registrar delay.
- **A major version upgrade needs a volume wipe.** See above.
- **Website searches answer 429.** There are two possible causes:
  - nginx's `limit_req`: 60 requests a minute for each address, with an HTML 429.
  - the API's limit for website visitors: 3,600 an hour for each /64 or IPv4 address, and 36,000 for each /48, with a `problem+json` 429.

  The API's log line for a proxied `/v1/search` has `rate_limit_remaining`. See `docs/deployment/environments.md` ("Where requests are limited").

- **`/healthz` never reaches the app through the edge.** The ModSecurity image answers `/healthz` with `OK` itself, on every host. So to check routing, request `/api/v1/poems/random?option=slug` on the site, or `/v1/poems/random?option=slug` on the API, as the deploy does.
