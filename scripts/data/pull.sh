#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

if [[ -f .env ]]; then
  set -a
  source .env
  set +a
fi

source ./scripts/data/r2.sh

if [[ $# -eq 0 ]]; then
  if ! data_r2_ready; then
    echo "[data-pull] no R2 token in .env, nothing to download" >&2
    exit 0
  fi
  listing="$(data_r2 "${PWD}" lsf -R --files-only --include '/*/SHA256SUMS' "r2:${DATA_BUCKET}/db/")" || {
    echo "[data-pull] cannot list r2:${DATA_BUCKET}/db/ (check the network and the DATA_* values in .env)" >&2
    exit 1
  }
  newest="$(printf '%s\n' "${listing}" | LC_ALL=C sort | tail -1)"
  if [[ -z "${newest}" ]]; then
    echo "[data-pull] the bucket holds no complete db/ folder" >&2
    exit 1
  fi
  set -- "db/${newest%/SHA256SUMS}"
fi

if [[ $# -ne 1 || "$1" != */* ]]; then
  echo "usage: $0 [<category>/<folder>], for example db/0046_15_10_2026" >&2
  exit 1
fi

category="${1%%/*}"
folder="${1#*/}"
data_check_name "${category}" "${folder}"
data_r2_require

target="data/${category}/${folder}"
if [[ -f "${target}/SHA256SUMS" ]]; then
  echo "[data-pull] using ${category}/${folder}" >&2
  exit 0
fi

if ! data_complete_in_bucket "${category}" "${folder}"; then
  echo "[data-pull] ${category}/${folder} is not complete in the bucket" >&2
  exit 1
fi

download="${target}/.download"
rm -rf "${download}"
mkdir -p "${download}"
trap 'rm -rf "${download}"' EXIT

echo "[data-pull] downloading ${category}/${folder}..." >&2
data_r2 "${PWD}/${target}" copy "r2:${DATA_BUCKET}/${category}/${folder}" /data/.download
if ! (cd "${download}" && sha256sum -c --quiet SHA256SUMS >&2); then
  echo "[data-pull] checksum mismatch in ${category}/${folder}" >&2
  exit 1
fi

find "${download}" -maxdepth 1 -type f ! -name SHA256SUMS -exec mv -f {} "${target}/" \;
mv -f "${download}/SHA256SUMS" "${target}/SHA256SUMS"
echo "[data-pull] using ${category}/${folder}" >&2
