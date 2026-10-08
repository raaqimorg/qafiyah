#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

: "${ACCOUNTS_BACKUP_BUCKET:?ACCOUNTS_BACKUP_BUCKET is required}"
: "${ACCOUNTS_BACKUP_RECIPIENT:?ACCOUNTS_BACKUP_RECIPIENT is required}"
: "${ACCOUNTS_BACKUP_R2_ENDPOINT:?ACCOUNTS_BACKUP_R2_ENDPOINT is required}"
: "${ACCOUNTS_BACKUP_R2_ACCESS_KEY_ID:?ACCOUNTS_BACKUP_R2_ACCESS_KEY_ID is required}"
: "${ACCOUNTS_BACKUP_R2_SECRET_ACCESS_KEY:?ACCOUNTS_BACKUP_R2_SECRET_ACCESS_KEY is required}"

name="qafiyah_accounts_$(date -u +%Y%m%dT%H%M%SZ).dump.age"
workdir="$(mktemp -d)"
trap 'rm -rf "${workdir}"' EXIT

echo "[accounts-backup] dumping and encrypting qafiyah_accounts..."
docker compose exec -T db pg_dump --username qafiyah_accounts --dbname qafiyah_accounts \
  --no-owner --no-privileges --format=custom \
  | age --recipient "${ACCOUNTS_BACKUP_RECIPIENT}" --output "${workdir}/${name}"

echo "[accounts-backup] uploading accounts/${name}..."
export RCLONE_CONFIG_R2_ACCESS_KEY_ID="${ACCOUNTS_BACKUP_R2_ACCESS_KEY_ID}"
export RCLONE_CONFIG_R2_SECRET_ACCESS_KEY="${ACCOUNTS_BACKUP_R2_SECRET_ACCESS_KEY}"
docker run --rm \
  --volume "${workdir}:/backup:ro" \
  --env RCLONE_CONFIG_R2_TYPE=s3 \
  --env RCLONE_CONFIG_R2_PROVIDER=Cloudflare \
  --env RCLONE_CONFIG_R2_REGION=auto \
  --env RCLONE_CONFIG_R2_NO_CHECK_BUCKET=true \
  --env RCLONE_CONFIG_R2_ENDPOINT="${ACCOUNTS_BACKUP_R2_ENDPOINT}" \
  --env RCLONE_CONFIG_R2_ACCESS_KEY_ID \
  --env RCLONE_CONFIG_R2_SECRET_ACCESS_KEY \
  rclone/rclone:1.75.1 copyto "/backup/${name}" "r2:${ACCOUNTS_BACKUP_BUCKET}/accounts/${name}"

metrics_dir=".metrics"
mkdir -p "${metrics_dir}"
cat > "${metrics_dir}/accounts-backup.prom.tmp" <<EOF
# HELP qafiyah_accounts_backup_last_success_timestamp_seconds Unix time of the last successful accounts backup.
# TYPE qafiyah_accounts_backup_last_success_timestamp_seconds gauge
qafiyah_accounts_backup_last_success_timestamp_seconds $(date -u +%s)
EOF
mv -f "${metrics_dir}/accounts-backup.prom.tmp" "${metrics_dir}/accounts-backup.prom"

echo "[accounts-backup] done"
