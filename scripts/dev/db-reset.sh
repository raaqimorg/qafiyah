#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

if [[ $# -eq 2 && "$1" == "--dump" ]]; then
  source ./scripts/data/r2.sh
  data_check_name db "$2"
  export DEV_DUMP="$2"
  compgen -G "data/db/${DEV_DUMP}/*.dump*" >/dev/null || ./scripts/data/pull.sh "db/${DEV_DUMP}"
elif [[ $# -ne 0 ]]; then
  echo "usage: $0 [--dump <folder>], for example --dump 0046_15_10_2026" >&2
  exit 1
fi

./scripts/dev/compose.sh down -v
./scripts/dev/db-up.sh
