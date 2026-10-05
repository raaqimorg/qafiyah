# Observability Agent Guide

Config only: the Prometheus, Loki, Alloy, and Grafana setup behind the private views of #137. The services themselves are defined in `docker-compose.yml`; nothing here builds.

## Shape

- `prometheus.yml`: every 15 s, scrapes every API replica on its metrics port (`dns_sd_configs` on `api:9464`, native histograms on), postgres_exporter, elasticsearch_exporter, blackbox_exporter's `/healthz` probes of `api` and `web`, Alloy's container (cAdvisor) and host (node exporter) components, Loki, and itself. The website is not scraped: it pushes over OTLP to the receiver Prometheus enables with `--web.enable-otlp-receiver`. `--enable-feature=created-timestamp-zero-ingestion` stores each series' start time as a zero sample, so a burst that creates a series (a route's first errors) is counted. Retention is 15 days or 1 GB, whichever comes first.
- `loki.yml`: Loki as one binary on its own volume, filesystem storage, logs kept 14 days (the compactor deletes older ones).
- `alloy/config.alloy`: Grafana Alloy reads every container of this compose project (`COMPOSE_PROJECT`, so a dev stack on the same host is not shipped twice) through the Docker socket and pushes the logs to Loki with `service` and `container` labels. It also runs cAdvisor (CPU and memory only, against each container's limits) and the node exporter (host CPU, memory, disk), which Prometheus scrapes from `alloy:12345/api/v0/component/...`.
- `grafana/provisioning/`: the Prometheus (uid `prometheus`) and Loki (uid `loki`) datasources, and one dashboard provider that loads `grafana/dashboards/` into the `Qafiyah` folder on every start.
- `grafana/dashboards/`: `latency.json`, `slow-queries.json`, `elasticsearch.json`, `health.json` (the home dashboard), `resources.json`, `edge.json`, `logs.json`. Each opens with a text panel saying what it measures and how to read it.

## Where the numbers come from

| Data                                              | Recorded by                                                                        | What it is                                                                                                                                                                                                   |
| ------------------------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `http_server_request_duration_seconds{job="api"}` | `apps/api/src/metrics.rs`, outermost layer                                         | every API request, by method, route template (`unmatched` for none), and status; a native histogram                                                                                                          |
| `http_server_request_duration_seconds{job="web"}` | `apps/web/src/middleware.ts`                                                       | every request that reaches Astro, timed to the end of its body (once, even when the page rewrites to `/404`), by method, route pattern, and status; an OpenTelemetry exponential histogram pushed every 15 s |
| `db_client_operation_duration_seconds`            | `apps/api/src/es/client.rs`                                                        | every Elasticsearch search the API sends, by alias, failures included                                                                                                                                        |
| `pg_stat_statements_*`                            | postgres_exporter as `qafiyah_monitor`                                             | the 100 statements with the most total time since the last reset, with their text                                                                                                                            |
| `elasticsearch_*`                                 | elasticsearch_exporter as `qafiyah_monitor`                                        | cluster health, heap, search totals, thread pool rejections                                                                                                                                                  |
| `probe_success`                                   | blackbox_exporter                                                                  | whether `/healthz` answered 2xx                                                                                                                                                                              |
| `container_*`                                     | cAdvisor in Alloy                                                                  | each container's memory working set and CPU, its limits, and its start time (restarts)                                                                                                                       |
| `node_*`                                          | node exporter in Alloy                                                             | the host's CPU, memory, swap, and disk                                                                                                                                                                       |
| `{service="web"}` nginx lines                     | the website's nginx (`apps/web/nginx.conf`, `log_format json`) via Alloy into Loki | one JSON line per request nginx answered: host, uri, status, `request_time`, `upstream_time`, `cache`                                                                                                        |
| `{service="api"}` lines                           | `apps/api/src/log.rs` via Alloy into Loki                                          | in production only requests that failed, took over 2 s, or found nothing, with their fields (`query_text`, `result_count`, ids)                                                                              |

The Postgres role is ensured by the one-shot `db-monitor-role` job (`scripts/db/monitor-role.sh`) on every `up`, the Elasticsearch user by the search-indexer on every run, so a fresh volume, the existing production volume, and a reseed all end up the same with no manual step.

## Rules the dashboards follow

- Latency is only ever a percentile, never an average and never an average of percentiles: from merged histograms (`sum by` first, then `histogram_quantile`) for the API and Astro, from the raw logged `request_time` values (`quantile_over_time`) for nginx.
- Every percentile has its request count beside it. p95 is shown only with at least 100 requests in the window and p99 with at least 500, so at least five requests sit beyond it; otherwise the cell is empty.
- `/healthz` is left out of every latency and error view: the health probe and Docker call it every few seconds.
- `pg_stat_statements` and Elasticsearch node totals are costs and are labeled "time spent", never latency.
- No data shows as "unknown", never as up. The health dashboard's freshness panel tells a broken pipe from no traffic, and shows whether Loki and Alloy are up.

## Reading it

- **Local:** the full Docker stack (`bun run smoke:stack` brings it up, or `./scripts/dev/compose.sh up -d --build --wait` with the stack environment the smoke runner sets) serves Grafana on `http://127.0.0.1:3300` (`DEV_GRAFANA_PORT`), user `admin`, password `GRAFANA_ADMIN_PASSWORD` (dev default in `scripts/dev/compose.sh`). `bun run dev` runs the API and website on the host without `METRICS_PORT` or an OTLP endpoint, so it records nothing.
- **Production:** `bun run observe` forwards `127.0.0.1:3301` to Grafana on the VPS over the existing SSH tunnel and prints how to read the password from `secrets/prod.enc.env`. Explore searches every container's logs.
- **Changing a dashboard:** edit it in the UI, export the JSON, and commit it here. Provisioned dashboards cannot be saved over from the UI, so git stays the only copy. Use only the variables `$route` and `$status` and Grafana's `$__range`, `$__rate_interval`, and `$__auto`: the stack smoke check substitutes exactly those when it runs every query against its own datasource.

## Tests

`scripts/smoke/observability.ts` runs in the `stack` CI phase after the other suites: every dashboard is provisioned, every Prometheus target is up, the API and website histograms have observations, the exporters report, both health probes succeed, container and host metrics arrive, the website's JSON access log reaches Loki, and every query in every dashboard evaluates against Prometheus or Loki.

## Security and privacy

- Alloy mounts the Docker socket (read-only) and the host's `/`, `/proc`, `/sys`, and `/var/lib/docker` read-only, and joins the host's cgroup namespace: that is how it reads every container's logs and stats. A compromised Alloy would be root-equivalent on the host. It publishes no port and sits only on the `observability` network.
- Loki keeps visitor addresses, request URIs, and search text for 14 days (Docker's own rotation keeps them only until 30 MB of lines per container). Grafana is reached only through the tunnel, behind its login.

## Known limits

- The Latency dashboard's website numbers exclude pages nginx answers from its cache and static files, which never reach Astro; the Edge dashboard covers every request nginx answered. Neither includes the edge gateway's own time or Cloudflare.
- nginx times to the millisecond, so a cache hit often reads 0.
- Counts over a window are Prometheus `increase()` and Loki `count_over_time` estimates; `increase()` extrapolates to the window's edges, so a short burst can read a few percent high.
- `created-timestamp-zero-ingestion` is still marked experimental in Prometheus 3.15.
- Grafana keeps no volume: its datasources and dashboards are provisioned on every start, and a recreated container (a deploy that changes Grafana's image or config) asks for the login again; a plain restart or an unchanged deploy keeps the session.
