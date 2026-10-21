#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

dev_default() {
  local name="$1" value="$2"
  [[ -n "${!name:-}" ]] && return 0
  [[ -f .env ]] && grep -qE "^[[:space:]]*(export[[:space:]]+)?${name}=" .env && return 0
  export "${name}=${value}"
}
dev_default POSTGRES_PASSWORD qafiyah
dev_default PG_READER_PASSWORD qafiyah-dev-pg-reader
dev_default PG_ACCOUNTS_PASSWORD qafiyah-dev-pg-accounts
dev_default ELASTIC_PASSWORD qafiyah-dev-es
dev_default ES_READER_PASSWORD qafiyah-dev-reader
dev_default PG_MONITOR_PASSWORD qafiyah-dev-pg-monitor
dev_default ES_MONITOR_PASSWORD qafiyah-dev-es-monitor
dev_default GRAFANA_ADMIN_PASSWORD qafiyah-dev-grafana

export COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-qafiyah-dev}"

files=(-f docker-compose.yml -f docker-compose.dev.yml)
[[ -n "${COMPOSE_CI_FILE:-}" ]] && files+=(-f "${COMPOSE_CI_FILE}")

exec docker compose "${files[@]}" "$@"
