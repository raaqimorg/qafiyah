#!/usr/bin/env bash
set -euo pipefail

: "${PG_MONITOR_PASSWORD:?PG_MONITOR_PASSWORD is required}"
: "${PGDATABASE:?PGDATABASE is required}"

psql -v ON_ERROR_STOP=1 -v monitor_pw="${PG_MONITOR_PASSWORD}" -v dbname="${PGDATABASE}" <<'SQL'
SELECT 'CREATE ROLE qafiyah_monitor LOGIN'
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'qafiyah_monitor')
\gexec
ALTER ROLE qafiyah_monitor WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS PASSWORD :'monitor_pw';
GRANT pg_monitor TO qafiyah_monitor;
GRANT CONNECT ON DATABASE :"dbname" TO qafiyah_monitor;
GRANT USAGE ON SCHEMA public TO qafiyah_monitor;
SQL

echo "[monitor-role] qafiyah_monitor is ready"
