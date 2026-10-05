# Deployment Architecture

One small Linux VPS fronted by Cloudflare. **Nothing is reachable inbound, including SSH**: web, API, and SSH all arrive through a Cloudflare Tunnel (`cloudflared` dials _out_ only), and every listener (`sshd` plus every container port) binds to `127.0.0.1`.

## How traffic gets in

The path is drawn in `docs/topology.md` ("Traffic into the VPS"); this section is the mechanics.

`cloudflared` runs as a **systemd service**, dialing out to Cloudflare (no inbound 80/443/22). Routing in `/etc/cloudflared/config.yml`: `qafiyah.com` + `www` → `localhost:80`, `api.qafiyah.com` → `localhost:80`, `ssh.qafiyah.com` → `ssh://localhost:22`, else 404. (`cloudflared tunnel list` for name/id.)

**The `edge-gateway` container is the single gateway**: both hosts enter on `127.0.0.1:80` (ModSecurity v3 + OWASP CRS), which inspects every request and reverse-proxies to the web container's nginx over a dedicated `edge` network (resolved as `web-edge:8080`), preserving the original `Host` so nginx routes by it (apex → Astro SSR; `api.*` → the `api` container over a dedicated `backend` network as `api-backend:8787`). Neither the web nginx nor the API has a host port; both are in-network only. The tunnel config still points at `localhost:80`, unchanged by the WAF. SSH clients connect via `ProxyCommand cloudflared access ssh --hostname ssh.qafiyah.com`.

Tradeoff: no public-port fallback (if the tunnel is down, SSH is gone too); and the path has one extra hop (edge-gateway → web), so an edge-gateway crash drops the whole edge until `restart: unless-stopped` recovers it.

## The stack (`/opt/qafiyah`, `docker compose`, 12 containers)

Canonical service definitions, pinned image versions, and ports live in `docker-compose.yml`.

| Container                                               | Role                                     | Host bind (loopback)      | Public?          |
| ------------------------------------------------------- | ---------------------------------------- | ------------------------- | ---------------- |
| `qafiyah-edge-gateway`                                  | ModSecurity + OWASP CRS (edge gateway)   | `127.0.0.1:80`            | via tunnel       |
| `qafiyah-web-N`                                         | Astro SSR + nginx (Host routing)         | none (`web:8080`)         | via edge-gateway |
| `qafiyah-api-N`                                         | Rust / axum API (mounted at `/v1`)       | none (`api-backend:8787`) | via nginx        |
| `qafiyah-db` (renamed `qafiyah-db-<N>` after a restore) | PostgreSQL                               | `127.0.0.1:5433`          | **no**           |
| `qafiyah-es`                                            | Elasticsearch                            | `127.0.0.1:9200`          | **no**           |
| `qafiyah-search-indexer`                                | Postgres → Elasticsearch sync            | none (init job)           | **no**           |
| `qafiyah-db-monitor-role`                               | ensures the `qafiyah_monitor` role       | none (init job)           | **no**           |
| `qafiyah-postgres-exporter`                             | `pg_stat_statements` and Postgres health | none (`:9187`)            | **no**           |
| `qafiyah-elasticsearch-exporter`                        | cluster health, heap, query totals       | none (`:9114`)            | **no**           |
| `qafiyah-blackbox-exporter`                             | `/healthz` probes of api and web         | none (`:9115`)            | **no**           |
| `qafiyah-prometheus`                                    | metrics store, scrapes, OTLP receiver    | none (`:9090`)            | **no**           |
| `qafiyah-grafana`                                       | the four dashboards (admin login)        | `127.0.0.1:3000`          | **no**           |

The last six are the private observability stack (`apps/observability/AGENTS.md`): open it with `bun run observe`, an SSH port forward over the existing tunnel.

- **DB self-seeds** on first boot from the newest dump in `data/db/` (only when the data volume is empty). Whichever script forces a restore (`bun run db:reseed` in prod, which reruns the same restore script inside the running container, or `bun run db:reset` in dev) renames the running container afterward to `<container>-<dump-number>` (e.g. `qafiyah-db-0019`), so `docker ps` shows which dump is live. `container_name` in the compose files is unaffected, Compose keeps tracking the container by its own labels.
- **`search-indexer`** is a one-shot init job, see `apps/search-indexer/AGENTS.md` and `docs/deployment/services.md` for how to force a reindex.

### Prod vs dev isolation (both on one host)

The bare `docker compose` commands (and `scripts/deploy/vps.sh` / `scripts/db/reseed.sh`) operate on the **production** stack defined by `docker-compose.yml` alone: project `qafiyah`, containers `qafiyah-*`, host ports `5433`/`9200`/`80`, volumes `qafiyah-db-data` / `qafiyah-es-data`.

Every **dev-facing** command (`bun run dev` / `up` / `down` / `db:up` / `db:reset` / `es:*` / `reindex`) routes through `scripts/dev/compose.sh`, which pins a separate project (`qafiyah-dev`) and layers `docker-compose.dev.yml`, remapping into a `-dev` namespace: containers `qafiyah-dev-*`, host ports `5434`/`9201`/`8090`, volumes `qafiyah-dev-*`. So a dev clone runs **on this same VPS** without colliding, and a dev `down -v` / `db:reset` can only ever wipe `qafiyah-dev-*` volumes. `docker-compose.yml` is never modified for dev, so deploys stay a no-op on prod identity.

## Deploy mechanics (what `bun run deploy` automates)

The ordered procedure is in `.claude/skills/deploy/SKILL.md`; this is what happens inside it. It uploads its steps over SSH to a temporary file on the host and runs that file with stdin from `/dev/null` (`remote_exec` in `scripts/lib/remote.sh`, shared with `db:reseed` and `reindex:prod`), so a step that reads stdin, such as `docker compose exec`, cannot swallow the steps after it. In `/opt/qafiyah` it runs:

```bash
git fetch --depth 1 origin main
git reset --hard FETCH_HEAD
docker compose build api web search-indexer                            # build; old stack keeps serving
docker compose up -d --no-deps db elasticsearch search-indexer edge-gateway     # converge non-rolled services
# then a rolling replace of api, then web (see rollout() in scripts/deploy/vps.sh):
#   docker compose up -d --no-deps --no-recreate --scale api=2 api   # add new replica
#   <wait for the new replica's healthcheck to pass>                 # old keeps serving
#   docker stop/rm <old api id>                                      # drain old
curl -fsS -H 'Host: qafiyah.com' 'http://127.0.0.1:80/api/v1/poems/random?option=slug' -o /dev/null   # edge smoke gate
curl -fsS -H 'Host: api.qafiyah.com' 'http://127.0.0.1:80/v1/poems/random?option=slug' -o /dev/null
docker compose ps
docker builder prune -f --max-used-space 5GB   # cap build cache so it can't fill the disk
```

Back on the dev machine, `check_public_health` (`scripts/lib/remote.sh`) requests `https://qafiyah.com/api/v1/poems/random?option=slug` and `https://api.qafiyah.com/v1/poems/random?option=slug` through Cloudflare with a few retries and fails the deploy if either does not answer 200; `db:reseed` ends with the same check. Both checks, like the edge smoke gate, use a path that reaches the API and Postgres: the edge gateway image answers `/healthz` itself for every host, so a `/healthz` check would pass with `web` or `api` down. The live random poem is `no-store`, so no cache can answer it either.

Afterward the script caps the Docker **build cache** at ~5GB (keeps recent layers so incremental builds stay fast) and prints a one-line summary of remaining reclaimable leftovers. This is the **only** thing a deploy ever deletes, and only build cache (never images in use, containers, or named volumes; db/es data untouched). Reclaim everything else by hand with `docker system prune` (still volume-safe).

**Zero-downtime.** The `rollout()` helper in `scripts/deploy/vps.sh` (self-contained, no plugin) doubles `web`/`api` replicas on the freshly built image, waits for the new ones to pass their healthcheck, then drains the originals by id. For the overlap to serve, both nginx hops re-resolve their upstream per request instead of pinning the IP at startup: the edge gateway proxies `web-edge:8080` via an nginx variable (`apps/edge-gateway/proxy_backend.conf.template`, mounted over the image's template) and the web nginx proxies `api-backend:8787` via one (`apps/web/nginx.conf`), both leaning on `resolver 127.0.0.11 valid=5s`. Because the roll scales them, `web`/`api` carry **no `container_name`** (replicas are `qafiyah-web-2` etc.; address by service) and **no host port** (they only `expose`). It is **fail-safe**: the old replica drains only after the new one is healthy, so a bad build aborts the deploy with the old container still serving. The stateful/init services (`db`, `es`, `search-indexer`) and the `edge-gateway` converge the old way; the `edge-gateway` recreates (a one-time edge blip) only on a deploy that changes its config.

**Rate limits are per process, so a rollout briefly doubles them.** The limiter is an in-memory map in each `api` container (`apps/api/src/rate_limit.rs`), so while `rollout()` runs both the old and new replica, each keeps its own windows and every caller's effective quota is up to double until the old one drains. The draining replica's counts are lost rather than merged. This is accepted: it is bounded by the rollout window and self-healing. It becomes a real problem only if `api` is ever run at more than one replica steady-state, which would silently multiply every plan's ceiling with no error and no log line. Moving limiter state out of process is the fix at that point.

Volumes persist and a failed build leaves the running stack untouched. **`bun` is not installed on the host**, `bun run …` scripts are dev-machine only; on the box use raw `docker compose`. Exception: a stateful major-version bump (see `docs/deployment/troubleshooting.md`).

## Security posture

- **Nothing is reachable inbound, SSH included.** Web, API, and SSH arrive over the tunnel (`cloudflared` egress-only), and every listener binds to `127.0.0.1`, so SSH, DB, Elasticsearch, and the search-indexer are never publicly exposed. Publishing on `0.0.0.0` punches through the firewall, keep binds on loopback (deliberate everywhere).
- Host hardening baseline (exact rules live on the box): default-deny inbound firewall with **no** allow rules (`sshd` listens on loopback only, reached via the tunnel), key-only SSH with brute-force banning, automatic security updates, and swap so builds/ES/Postgres don't OOM.
- Every service carries a `mem_limit`, `cpus`, and `pids_limit` in `docker-compose.yml` (Elasticsearch and Postgres also a `mem_reservation`), sized for the box (4 vCPUs, 8 GB of RAM, measured 2026-10-05), so a leak or fork storm in one container cannot take the whole host down. `api` and `search-indexer` additionally run `read_only: true`.
- Base images are pinned by tag, not digest, deliberately: floating `-alpine` tags pick up base-OS security patches on each rebuild, while the application dependencies are locked by `--locked` (Rust) and `--frozen-lockfile` (Bun). Bump a tag explicitly when a specific base revision matters.
- The observability stack adds no public surface: Grafana binds `127.0.0.1:3000` and joins only the `observability` network with Prometheus, the API's metrics port (`9464`) and Prometheus's OTLP receiver are reachable only on the internal `default` network, and the exporters use their own monitor credentials (`PG_MONITOR_PASSWORD`, `ES_MONITOR_PASSWORD`) with read-only monitoring privileges.
- Exposure check: `ss -tulpn | grep -vE '127\.0\.0\.1|\[::1\]'` should show **no** public listeners (only `cloudflared`'s outbound QUIC sockets); `ufw status verbose` for the firewall.
