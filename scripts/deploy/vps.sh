#!/usr/bin/env bash
set -euo pipefail

source ./scripts/lib/remote.sh

refuse() {
  echo "✗ $1" >&2
  exit 1
}

if [[ "${1:-}" == --skip-ci-check ]]; then
  echo "⚠ skipping the GitHub CI check, deploying origin/main unverified"
else
  command -v gh >/dev/null || refuse "the GitHub CLI (gh) is needed to confirm CI passed; pass --skip-ci-check to deploy anyway"
  target_sha="$(git ls-remote origin refs/heads/main | cut -f1)"
  ci_state="$(gh run list --workflow ci.yml --commit "${target_sha}" --event push --limit 1 \
    --json status,conclusion --jq '.[0] | "\(.status) \(.conclusion)"' 2>/dev/null || true)"
  case "${ci_state}" in
    "completed success") echo "✓ GitHub CI passed for ${target_sha:0:7}" ;;
    "" | "null null") refuse "no GitHub CI run found for origin/main (${target_sha:0:7}); push first and let it run" ;;
    completed*) refuse "GitHub CI did not pass for ${target_sha:0:7} (${ci_state#completed }); fix main before deploying" ;;
    *) refuse "GitHub CI is still running for ${target_sha:0:7}; deploy once it passes (gh run watch)" ;;
  esac
fi

echo "→ Deploying origin/main to ${REMOTE_HOST} (zero-downtime) ..."

remote_exec ./scripts/lib/reclaimable.sh ./scripts/lib/tag-db-container.sh <<'REMOTE'
set -euo pipefail
cd "$REMOTE_DIR"

rollout() {
  svc="$1"
  old=$(docker compose ps -q "$svc")
  n_old=$(printf '%s\n' "$old" | grep -c .)
  [ "$n_old" -ge 1 ] || { echo "✗ rollout $svc: nothing running"; exit 1; }
  target=$((2 * n_old))
  echo "→ rolling $svc ($n_old -> $target -> $n_old)"
  docker compose up -d --no-deps --no-recreate --scale "$svc=$target" "$svc"
  deadline=$((SECONDS + 150))
  while :; do
    ids=$(docker compose ps -q "$svc"); n=0; h=0
    for id in $ids; do
      n=$((n + 1))
      s=$(docker inspect -f '{{.State.Health.Status}}' "$id" 2>/dev/null || echo none)
      [ "$s" = healthy ] && h=$((h + 1))
    done
    [ "$n" -ge "$target" ] && [ "$h" -eq "$n" ] && break
    [ "$SECONDS" -ge "$deadline" ] && { echo "✗ rollout $svc timeout ($h/$n healthy)"; docker compose logs --tail=40 "$svc"; exit 1; }
    sleep 2
  done
  echo "  $svc: new replica(s) healthy → settling, then draining old"
  sleep 7
  docker stop $old >/dev/null && docker rm $old >/dev/null
}

git fetch --depth 1 origin main
current=$(git rev-parse --short HEAD)
target=$(git rev-parse --short FETCH_HEAD)
echo "  current ${current}  ->  target ${target}"
git reset --hard FETCH_HEAD
./scripts/secrets/pull.sh prod

export SENTRY_RELEASE="$target"

docker compose build api web search-indexer

docker compose up -d --no-deps db elasticsearch search-indexer edge-gateway db-monitor-role postgres-exporter elasticsearch-exporter blackbox-exporter prometheus grafana
tag_db_container "$(docker compose ps -q db)"

rollout api
rollout web

echo ""
echo "→ edge smoke test (through the edge gateway on 127.0.0.1:80 to the API and database)"
curl -fsS -H 'Host: qafiyah.com' 'http://127.0.0.1:80/api/v1/poems/random?option=slug' -o /dev/null
echo "  apex search proxy ok"
curl -fsS -H 'Host: api.qafiyah.com' 'http://127.0.0.1:80/v1/poems/random?option=slug' -o /dev/null
echo "  api  random poem  ok"

echo ""
echo "→ purging the Cloudflare cache (cached pages still point at the previous build's scripts)"
zone_id=$(grep -E '^CLOUDFLARE_ZONE_ID=' .env | cut -d= -f2- || true)
purge_token=$(grep -E '^CLOUDFLARE_CACHE_PURGE_TOKEN=' .env | cut -d= -f2- || true)
purge_result=""
if [ -n "$zone_id" ] && [ -n "$purge_token" ]; then
  purge_result=$(curl -sS -X POST "https://api.cloudflare.com/client/v4/zones/${zone_id}/purge_cache" \
    -H "Authorization: Bearer ${purge_token}" -H 'Content-Type: application/json' \
    --data '{"purge_everything":true}' || true)
fi
case "$purge_result" in
  *'"success":true'*) echo "  cloudflare cache purged" ;;
  *)
    echo "✗ Cloudflare purge failed: purge everything in the Cloudflare dashboard now, or cached pages will load scripts this build no longer has" >&2
    echo "  response: ${purge_result:-none (CLOUDFLARE_ZONE_ID or CLOUDFLARE_CACHE_PURGE_TOKEN missing from .env)}" >&2
    exit 1
    ;;
esac

echo ""
echo "=== prod status ==="
docker compose ps
echo ""
echo "✓ deployed $(git rev-parse --short HEAD)"

echo ""
echo "→ capping build cache at 5GB (keeps recent layers for fast rebuilds)"
docker builder prune -f --max-used-space 5GB 2>&1 | tail -1 || true

print_reclaimable
REMOTE

check_public_health
