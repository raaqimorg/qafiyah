#!/usr/bin/env bash
set -euo pipefail

source ./scripts/lib/remote.sh

assume_yes=false
case "${1:-}" in
  -y | --yes) assume_yes=true ;;
  "") ;;
  *)
    echo "usage: $0 [-y|--yes]" >&2
    exit 1
    ;;
esac

echo "→ Building api and search-indexer from origin/main, then restoring the newest dump into ${REMOTE_HOST}'s corpus database (the API is down during the restore; qafiyah_accounts is untouched)."
if [[ "$assume_yes" != true ]]; then
  read -r -p "  Continue? [y/N] " reply
  [[ "$reply" =~ ^[Yy]$ ]] || { echo "  aborted"; exit 1; }
fi

remote_exec ./scripts/lib/tag-db-container.sh <<'REMOTE'
set -euo pipefail
cd "$REMOTE_DIR"
git fetch --depth 1 origin main
git reset --hard FETCH_HEAD
./scripts/secrets/pull.sh prod
./scripts/db/resolve-dump.sh
newest=$(find data/db -mindepth 1 -maxdepth 1 -type d ! -name '0000_default' | sort | tail -1)
if ! compgen -G "${newest}/*.dump" >/dev/null && ! compgen -G "${newest}/*.dump.part-??" >/dev/null; then
  echo "✗ ${newest} is still encrypted: add DUMP_KEY__$(basename "${newest}") to secrets/prod.enc.env" >&2
  exit 1
fi
echo "  newest dump: ${newest}"
SENTRY_RELEASE=$(git rev-parse --short HEAD)
export SENTRY_RELEASE
docker compose build api search-indexer
trap 'echo "✗ restore failed, the API is still stopped: fix the cause and rerun bun run db:reseed" >&2' ERR
docker compose stop api
docker compose exec -T db bash /docker-entrypoint-initdb.d/10-restore.sh
docker compose run --rm --no-deps db-monitor-role
docker compose up -d --no-deps --wait api
trap - ERR
docker compose run --rm -e SEARCH_INDEXER_FORCE=true search-indexer
tag_db_container "$(docker compose ps -q db)"
echo ""
echo "=== prod status ==="
docker compose ps
echo ""
echo "✓ restored ${newest} ($(git rev-parse --short HEAD))"
REMOTE

check_public_health
