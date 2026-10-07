#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <prod-db-host>" >&2
  exit 1
fi

host="$1"
user="${POSTGRES_USER:-qafiyah}"
db="${POSTGRES_DB:-qafiyah}"

echo "[create-dump] refreshing poems.has_tashkeel on ${host}..."
psql -v ON_ERROR_STOP=1 -h "${host}" -U "${user}" -d "${db}" \
  -c 'SELECT public.refresh_poem_tashkeel();'

echo "[create-dump] refreshing poem_relations on ${host}..."
psql -v ON_ERROR_STOP=1 -h "${host}" -U "${user}" -d "${db}" \
  -c 'SELECT public.refresh_poem_relations();'

echo "[create-dump] refreshing the taxonomy stats tables on ${host}..."
psql -v ON_ERROR_STOP=1 -h "${host}" -U "${user}" -d "${db}" \
  -c 'SELECT public.refresh_taxonomy_stats();'

out="qafiyah_public_$(date +%Y%m%d_%H%M%S).dump"
echo "[create-dump] dumping ${db}@${host} to ${out}..."
pg_dump -h "${host}" -p 5432 -U "${user}" -d "${db}" \
  --schema=public --no-owner --no-privileges --no-tablespaces \
  -Fc -f "${out}"

echo "[create-dump] wrote ${out}"
