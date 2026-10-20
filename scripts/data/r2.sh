#!/usr/bin/env bash

DATA_RCLONE_IMAGE="rclone/rclone:1.75.1"
DATA_KEYS=(DATA_R2_ENDPOINT DATA_BUCKET DATA_R2_ACCESS_KEY_ID DATA_R2_SECRET_ACCESS_KEY)

data_r2_ready() {
  local key
  for key in "${DATA_KEYS[@]}"; do
    [[ -n "${!key:-}" ]] || return 1
  done
}

data_r2_require() {
  local key
  for key in "${DATA_KEYS[@]}"; do
    if [[ -z "${!key:-}" ]]; then
      echo "[data] ${key} is required (set it in .env)" >&2
      exit 1
    fi
  done
}

data_check_name() {
  if [[ ! "$1" =~ ^(db|avatars|feelings)$ || ! "$2" =~ ^[0-9]{4}_[0-9]{2}_[0-9]{2}_[0-9]{4}$ ]]; then
    echo "[data] not a snapshot name: $1/$2" >&2
    exit 1
  fi
}

data_r2() {
  local host_dir="$1"
  shift
  RCLONE_CONFIG_R2_ACCESS_KEY_ID="${DATA_R2_ACCESS_KEY_ID}" \
    RCLONE_CONFIG_R2_SECRET_ACCESS_KEY="${DATA_R2_SECRET_ACCESS_KEY}" \
    docker run --rm \
    --user "$(id -u):$(id -g)" \
    --volume "${host_dir}:/data" \
    --env HOME=/tmp \
    --env RCLONE_CONFIG=/dev/null \
    --env RCLONE_CONFIG_R2_TYPE=s3 \
    --env RCLONE_CONFIG_R2_PROVIDER=Cloudflare \
    --env RCLONE_CONFIG_R2_REGION=auto \
    --env RCLONE_CONFIG_R2_NO_CHECK_BUCKET=true \
    --env RCLONE_CONFIG_R2_ENDPOINT="${DATA_R2_ENDPOINT}" \
    --env RCLONE_CONFIG_R2_ACCESS_KEY_ID \
    --env RCLONE_CONFIG_R2_SECRET_ACCESS_KEY \
    "${DATA_RCLONE_IMAGE}" "$@"
}

data_complete_in_bucket() {
  local listing
  listing="$(data_r2 "${PWD}" lsf --files-only --include /SHA256SUMS "r2:${DATA_BUCKET}/$1/$2/")" || {
    echo "[data] cannot list r2:${DATA_BUCKET}/$1/$2/ (check the network and the DATA_* values in .env)" >&2
    exit 1
  }
  [[ "${listing}" == "SHA256SUMS" ]]
}
