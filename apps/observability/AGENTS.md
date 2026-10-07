# Observability Agent Guide

This directory is configuration only: the Prometheus, Loki, Alloy, and Grafana setup behind the private views of #137. `docker-compose.yml` defines the services. Nothing here builds.

## Shape

- `prometheus.yml`: Prometheus scrapes every 15 seconds.
  - It scrapes every API replica on its metrics port (`dns_sd_configs` on `api:9464`, with native histograms on).
  - It also scrapes postgres_exporter, elasticsearch_exporter, the `/healthz` probes of `api` and `web` from blackbox_exporter, Alloy's container (cAdvisor) and host (node exporter) components, Loki, and itself.
  - It does not scrape the website. The website pushes over OTLP to the receiver that `--web.enable-otlp-receiver` turns on.
  - `--enable-feature=created-timestamp-zero-ingestion` stores the start time of each series as a zero sample. So a burst that creates a series (the first errors of a route) is counted.
  - Retention is 15 days or 1 GB, whichever comes first.
- `loki.yml`: Loki runs as one binary, on its own volume, with filesystem storage.
  - It keeps logs for 7 days. The compactor deletes older ones.
  - When it stops, it flushes what it holds in memory to disk (`flush_on_shutdown`). Without this, a restart (a deploy that changes Loki, or a reboot) could lose recent logs. Loki's write-ahead log alone loses the newest lines of any stream that was flushed and created again since its last checkpoint. That checkpoint completes about 10 minutes after a start.
  - The write-ahead log still covers a crash.
- `alloy/config.alloy`: Grafana Alloy reads every container of this Compose project through the Docker socket.
  - It filters on `COMPOSE_PROJECT`, so it does not ship a dev stack on the same host a second time.
  - It looks for new containers every 5 seconds, and reads each one from its first line. So it keeps a container that lives about 10 seconds or longer complete. A shorter one-shot, such as `db-monitor-role` under `compose run --rm`, only prints to the terminal.
  - It pushes the logs to Loki with `service` and `container` labels. It sends at most 10 lines a second for each service, beyond a burst of 2,000 (`stage.limit` by `service`).
  - Loki has no retention by size, so this rate limits its disk. A continuous flood on three services stores about 130 MB a day, which is under 1 GB over the 7 days, at the measured compression of 6.6 times.
  - It counts dropped lines for each service (`loki_process_dropped_lines_by_label_total`, on the Health dashboard). The metrics still count every request.
  - It also runs cAdvisor (CPU and memory only, against the limits of each container) and the node exporter (host CPU, memory, and disk). Prometheus scrapes them from `alloy:12345/api/v0/component/...`.
- `grafana/provisioning/` holds the datasources and one dashboard provider.
  - The datasources are Prometheus (uid `prometheus`) and Loki (uid `loki`). The plugins in the pinned Grafana image serve them.
  - `GF_PLUGINS_PREINSTALL_DISABLED` stops Grafana from installing plugins from grafana.com after it starts. Those installs registered these two plugins again, and their queries failed with 404 for some seconds (#198).
  - The dashboard provider loads `grafana/dashboards/` into the `Qafiyah` folder on every start.
  - `alerting/` holds the alert rules, the Telegram contact point, and the notification policy (see "Alerts").
- `grafana/dashboards/`: `latency.json`, `slow-queries.json`, `elasticsearch.json`, `health.json` (the home dashboard), `resources.json`, `edge.json`, and `logs.json`. Each one starts with a text panel that says what it measures and how to read it.

## Where the numbers come from

| Data                                              | Recorded by                                                                        | What it is                                                                                                                                                                                                                                                         |
| ------------------------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `http_server_request_duration_seconds{job="api"}` | `apps/api/src/metrics.rs`, outermost layer                                         | every API request, by method, route template (`unmatched` for none), and status; a native histogram                                                                                                                                                                |
| `http_server_request_duration_seconds{job="web"}` | `apps/web/src/middleware.ts`                                                       | every request that reaches Astro, timed to the end of its body (once, even when the page rewrites to `/404`), by method, route pattern, and status; an OpenTelemetry exponential histogram pushed every 15 s                                                       |
| `db_client_operation_duration_seconds`            | `apps/api/src/es/client.rs`                                                        | every Elasticsearch search the API sends, by alias, failures included                                                                                                                                                                                              |
| `pg_stat_statements_*`                            | postgres_exporter as `qafiyah_monitor`                                             | the 100 statements with the most total time since the last reset, with their text                                                                                                                                                                                  |
| `elasticsearch_*`                                 | elasticsearch_exporter as `qafiyah_monitor`                                        | cluster health, heap, search totals, thread pool rejections                                                                                                                                                                                                        |
| `probe_success`                                   | blackbox_exporter                                                                  | whether `/healthz` answered 2xx                                                                                                                                                                                                                                    |
| `container_*`                                     | cAdvisor in Alloy                                                                  | each container's memory working set and CPU, its limits, and its start time (restarts)                                                                                                                                                                             |
| `node_*`                                          | node exporter in Alloy                                                             | the host's CPU, memory, swap, and disk                                                                                                                                                                                                                             |
| `{service="web"}` nginx lines                     | the website's nginx (`apps/web/nginx.conf`, `log_format json`) via Alloy into Loki | one JSON line per request nginx answered: host, uri, status, `request_time`, `upstream_time`, `cache`                                                                                                                                                              |
| `{service="api"}` lines                           | `apps/api/src/log.rs` via Alloy into Loki                                          | `tracing` JSON; in production only `error` and `warn` lines (a 5xx, a request over 2 s, a failed query or search) and `info` "found nothing" lines, each with its request span (`span_route`, `span_request_id`, `span_query_text`, and the other recorded fields) |

The one-shot `db-monitor-role` job (`scripts/db/monitor-role.sh`) makes sure that the Postgres role exists, on every `up`. The search indexer does the same for the Elasticsearch user, on every run. So a new volume, the existing production volume, and a reseed all get the same result, with no manual step.

## Rules the dashboards follow

- Latency is always a percentile. It is never an average, and never an average of percentiles.
  - For the API and Astro, it comes from merged histograms: `sum by` first, then `histogram_quantile`.
  - For nginx, it comes from the raw logged `request_time` values (`quantile_over_time`).
- Every percentile has its request count next to it. p95 shows only with at least 100 requests in the window, and p99 only with at least 500. So at least five requests are beyond it. Otherwise, the cell is empty.
- Every latency and error view leaves out `/healthz`, because the health probe and Docker call it every few seconds.
- `pg_stat_statements` and the Elasticsearch node totals are costs. They are labeled "time spent", never latency.
- No data shows as "unknown", never as up. The freshness panel of the health dashboard tells a broken pipeline from no traffic, and shows whether Loki and Alloy are up.

## Reading it

- **Local:** the full Docker stack serves Grafana on `http://127.0.0.1:3300` (`DEV_GRAFANA_PORT`). Sign in as `admin`, with the password `GRAFANA_ADMIN_PASSWORD` (the dev default is in `scripts/dev/compose.sh`).
  - `bun run smoke:stack` starts the stack. You can also run `./scripts/dev/compose.sh up -d --build --wait`, with the stack environment that the smoke runner sets.
  - `bun run dev` runs the API and the website on the host, without `METRICS_PORT` or an OTLP endpoint. So it records nothing.
- **Production:** `bun run observe` forwards `127.0.0.1:3301` to Grafana on the VPS, over the existing SSH tunnel. It prints how to read the password from `secrets/prod.enc.env`. Explore searches the logs of every container.
- **To change a dashboard:** edit it in the UI, export the JSON, and commit it here.
  - The UI cannot save over a provisioned dashboard, so git stays the only copy.
  - Use only the variables `$route` and `$status`, and Grafana's `$__range`, `$__rate_interval`, and `$__auto`. The stack smoke check replaces exactly those when it runs every query against its own datasource.

## Alerts

Grafana alerting sends every alert to the Telegram group "Qafiyah Alerts", through the bot `@vigorousJJa_bot`. It sends a message when an alert fires, again every 4 hours while it keeps firing, and once when it resolves. Grafana evaluates the rules every 20 seconds.

| Alert                                 | Fires when                                                                                                                                    | After |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| API down, Website down                | the `/healthz` probe fails                                                                                                                    | 1 min |
| Postgres down                         | `pg_up` is 0                                                                                                                                  | 1 min |
| Elasticsearch unhealthy               | the cluster is unreachable or red                                                                                                             | 1 min |
| Metrics pipeline down                 | Prometheus cannot scrape Alloy, Loki, the probes, or an exporter                                                                              | 1 min |
| Error rate                            | more than 5% of API or website requests answer 5xx over 5 minutes, with at least 20 requests                                                  | 2 min |
| Container memory                      | a container uses more than 90% of its memory limit                                                                                            | 2 min |
| Disk                                  | a host filesystem is more than 85% full                                                                                                       | 2 min |
| API, Search, and Website latency burn | more than 14.4% of requests are slower than the target over both the last hour and the last 5 minutes, with at least 100 requests in the hour | 2 min |

- The latency targets are first guesses: 300 ms for the API outside `/v1/search`, 1 s for `/v1/search`, and 1 s for the website. Tune them in `grafana/provisioning/alerting/rules.yml` after some weeks of production data.
- A rule whose query returns no data stays quiet. "Metrics pipeline down" is the alert for missing data.
- **Credentials:** `TELEGRAM_BOT_TOKEN` and `TELEGRAM_CHAT_ID` exist only in `secrets/prod.enc.env`. The schema refuses them in dev. Without them, `docker-compose.yml` passes placeholders, so a dev or CI stack evaluates the rules but every send fails. Grafana logs Telegram's answer, never the token.
- **Before a reseed:** the reseed stops the API for a few minutes, so silence the alerts first. Open Grafana (`bun run observe`), go to Alerting, then Silences, and add a silence on `severity =~ .+` for 30 minutes. Grafana keeps no volume, so a deploy that recreates it also drops its silences.
- **If the group changes:** Telegram gives a group a new chat ID when it becomes a supergroup (for example, when topics are turned on). Alerts then stop. Read the new ID from the bot's `getUpdates`, and update `TELEGRAM_CHAT_ID` with `bun run secrets:edit prod`.

## Tests

`scripts/smoke/observability.ts` runs in the `stack` CI phase, after the other suites. It checks these things:

- Every dashboard is provisioned.
- Every Prometheus target is up.
- The API and website histograms have observations.
- The exporters report, and both health probes succeed.
- Container and host metrics arrive.
- The website's JSON access log reaches Loki.
- Every query in every dashboard evaluates against Prometheus or Loki.
- Every alert rule is provisioned, and its query evaluates against Prometheus.

## Security and privacy

- Alloy mounts the Docker socket and the host's `/`, `/proc`, `/sys`, and `/var/lib/docker`, all read-only. It also joins the host's cgroup namespace. This is how it reads the logs and stats of every container. So an attacker who takes over Alloy would have the equivalent of root on the host. Alloy publishes no port, and is only on the `observability` network.
- Loki keeps visitor addresses, request URIs, and search text for 7 days. Docker's own rotation keeps them only up to 30 MB of lines for each container. Grafana is reachable only through the tunnel, behind its sign-in.

## Known limits

- The website numbers on the Latency dashboard do not include pages that nginx answers from its cache, or static files. Those never reach Astro. The Edge dashboard covers every request that nginx answered. Neither includes the time of the edge gateway or of Cloudflare.
- nginx measures time in milliseconds, so a cache hit often reads 0.
- Counts over a window are estimates, from Prometheus `increase()` and Loki `count_over_time`. `increase()` extrapolates to the edges of the window, so a short burst can read a few percent high.
- Prometheus 3.15 still marks `created-timestamp-zero-ingestion` as experimental.
- Grafana keeps no volume. It provisions its datasources and dashboards on every start. A recreated container (a deploy that changes Grafana's image or configuration) asks for the sign-in again. A plain restart, or a deploy with no change to Grafana, keeps the session.
