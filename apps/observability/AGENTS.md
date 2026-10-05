# Observability Agent Guide

Config only: the Prometheus and Grafana setup behind the four private views of #137 (latency per route, slow queries, Elasticsearch, health). The services themselves are defined in `docker-compose.yml`; nothing here builds.

## Shape

- `prometheus.yml`: scrapes every API replica on its metrics port (`dns_sd_configs` on `api:9464`, native histograms on), postgres_exporter, elasticsearch_exporter, blackbox_exporter's `/healthz` probes of `api` and `web`, and itself, every 15 s. The website is not scraped: it pushes over OTLP to the receiver Prometheus enables with `--web.enable-otlp-receiver`. Retention is 15 days or 1 GB, whichever comes first.
- `grafana/provisioning/`: one Prometheus datasource (uid `prometheus`) and one dashboard provider that loads `grafana/dashboards/` into the `Qafiyah` folder on every start.
- `grafana/dashboards/`: `latency.json`, `slow-queries.json`, `elasticsearch.json`, `health.json` (the home dashboard). Each opens with a text panel saying what it measures and how to read it.

## Where the numbers come from

| Metric                                            | Recorded by                                 | What it is                                                                                                                                                     |
| ------------------------------------------------- | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `http_server_request_duration_seconds{job="api"}` | `apps/api/src/metrics.rs`, outermost layer  | every API request, by method, route template (`unmatched` for none), and status; a native histogram                                                            |
| `http_server_request_duration_seconds{job="web"}` | `apps/web/src/middleware.ts`                | every request that reaches Astro, timed to the end of its body, by method, route pattern, and status; an OpenTelemetry exponential histogram pushed every 15 s |
| `db_client_operation_duration_seconds`            | `apps/api/src/es/client.rs`                 | every Elasticsearch search the API sends, by alias, failures included                                                                                          |
| `pg_stat_statements_*`                            | postgres_exporter as `qafiyah_monitor`      | the 100 statements with the most total time since the last reset, with their text                                                                              |
| `elasticsearch_*`                                 | elasticsearch_exporter as `qafiyah_monitor` | cluster health, heap, search totals, thread pool rejections                                                                                                    |
| `probe_success`                                   | blackbox_exporter                           | whether `/healthz` answered 2xx                                                                                                                                |

The Postgres role is ensured by the one-shot `db-monitor-role` job (`scripts/db/monitor-role.sh`) on every `up`, the Elasticsearch user by the search-indexer on every run, so a fresh volume, the existing production volume, and a reseed all end up the same with no manual step.

## Rules the dashboards follow

- Latency is only ever a percentile computed from merged histograms when the panel is drawn (`sum by` first, then `histogram_quantile`), never an average and never an average of percentiles.
- Every percentile has its request count beside it. p95 is shown only with at least 100 requests in the window and p99 with at least 500, so at least five requests sit beyond it; otherwise the cell is empty.
- `pg_stat_statements` and Elasticsearch node totals are costs and are labeled "time spent", never latency.
- No data shows as "unknown", never as up. The health dashboard's freshness panel tells a broken pipe from no traffic.

## Reading it

- **Local:** the full Docker stack (`bun run smoke:stack` brings it up, or `./scripts/dev/compose.sh up -d --build --wait` with the stack environment the smoke runner sets) serves Grafana on `http://127.0.0.1:3300` (`DEV_GRAFANA_PORT`), user `admin`, password `GRAFANA_ADMIN_PASSWORD` (dev default in `scripts/dev/compose.sh`). `bun run dev` runs the API and website on the host without `METRICS_PORT` or an OTLP endpoint, so it records nothing.
- **Production:** `bun run observe` forwards `127.0.0.1:3301` to Grafana on the VPS over the existing SSH tunnel and prints how to read the password from `secrets/prod.enc.env`.
- **Changing a dashboard:** edit it in the UI, export the JSON, and commit it here. Provisioned dashboards cannot be saved over from the UI, so git stays the only copy. Use only the variables `$route` and `$status` and Grafana's `$__range` and `$__rate_interval`: the stack smoke check substitutes exactly those when it runs every query.

## Tests

`scripts/smoke/observability.ts` runs in the `stack` CI phase after the other suites: the four dashboards are provisioned, every Prometheus target is up, the API and website histograms have observations, the exporters report, both health probes succeed, and every query in every dashboard evaluates.

## Known limits

- Website numbers exclude pages nginx answers from its cache and static files, which never reach Astro.
- Counts over a window are Prometheus `increase()` estimates: a new series (a route's first request on a replica) counts from its first scrape or push, so up to 15 s of its first observations are not in the window's count.
- Grafana keeps no volume: its datasource and dashboards are provisioned on every start, and a recreated container (every deploy) asks for the login again.
