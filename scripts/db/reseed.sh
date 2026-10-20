#!/usr/bin/env bash
set -euo pipefail

source ./scripts/lib/remote.sh
source ./scripts/data/r2.sh

usage() {
  echo "usage: $0 [-y|--yes] <dump-folder>, for example 0047_25_10_2026" >&2
  exit 1
}

assume_yes=false
folder=""
for arg in "$@"; do
  case "${arg}" in
    -y | --yes) assume_yes=true ;;
    *)
      [[ -z "${folder}" ]] || usage
      folder="${arg}"
      ;;
  esac
done
[[ -n "${folder}" ]] || usage
data_check_name db "${folder}"

echo "→ Building api and search-indexer from origin/main, then restoring ${folder} beside ${REMOTE_HOST}'s corpus database and swapping it in (the API keeps serving; qafiyah_accounts is untouched)."
if [[ "$assume_yes" != true ]]; then
  read -r -p "  Continue? [y/N] " reply
  [[ "$reply" =~ ^[Yy]$ ]] || { echo "  aborted"; exit 1; }
fi

{
  printf 'folder=%q\n' "${folder}"
  cat <<'REMOTE'
set -euo pipefail
cd "$REMOTE_DIR"
git fetch --depth 1 origin main
git reset --hard FETCH_HEAD
./scripts/secrets/pull.sh prod
./scripts/db/use-dump.sh "${folder}"
SENTRY_RELEASE=$(git rev-parse --short HEAD)
export SENTRY_RELEASE
docker compose build api search-indexer
./scripts/db/restore-and-swap.sh
rollout api
docker compose run --rm -e SEARCH_INDEXER_FORCE=true search-indexer
tag_db_container "$(docker compose ps -q db)"
echo ""
echo "=== prod status ==="
docker compose ps
echo ""
echo "✓ restored ${folder} ($(git rev-parse --short HEAD))"
REMOTE
} | remote_exec ./scripts/lib/tag-db-container.sh ./scripts/lib/rollout.sh

check_public_health
