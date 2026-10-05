#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

./scripts/db/resolve-dump.sh

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

worktree_flag=false
args=()
for arg in "$@"; do
  case "$arg" in
    --worktree | -w) worktree_flag=true ;;
    *) args+=("$arg") ;;
  esac
done

in_linked_worktree() {
  local git_dir common_dir
  git_dir="$(git rev-parse --path-format=absolute --git-dir 2>/dev/null)" || return 1
  common_dir="$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" || return 1
  [[ -n "${git_dir}" && "${git_dir}" != "${common_dir}" ]]
}

if [[ "${worktree_flag}" == false ]] && in_linked_worktree; then
  worktree_flag=true
fi

if [[ "${worktree_flag}" == true ]]; then
  env_output="$(bun scripts/dev/worktree.ts --env)" || exit 1
  set -a
  eval "$env_output"
  set +a
else
  export COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-qafiyah-dev}"
fi

exec docker compose -f docker-compose.yml -f docker-compose.dev.yml "${args[@]}"
