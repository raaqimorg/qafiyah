#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
# shellcheck source=scripts/lib/remote.sh
. ./scripts/lib/remote.sh

LOCAL_PORT="${OBSERVE_PORT:-3301}"

echo "→ Grafana on ${REMOTE_HOST}: http://127.0.0.1:${LOCAL_PORT} (user admin; Ctrl-C closes the tunnel)"
echo "  password: sops -d --extract '[\"GRAFANA_ADMIN_PASSWORD\"]' secrets/prod.enc.env"
exec ssh -N -o ExitOnForwardFailure=yes -L "127.0.0.1:${LOCAL_PORT}:127.0.0.1:3000" "$REMOTE_HOST"
