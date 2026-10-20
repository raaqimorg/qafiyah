#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <dump-folder>, for example 0046_15_10_2026" >&2
  exit 1
fi

folder="$1"
source ./scripts/data/r2.sh
data_check_name db "${folder}"

./scripts/data/pull.sh "db/${folder}"

for dir in data/db/*/; do
  name="$(basename "${dir}")"
  [[ "${name}" == "0000_default" || "${name}" == "${folder}" ]] && continue
  git clean -fdxq -- "${dir}"
  rmdir "${dir}" 2>/dev/null || true
done

echo "[use-dump] data/db holds only ${folder}" >&2
