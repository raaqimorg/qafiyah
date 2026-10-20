#!/usr/bin/env bash
set -euo pipefail

source ./scripts/lib/remote.sh

refuse() {
  echo "✗ $1" >&2
  exit 1
}

release_run_with_same_files() {
  local tree sha
  tree="$(git rev-parse "${target_sha}^{tree}")"
  while read -r sha; do
    if [[ "$(git rev-parse --quiet --verify "${sha}^{tree}" 2>/dev/null || true)" == "${tree}" ]]; then
      echo "${sha}"
      return 0
    fi
  done < <(gh run list --workflow ci.yml --event pull_request --status success --limit 50 \
    --json headSha,headBranch --jq '.[] | select(.headBranch | test("^v[0-9]+$")) | .headSha' 2>/dev/null || true)
  return 1
}

if [[ "${1:-}" == --skip-ci-check ]]; then
  echo "⚠ skipping the GitHub CI check, deploying origin/main unverified"
else
  command -v gh >/dev/null || refuse "the GitHub CLI (gh) is needed to confirm CI passed; pass --skip-ci-check to deploy anyway"
  git fetch --quiet origin
  target_sha="$(git rev-parse origin/main)"
  ci_state="$(gh run list --workflow ci.yml --commit "${target_sha}" --event push --limit 1 \
    --json status,conclusion --jq '.[0] | "\(.status) \(.conclusion)"' 2>/dev/null || true)"
  case "${ci_state}" in
    "completed success") echo "✓ GitHub CI passed for ${target_sha:0:7}" ;;
    completed*) refuse "GitHub CI did not pass for ${target_sha:0:7} (${ci_state#completed }); fix main before deploying" ;;
    *)
      if tested_sha="$(release_run_with_same_files)"; then
        echo "✓ GitHub CI passed for the release pull request at ${tested_sha:0:7}, which has the same files as ${target_sha:0:7}"
      elif [[ "${ci_state}" == "" || "${ci_state}" == "null null" ]]; then
        refuse "no GitHub CI run found for origin/main (${target_sha:0:7}); push first and let it run"
      else
        refuse "GitHub CI is still running for ${target_sha:0:7}; deploy once it passes (gh run watch)"
      fi
      ;;
  esac
fi

echo "→ Deploying origin/main to ${REMOTE_HOST} (zero-downtime) ..."

remote_exec ./scripts/lib/reclaimable.sh ./scripts/lib/tag-db-container.sh ./scripts/lib/rollout.sh <<'REMOTE'
set -euo pipefail
cd "$REMOTE_DIR"

git fetch --depth 1 origin main
current=$(git rev-parse --short HEAD)
target=$(git rev-parse --short FETCH_HEAD)
deployed=$(git rev-parse --verify --quiet refs/deployed || git rev-parse HEAD)
echo "  current ${current}  ->  target ${target}"
git reset --hard FETCH_HEAD
./scripts/secrets/pull.sh prod

export SENTRY_RELEASE="$target"

docker compose build api web search-indexer mcp

docker compose up -d --no-deps db elasticsearch search-indexer edge-gateway db-monitor-role postgres-exporter elasticsearch-exporter blackbox-exporter prometheus loki alloy grafana
changed_config=$(git diff --name-only "$deployed" HEAD -- apps/edge-gateway apps/observability) ||
  changed_config="apps/edge-gateway/proxy_backend.conf.template apps/observability/prometheus.yml apps/observability/loki.yml apps/observability/alloy/ apps/observability/grafana/"
reload_edge=false
recreate=()
for path in $changed_config; do
  case "$path" in
    apps/edge-gateway/proxy_backend.conf.template) reload_edge=true ;;
    apps/observability/prometheus.yml) recreate+=(prometheus) ;;
    apps/observability/loki.yml) recreate+=(loki) ;;
    apps/observability/alloy/*) recreate+=(alloy) ;;
    apps/observability/grafana/*) recreate+=(grafana) ;;
  esac
done
if [[ "$reload_edge" == true ]]; then
  echo "→ applying the changed edge template with a graceful nginx reload"
  edge=$(docker compose ps -q edge-gateway)
  docker exec "$edge" mkdir -p /tmp/edge-templates/includes
  docker cp apps/edge-gateway/proxy_backend.conf.template "$edge:/tmp/edge-templates/includes/proxy_backend.conf.template"
  docker exec "$edge" sh -c 'cp /etc/nginx/includes/proxy_backend.conf /tmp/proxy_backend.conf.live &&
    NGINX_ENVSUBST_TEMPLATE_DIR=/tmp/edge-templates /docker-entrypoint.d/20-envsubst-on-templates.sh >/dev/null &&
    /docker-entrypoint.d/92-update-real_ip.sh &&
    nginx -t -q && nginx -s reload 2>/dev/null ||
    { cp /tmp/proxy_backend.conf.live /etc/nginx/includes/proxy_backend.conf; echo "✗ the new edge template failed nginx -t, so the old one stays live" >&2; exit 1; }'
fi
if ((${#recreate[@]} > 0)); then
  mapfile -t recreate < <(printf '%s\n' "${recreate[@]}" | sort -u)
  echo "→ recreating the services whose mounted configuration changed since ${deployed:0:7}: ${recreate[*]}"
  docker compose up -d --no-deps --force-recreate "${recreate[@]}"
fi
tag_db_container "$(docker compose ps -q db)"

rollout api
rollout mcp
rollout web

echo ""
echo "→ edge smoke test (through the edge gateway on 127.0.0.1:80 to the API and database)"
curl -fsS -H 'Host: qafiyah.com' 'http://127.0.0.1:80/api/v1/poems/random?option=slug' -o /dev/null
echo "  apex search proxy ok"
curl -fsS -H 'Host: api.qafiyah.com' 'http://127.0.0.1:80/v1/poems/random?option=slug' -o /dev/null
echo "  api  random poem  ok"
curl -fsS -H 'Host: mcp.qafiyah.com' -H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream' \
  --data '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' 'http://127.0.0.1:80/mcp' -o /dev/null
echo "  mcp  tools list   ok"

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
git update-ref refs/deployed HEAD
echo "✓ deployed $(git rev-parse --short HEAD)"

echo ""
echo "→ capping build cache at 5GB (keeps recent layers for fast rebuilds)"
docker builder prune -f --max-used-space 5GB 2>&1 | tail -1 || true

print_reclaimable
REMOTE

check_public_health
