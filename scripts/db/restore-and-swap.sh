#!/usr/bin/env bash
set -euo pipefail

admin_psql() {
  docker compose exec -T -e PSQL_DATABASE="${PSQL_DATABASE:-postgres}" db \
    sh -c 'exec psql -v ON_ERROR_STOP=1 -qtA --username "$POSTGRES_USER" --dbname "$PSQL_DATABASE" "$@"' psql "$@"
}

live="$(docker compose exec -T db printenv POSTGRES_DB | tr -d '[:space:]')"
next="${live}_next"
previous="${live}_previous"

db_bytes="$(admin_psql -c "SELECT pg_database_size('${live}')")"
read -r disk_kb free_kb < <(docker compose exec -T db df -Pk /var/lib/postgresql | awk 'NR == 2 { print $2, $4 }')
needed_kb=$((db_bytes / 1024 + disk_kb / 10))
if ((free_kb < needed_kb)); then
  echo "✗ a restore beside ${live} needs $((needed_kb / 1048576)) GB free (its size, plus 10% of the disk for Elasticsearch's 90% watermark), and the disk has $((free_kb / 1048576)) GB" >&2
  exit 1
fi

trap 'admin_psql -c "DROP DATABASE IF EXISTS ${next}" >/dev/null 2>&1 || true' EXIT
admin_psql -c "DROP DATABASE IF EXISTS ${previous}" -c "DROP DATABASE IF EXISTS ${next}" -c "CREATE DATABASE ${next}"

echo "→ restoring the newest dump into ${next}, while the API keeps serving ${live}"
docker compose exec -T -e POSTGRES_DB="${next}" db bash -s <scripts/db/init.sh
docker compose run --rm --no-deps -e PGDATABASE="${next}" db-monitor-role

poems="$(PSQL_DATABASE="${next}" admin_psql -c 'SELECT count(*) FROM poems')"
if [[ "${poems}" -eq 0 ]]; then
  echo "✗ ${next} has no poems after the restore, so ${live} stays as it is" >&2
  exit 1
fi

echo "→ swapping ${next} (${poems} poems) in for ${live}"
if ! admin_psql >/dev/null <<SQL; then
ALTER DATABASE ${live} WITH ALLOW_CONNECTIONS false;
SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname IN ('${live}', '${next}') AND pid <> pg_backend_pid();
BEGIN;
ALTER DATABASE ${live} RENAME TO ${previous};
ALTER DATABASE ${next} RENAME TO ${live};
COMMIT;
SQL
  admin_psql -c "ALTER DATABASE ${live} WITH ALLOW_CONNECTIONS true"
  echo "✗ the swap failed, and ${live} still serves the old dump" >&2
  exit 1
fi
admin_psql -c "DROP DATABASE ${previous}"
echo "✓ ${live} serves the new dump"
