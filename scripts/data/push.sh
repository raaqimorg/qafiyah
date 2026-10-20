#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || ! -d "$1" ]]; then
  echo "usage: $0 <snapshot-folder>, for example data/db/0047_25_10_2026" >&2
  exit 1
fi

source_dir="$(cd "$1" && pwd)"
folder="$(basename "${source_dir}")"
category="$(basename "$(dirname "${source_dir}")")"

cd "$(dirname "$0")/../.."

if [[ -f .env ]]; then
  set -a
  source .env
  set +a
fi

source ./scripts/data/r2.sh

data_check_name "${category}" "${folder}"
data_r2_require

if data_complete_in_bucket "${category}" "${folder}"; then
  echo "[data-push] ${category}/${folder} is already complete in the bucket" >&2
  exit 1
fi

mapfile -t files < <(cd "${source_dir}" && find . -maxdepth 1 -type f ! -name SHA256SUMS ! -name '.*' | sed 's|^\./||' | LC_ALL=C sort)
if [[ ${#files[@]} -eq 0 ]]; then
  echo "[data-push] ${source_dir} holds no files" >&2
  exit 1
fi
(cd "${source_dir}" && sha256sum "${files[@]}" >SHA256SUMS)

echo "[data-push] uploading ${category}/${folder}..." >&2
data_r2 "${source_dir}" copy --immutable --exclude SHA256SUMS --exclude '.*' /data "r2:${DATA_BUCKET}/${category}/${folder}"
data_r2 "${source_dir}" copyto /data/SHA256SUMS "r2:${DATA_BUCKET}/${category}/${folder}/SHA256SUMS"
echo "[data-push] uploaded ${category}/${folder}" >&2
