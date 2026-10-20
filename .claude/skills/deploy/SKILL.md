---
name: deploy
description: Deploy, release data or index changes to, or roll back the qafiyah production VPS stack. Use when the user asks to deploy, ship to prod, release, or roll back.
disable-model-invocation: true
---

This skill has only the ordered steps. The full architecture and the reasons are in `docs/deployment/`. Link to them; do not explain them again here.

## 0. Release the version into `main` first

A deploy ships `main`. `main` changes only in two ways:

- A version is released into it.
- An urgent fix is merged into it directly. Then deploy it, and merge `main` into the open version branch (`docs/pull-requests.md`).

In every other case, release the open version (`vN`) before you deploy. Follow "Versions and releases" in `docs/pull-requests.md`:

1. Write `docs/changelog/vN.md`.
2. Merge the release pull request into `main` with a merge commit.
3. Deploy. The release pull request's CI run already tested the same files, so the deploy does not wait for the CI run on `main`.

After the deploy, delete `vN`, and open `v<N+1>` from `main`.

## 1. Deploy the VPS stack

This covers every container in `docker-compose.yml`: the site, the data stores, the edge gateway, and the observability stack. Run it from a dev machine with the repo and SSH access:

```bash
bun run deploy        # scripts/deploy/vps.sh
```

1. It checks that the GitHub CI run for `origin/main` passed. If that run failed, it stops. If that run is still going or never ran, it accepts a passed CI run of a release pull request (head branch `vN`) whose commit has exactly the same files as `origin/main`; otherwise it stops. In an emergency, `bun run deploy --skip-ci-check` skips this check.
2. It builds, then replaces `api` and `web` with zero downtime. It applies the mounted configuration files that changed since the last deploy: the edge template with a graceful nginx reload, and the observability files by recreating their service (`docs/deployment/architecture.md`).
3. It runs the edge smoke gate.
4. It purges the Cloudflare cache. If the purge fails, the deploy exits with an error. Then purge everything in the Cloudflare dashboard at once.
5. It prunes the build cache.
6. From your machine, through Cloudflare, it requests `https://qafiyah.com/api/v1/poems/random?option=slug` and `https://api.qafiyah.com/v1/poems/random?option=slug`. It exits with an error if either one does not answer 200.

`docs/deployment/architecture.md` explains what it does internally. Bun is not installed on the host, so always run this from your dev machine.

**Verify after it finishes:**

```bash
bun run api:conformance prod   # sends every documented API example to prod through Schemathesis (needs Docker)
```

On the host, `docker compose ps -a` must show all 15 containers healthy, or exited with 0 for the one-shot jobs.

## 2. Rolling back a bad deploy

**There is no separate rollback script.** `bun run deploy` always syncs the host to the current tip of `origin/main` (`git reset --hard FETCH_HEAD`).

A bad build is already fail-safe. The old replica keeps serving until the new one passes its healthcheck, so a deploy cannot ship half of a change.

To undo a deploy that _did_ complete, do these steps:

1. Revert `main` to the last good commit, as an urgent fix (`docs/pull-requests.md`).
2. When CI passes on that commit, run `bun run deploy` again. If it cannot wait, use `--skip-ci-check`.

This repo has no faster path today.

## 3. Ship a new Postgres or Elasticsearch dump to prod (release data)

A normal deploy keeps the data volume. So a new dump in the R2 bucket `qafiyah-data` has no effect until it is restored.

```bash
bun run db:reseed 0047_25_10_2026   # syncs to origin/main, builds api and search-indexer, restores the named dump, starts the new api, rebuilds es
```

It replaces only the corpus database (`qafiyah`). It does not touch `qafiyah_accounts`. It does these steps:

1. It downloads the named dump from R2 to the server, and deletes every other dump there (`scripts/db/use-dump.sh`).
2. It builds `api` and `search-indexer` from `origin/main`, while the old stack serves.
3. It checks that the disk can hold a second copy of the corpus, and stops if not.
4. It restores the dump into `qafiyah_next`, while the API keeps serving `qafiyah` (#322). The restore runs the scripts of the synced checkout, not the scripts that the container started with (#209).
5. It swaps the two databases in one transaction, and drops the old one. The queries that run at that moment fail. A request that needs a new connection waits through the swap (#330).
6. It rolls the API to the new build, as a deploy does.
7. It rebuilds Elasticsearch with the new indexer and an alias swap, while the API serves.
8. It ends with the same public health check as the deploy.

The dump folder name is required, so a rollback is the same command with an older name. It needs the `DATA_*` values in `secrets/prod.enc.env` (the read-only token). It asks for confirmation, unless you run it with `-y`. For details, see `docs/deployment/environments.md`.

**For a new dump together with code changes**, do these steps in this order:

1. Release the version that holds the code for the new dump into `main` (step 0), and let CI pass.
2. Run `bun run db:reseed <dump-folder>`. Between the swap and the end of its API rollout (about 30 seconds), the old API serves the new data. If the old API cannot read the new schema, its queries fail for those seconds.
3. Run `bun run deploy` to roll out `web`. It also replaces `api` with the same build. No separate reindex is necessary, because the reseed already rebuilt search with the new indexer.
4. Verify with `bun run api:conformance prod`. The home page's search filters read their options and counts from the API, so they follow the new dump automatically.

## 4. Rebuild only the search index (fix Elasticsearch differences, no data change)

```bash
bun run reindex:prod
# the same, run on the host from /opt/qafiyah:
docker compose run --rm -e SEARCH_INDEXER_FORCE=true search-indexer
```

## 5. Major Postgres or Elasticsearch version upgrade

This is not a normal deploy. A new major image needs a volume wipe, or it restarts in a loop. The Postgres volume also holds the accounts database, so back it up first. The ordered steps are in `docs/deployment/troubleshooting.md`.

## 6. Enforce the WAF (change ModSecurity from DetectionOnly to On)

Do this only when the logs are clean against real traffic with Arabic queries. The background and the validation history are in `docs/deployment/services.md`.

```bash
# 1. First, look for requests that it would block (DetectionOnly only logs them):
docker compose logs -f edge-gateway | grep -i modsecurity   # especially /v1/search and Arabic query strings

# 2. Check that routing still works:
curl -s -H 'Host: qafiyah.com'     'http://127.0.0.1:80/api/v1/poems/random?option=slug' -o /dev/null -w 'apex %{http_code}\n'
curl -s -H 'Host: api.qafiyah.com' http://127.0.0.1:80/v1/docs -o /dev/null -w 'api  %{http_code}\n'

# 3. When the logs are clean, enforce it:
#    set MODSEC_RULE_ENGINE: On in docker-compose.yml, release it into main (step 0), then run step 1 (bun run deploy).
```

If a legitimate request keeps tripping a rule after that, exclude that rule id (CRS tuning). Do **not** lower `BLOCKING_PARANOIA` to 0.

**Rollback:** set `MODSEC_RULE_ENGINE: DetectionOnly` again, merge it into `main` as an urgent fix, and deploy. The gateway then blocks nothing, and the tunnel configuration needs no change.

Removing the gateway from the path is not a quick rollback. The deploy script names `edge-gateway`, and its rolling replace scales `web` to two replicas. A host port on `web` allows only one replica. So that change also needs changes to `scripts/deploy/vps.sh`.
