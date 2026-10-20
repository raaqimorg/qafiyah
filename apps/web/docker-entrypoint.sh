#!/bin/sh
for name in INTERNAL_API_KEY SESSION_STATE_SECRET; do
  if [ -z "$(printenv "$name")" ]; then
    echo "[web] $name is empty, refusing to start" >&2
    exit 1
  fi
done

build_assets=/app/apps/web/dist/client/_astro
kept_assets=/var/lib/web-assets/_astro
if [ -w "$kept_assets" ]; then
  (cd "$build_assets" && find . -type f) | while read -r file; do
    if [ -e "$kept_assets/$file" ]; then
      touch "$kept_assets/$file"
    else
      mkdir -p "$kept_assets/${file%/*}" && cp "$build_assets/$file" "$kept_assets/$file"
    fi
  done
  find "$kept_assets" -type f -mtime +7 -delete
else
  echo "[web] $kept_assets is not writable, so pages from earlier builds will miss their scripts" >&2
fi

HOST="${HOST:-127.0.0.1}" PORT="${SSR_PORT:-4321}" bun /app/apps/web/dist/server/entry.mjs &
ssr=$!
until nc -z 127.0.0.1 "${SSR_PORT:-4321}"; do
  if ! kill -0 "$ssr" 2>/dev/null; then
    echo "[web] the Astro server exited before it listened" >&2
    exit 1
  fi
  sleep 0.1
done
nginx -g 'daemon off;' &
ngx=$!

term() {
  kill -QUIT "$ngx" 2>/dev/null
  wait "$ngx" 2>/dev/null
  kill -TERM "$ssr" 2>/dev/null
  wait "$ssr" 2>/dev/null
  exit 0
}
trap term TERM INT

while kill -0 "$ssr" 2>/dev/null && kill -0 "$ngx" 2>/dev/null; do
  sleep 5
done

kill "$ssr" "$ngx" 2>/dev/null
exit 1
