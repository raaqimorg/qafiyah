#!/usr/bin/env bash
set -euo pipefail

dump=""
for dump_dir in $(find /dumps -mindepth 1 -maxdepth 1 -type d | sort -r); do
  candidate="$(find "${dump_dir}" -maxdepth 1 -name '*.dump' -type f | head -1)"
  if [[ -n "${candidate}" ]]; then
    dump="${candidate}"
    break
  fi

  first_part="$(find "${dump_dir}" -maxdepth 1 -name '*.dump.part-*' ! -name '*.enc' -type f | sort | head -1)"
  if [[ -n "${first_part}" ]]; then
    dump="/tmp/$(basename "${first_part%.part-*}")"
    echo "[db-init] reassembling split dump from ${dump_dir}..."
    find "${dump_dir}" -maxdepth 1 -name '*.dump.part-*' ! -name '*.enc' -type f | sort | xargs cat >"${dump}"
    trap 'rm -f "${dump}"' EXIT
    break
  fi
done

if [[ -z "${dump}" ]]; then
  echo "[db-init] no usable dump found in /dumps, starting with an empty database" >&2
  exit 0
fi

dump_name="$(basename "${dump_dir}")"
dump_number="${dump_name%%_*}"

echo "[db-init] restoring ${dump} into ${POSTGRES_DB}..."
psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" \
  -c 'DROP SCHEMA IF EXISTS public CASCADE;'
pg_restore --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" \
  --format=custom --no-owner --no-acl "${dump}"

psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" \
  -c 'CREATE EXTENSION IF NOT EXISTS pg_stat_statements;'

echo "[db-init] ensuring refresh_poem_relations() exists..."
psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" \
  -f /docker-entrypoint-initdb.d/sql/refresh-poem-relations.sql

echo "[db-init] ensuring poem_aliases exists..."
psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" \
  -f /docker-entrypoint-initdb.d/sql/poem-aliases.sql

echo "[db-init] ensuring merge_poem() exists..."
psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" \
  -f /docker-entrypoint-initdb.d/sql/merge-poem.sql

echo "[db-init] ensuring poems.recension_of_id exists..."
psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" \
  -f /docker-entrypoint-initdb.d/sql/poem-recensions.sql

echo "[db-init] ensuring poet_aliases exists..."
psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" \
  -f /docker-entrypoint-initdb.d/sql/poet-aliases.sql

echo "[db-init] ensuring merge_poet() exists..."
psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" \
  -f /docker-entrypoint-initdb.d/sql/merge-poet.sql

echo "[db-init] ensuring the taxonomy stats tables exist and match the restored data..."
psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" \
  -f /docker-entrypoint-initdb.d/sql/refresh-taxonomy-stats.sql
psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" \
  -c 'SELECT public.refresh_taxonomy_stats();'

if [[ -n "${PG_READER_PASSWORD:-}" ]]; then
  echo "[db-init] provisioning read-only role qafiyah_api..."
  psql -v ON_ERROR_STOP=1 -v reader_pw="${PG_READER_PASSWORD}" \
    --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" <<'SQL'
SELECT 'CREATE ROLE qafiyah_api LOGIN'
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'qafiyah_api')
\gexec
ALTER ROLE qafiyah_api WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS PASSWORD :'reader_pw';
REVOKE ALL ON SCHEMA public FROM qafiyah_api;
GRANT USAGE ON SCHEMA public TO qafiyah_api;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO qafiyah_api;
DO $do$
DECLARE r record;
BEGIN
  FOR r IN
    SELECT c.relname
    FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE n.nspname = 'public' AND c.relkind = 'r' AND c.relrowsecurity
  LOOP
    EXECUTE format('DROP POLICY IF EXISTS qafiyah_api_read ON public.%I', r.relname);
    EXECUTE format('CREATE POLICY qafiyah_api_read ON public.%I FOR SELECT TO qafiyah_api USING (true)', r.relname);
  END LOOP;
END
$do$;
SQL
  psql -v ON_ERROR_STOP=1 -v dbname="${POSTGRES_DB}" \
    --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" <<'SQL'
REVOKE CONNECT ON DATABASE :"dbname" FROM PUBLIC;
GRANT CONNECT ON DATABASE :"dbname" TO qafiyah_api;
SQL
else
  echo "[db-init] PG_READER_PASSWORD unset; skipping read-only role provisioning" >&2
fi

echo "[db-init] tagging database with dump ${dump_number}..."
psql -v ON_ERROR_STOP=1 -v dbname="${POSTGRES_DB}" -v dump_number="${dump_number}" \
  --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" <<'SQL'
COMMENT ON DATABASE :"dbname" IS :'dump_number';
SQL

echo "[db-init] restore complete"
