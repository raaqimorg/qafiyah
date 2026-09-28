#!/usr/bin/env bash

REMOTE_HOST="${REMOTE_HOST:-qafiyah}"
REMOTE_DIR="${REMOTE_DIR:-/opt/qafiyah}"
PUBLIC_SITE_URL="${PUBLIC_SITE_URL:-https://qafiyah.com}"
PUBLIC_API_URL="${PUBLIC_API_URL:-https://api.qafiyah.com}"

remote_exec() {
  {
    for helper in "$@"; do cat "$helper"; done
    printf 'REMOTE_DIR=%q\n' "$REMOTE_DIR"
    cat
  } | ssh "$REMOTE_HOST" 'script=$(mktemp) && cat >"$script" && bash "$script" </dev/null; status=$?; rm -f "$script"; exit "$status"'
}

check_public_health() {
  echo ""
  echo "→ public health check (through Cloudflare to the API and database)"
  local url
  for url in "${PUBLIC_SITE_URL}/api/v1/poems/random?option=slug" "${PUBLIC_API_URL}/v1/poems/random?option=slug"; do
    if ! curl -fsS --retry 5 --retry-all-errors --retry-delay 3 --max-time 15 -o /dev/null "$url"; then
      echo "✗ ${url} is not answering 200: check the stack now (docker compose ps on ${REMOTE_HOST})" >&2
      return 1
    fi
    echo "  ${url} ok"
  done
}
