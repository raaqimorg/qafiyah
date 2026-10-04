# Edge Gateway Agent Guide

Configuration only, no code and no build. The production edge is the stock OWASP ModSecurity CRS nginx image, pinned by tag in `docker-compose.yml`, inspecting every request on `127.0.0.1:80` and proxying to the web container. How it is tuned and why it still runs in `DetectionOnly`: `docs/deployment/services.md` ("Web Application Firewall"). Where it sits in the traffic path and how the zero-downtime rollout depends on it: `docs/deployment/architecture.md`.

`proxy_backend.conf.template` is the one configuration file here. It is a verbatim copy of the image's `includes/proxy_backend.conf.template`, mounted over the original by `docker-compose.yml`, with three changes: the backend is proxied through an nginx variable (`set $crs_backend ${BACKEND}; proxy_pass $crs_backend;`) so nginx honors the resolver the image already defines (`127.0.0.11 valid=5s`) and re-resolves `web-edge:8080` at request time, and `proxy_read_timeout` is 60s instead of the stock 36000s because the app has no streaming/WebSocket/SSE upstream to protect (a 10-hour ceiling would pin an edge worker connection on any wedged SSR render). The third, `proxy_next_upstream error timeout; proxy_next_upstream_tries 2;`, retries a request once when the connection to the web container fails or times out; nginx never retries a non-idempotent request such as a POST once it has been sent. With the stock literal `proxy_pass`, nginx pins the web container's IP at startup and 502s after a zero-downtime web roll hands the replacement a new IP.

When the image tag in `docker-compose.yml` is bumped, refresh the copy from the running container and re-apply those three changes:

```bash
docker exec qafiyah-edge-gateway cat /etc/nginx/templates/includes/proxy_backend.conf.template
```
