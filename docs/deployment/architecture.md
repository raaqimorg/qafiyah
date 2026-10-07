# Deployment Architecture

Production is one small Linux VPS behind Cloudflare. **Nothing is reachable from outside, not even SSH.** The web, the API, and SSH all arrive through a Cloudflare Tunnel, and `cloudflared` only dials _out_. Every listener (`sshd` and every container port) binds to `127.0.0.1`.

## How traffic gets in

`docs/topology.md` ("Production deployment") draws the path. This section explains how it works.

`cloudflared` runs as a **systemd service**. It dials out to Cloudflare, so there are no inbound ports 80, 443, or 22. `/etc/cloudflared/config.yml` sets the routing:

- `qafiyah.com` and `www` go to `localhost:80`.
- `api.qafiyah.com` goes to `localhost:80`.
- `ssh.qafiyah.com` goes to `ssh://localhost:22`.
- Every other host gets 404.

To see the tunnel's name and id, run `cloudflared tunnel list`.

**The `edge-gateway` container is the only gateway.** Both hosts enter on `127.0.0.1:80`, which runs ModSecurity v3 with the OWASP CRS:

- It inspects every request, and proxies it to the web container's nginx over a dedicated `edge` network, as `web-edge:8080`.
- It keeps the original `Host`, so the web nginx routes by it. The apex goes to the Astro server-side renderer. `api.*` goes to the `api` container over a dedicated `backend` network, as `api-backend:8787`.
- Neither the web nginx nor the API has a host port. Both are reachable only inside their networks.
- The tunnel configuration still points at `localhost:80`; the WAF did not change it.

SSH clients connect with `ProxyCommand cloudflared access ssh --hostname ssh.qafiyah.com`.

The trade-offs: there is no public port to fall back on, so if the tunnel is down, SSH is down too. The path also has one extra hop (edge gateway to web). So if the edge gateway crashes, the whole site is down until `restart: unless-stopped` starts it again.

## The stack (`/opt/qafiyah`, `docker compose`, 14 containers)

`docker-compose.yml` holds the definitive service definitions, the pinned image versions, and the ports.

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
| `qafiyah-grafana`                                       | the seven dashboards (admin login)       | `127.0.0.1:3000`          | **no**           |
| `qafiyah-loki`                                          | container logs, kept 7 days              | none (`:3100`)            | **no**           |
| `qafiyah-alloy`                                         | log shipping, container and host stats   | none (`:12345`)           | **no**           |

The last eight containers are the private observability stack (`apps/observability/AGENTS.md`). To open it, run `bun run observe`. It forwards a port over the existing SSH connection, which goes through the tunnel.

- **The database seeds itself** on the first boot, from the newest dump in `data/db/`. This happens only when the data volume is empty.
  - In production, `bun run db:reseed` forces a restore. It recreates the database container from the synced checkout, and runs the same restore script in it. In dev, `bun run db:reset` forces a restore.
  - After a restore, the script renames the running container to `<container>-<dump-number>` (for example, `qafiyah-db-0019`). So `docker ps` shows which dump is live.
  - `container_name` in the Compose files does not change. Compose keeps track of the container by its own labels.
- **`search-indexer`** is a one-shot init job. To force a reindex, see `apps/search-indexer/AGENTS.md` and `docs/deployment/services.md`.

### Isolation of production from dev (both on one host)

The bare `docker compose` commands, and `scripts/deploy/vps.sh` and `scripts/db/reseed.sh`, act on the **production** stack. Only `docker-compose.yml` defines it: the project `qafiyah`, the containers `qafiyah-*`, the host ports `5433`, `9200`, and `80`, and the volumes `qafiyah-db-data` and `qafiyah-es-data`.

Every **dev** command goes through `scripts/dev/compose.sh`: `bun run dev`, `up`, `down`, `db:up`, `db:reset`, `es:*`, and `reindex`. This script sets a separate project (`qafiyah-dev`), and adds `docker-compose.dev.yml`. That file moves everything into a `-dev` namespace: the containers `qafiyah-dev-*`, the host ports `5434`, `9201`, and `8090`, and the volumes `qafiyah-dev-*`.

So a dev clone can run **on the same VPS** without a collision. A dev `down -v` or `db:reset` can only wipe `qafiyah-dev-*` volumes. Nothing changes `docker-compose.yml` for dev, so a deploy never changes the identity of production.

## Deploy mechanics (what `bun run deploy` does)

`.claude/skills/deploy/SKILL.md` has the ordered procedure. This section shows what happens inside it.

The deploy uploads its steps over SSH to a temporary file on the host. It runs that file with stdin from `/dev/null` (`remote_exec` in `scripts/lib/remote.sh`, which `db:reseed` and `reindex:prod` also use). So a step that reads stdin, such as `docker compose exec`, cannot consume the steps after it. In `/opt/qafiyah`, it runs these commands:

```bash
git fetch --depth 1 origin main
git reset --hard FETCH_HEAD
docker compose build api web search-indexer                     # build; the old stack keeps serving
docker compose up -d --no-deps db elasticsearch search-indexer edge-gateway \
  db-monitor-role postgres-exporter elasticsearch-exporter blackbox-exporter \
  prometheus loki alloy grafana                                 # update the services that are not rolled
# then a rolling replace of api, then web (see rollout() in scripts/deploy/vps.sh):
#   docker compose up -d --no-deps --no-recreate --scale api=2 api   # add a new replica
#   <wait until the new replica's healthcheck passes>                # the old one keeps serving
#   docker stop/rm <old api id>                                      # remove the old one
curl -fsS -H 'Host: qafiyah.com' 'http://127.0.0.1:80/api/v1/poems/random?option=slug' -o /dev/null   # edge smoke gate
curl -fsS -H 'Host: api.qafiyah.com' 'http://127.0.0.1:80/v1/poems/random?option=slug' -o /dev/null
docker compose ps
docker builder prune -f --max-used-space 5GB   # cap the build cache so that it cannot fill the disk
```

Back on the dev machine, `check_public_health` (`scripts/lib/remote.sh`) does a final check:

- It requests `https://qafiyah.com/api/v1/poems/random?option=slug` and `https://api.qafiyah.com/v1/poems/random?option=slug` through Cloudflare, with a few retries.
- The deploy fails if either one does not answer 200. `db:reseed` ends with the same check.
- These checks, like the edge smoke gate, use a path that reaches the API and Postgres. The edge gateway image answers `/healthz` itself for every host, so a `/healthz` check would pass with `web` or `api` down.
- The live random poem is `no-store`, so no cache can answer it either.

After that, the script caps the Docker **build cache** at about 5 GB. It keeps the recent layers, so the next builds stay fast. It then prints a one-line summary of what else can be removed. This is the **only** thing that a deploy ever deletes, and it deletes only build cache. It never deletes images in use, containers, or named volumes, and it never touches the database or Elasticsearch data. To remove the rest by hand, run `docker system prune`, which also keeps the volumes. It also removes stopped containers, including the finished one-shot jobs.

**Zero downtime.** The `rollout()` helper in `scripts/deploy/vps.sh` needs no plugin. It works like this:

1. It doubles the `web` or `api` replicas on the new image.
2. It waits until the new replicas pass their healthcheck.
3. It removes the old replicas by id.

For the overlap to serve traffic, both nginx hops look up their upstream for each request, instead of keeping the IP from startup. The edge gateway proxies `web-edge:8080` through an nginx variable (`apps/edge-gateway/proxy_backend.conf.template`, mounted over the image's template). The web nginx proxies `api-backend:8787` the same way (`apps/web/nginx.conf`). Both use `resolver 127.0.0.11 valid=5s`.

Because the rollout scales `web` and `api`, they have **no `container_name`**. Their replicas are `qafiyah-web-2` and so on, so address them by service name. They also have **no host port**; they only `expose` one.

The rollout is **fail-safe**. The old replica stops only after the new one is healthy. So a bad build stops the deploy while the old container still serves. The stateful and init services (`db`, `elasticsearch`, `search-indexer`) and the `edge-gateway` are updated in place. The `edge-gateway` is recreated, with a short interruption, only on a deploy that changes its configuration.

**Rate limits are counted in each process, so a rollout briefly doubles them.** The limiter is an in-memory map in each `api` container (`apps/api/src/rate_limit.rs`).

- While `rollout()` runs both the old and the new replica, each keeps its own windows. So every caller's real quota is up to twice the limit until the old replica stops.
- The counts of the old replica are lost, not merged.
- This is accepted, because the rollout window limits it and it corrects itself.
- It becomes a real problem only if `api` ever runs with more than one replica all the time. That would multiply every plan's limit, with no error and no log line. The fix at that point is to move the limiter's state out of the process.

The volumes persist, and a failed build leaves the running stack as it was. **Bun is not installed on the host.** The `bun run ...` scripts run only on a dev machine. On the server, use plain `docker compose`. The one exception is a major version upgrade of a stateful store (see `docs/deployment/troubleshooting.md`).

## Security posture

- **Nothing is reachable from outside, not even SSH.** The web, the API, and SSH arrive over the tunnel, and `cloudflared` only connects out. Every listener binds to `127.0.0.1`, so SSH, the database, Elasticsearch, and the search indexer are never public. A port published on `0.0.0.0` goes around the firewall, so keep every bind on loopback. This is deliberate everywhere.
- The host has a baseline of hardening. The exact rules are on the server.
  - The inbound firewall denies everything by default, and has **no** allow rules. `sshd` listens on loopback only, and the tunnel reaches it.
  - SSH accepts keys only, and bans repeated failed logins.
  - Security updates install automatically.
  - Swap prevents out-of-memory failures during builds and in Elasticsearch and Postgres.
- Every service has a `mem_limit`, `cpus`, and `pids_limit` in `docker-compose.yml`. Elasticsearch and Postgres also have a `mem_reservation`. The limits fit the server: 4 vCPUs and 8 GB of RAM, measured on 2026-10-05. So a leak or a fork storm in one container cannot bring down the whole host. `api` and `search-indexer` also run with `read_only: true`.
- Base images are pinned by tag, not by digest, on purpose. Floating `-alpine` tags pick up security patches of the base OS on each rebuild. The application dependencies are locked by `--locked` (Rust) and `--frozen-lockfile` (Bun). When a specific base revision matters, change the tag explicitly.
- Alloy mounts the Docker socket and the host's `/`, `/proc`, `/sys`, and `/var/lib/docker`, all read-only, to read every container's logs and stats.
  - So if an attacker takes over Alloy, they have the equivalent of root. Alloy publishes no port, and joins only the `observability` network.
  - Loki keeps visitor addresses, request URIs, and search text for 7 days.
  - Alloy ships at most 10 lines a second for each service, beyond a burst of 2,000. So a flood of requests keeps Loki under about 1 GB.
- The observability stack adds nothing public:
  - Grafana binds `127.0.0.1:3000`, and joins only the `observability` network, with Prometheus.
  - The API's metrics port (`9464`) is reachable only on the internal networks.
  - Prometheus's OTLP receiver is on `default`, and on `metrics`, which is the website's only way to reach it.
  - The exporters use their own monitor credentials (`PG_MONITOR_PASSWORD`, `ES_MONITOR_PASSWORD`), with read-only monitoring rights.
- To check exposure, run `ss -tulpn | grep -vE '127\.0\.0\.1|\[::1\]'`. It must show **no** public listeners, only the outbound QUIC sockets of `cloudflared`. To see the firewall, run `ufw status verbose`.
