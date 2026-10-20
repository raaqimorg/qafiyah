#!/usr/bin/env bash

rollout() {
  svc="$1"
  old=$(docker compose ps -q "$svc")
  n_old=$(printf '%s\n' "$old" | grep -c . || true)
  if [ "$n_old" -lt 1 ]; then
    echo "→ starting $svc (nothing of it was running)"
    docker compose up -d --no-deps --wait "$svc"
    return
  fi
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
