# Edge Gateway Agent Guide

This directory is configuration only, with no code and no build. The production edge is the stock OWASP ModSecurity CRS nginx image, pinned by tag in `docker-compose.yml`. It inspects every request on `127.0.0.1:80`, and proxies it to the web container.

- How it is tuned, and why it still runs in `DetectionOnly`: `docs/deployment/services.md` ("Web Application Firewall").
- Where it is in the traffic path, and how the zero-downtime rollout depends on it: `docs/deployment/architecture.md`.

`proxy_backend.conf.template` is the one configuration file here. It is an exact copy of the image's `includes/proxy_backend.conf.template`, and `docker-compose.yml` mounts it over the original. It has three changes:

1. It proxies the backend through an nginx variable: `set $crs_backend ${BACKEND}; proxy_pass $crs_backend;`. So nginx uses the resolver that the image already defines (`127.0.0.11 valid=5s`), and looks up `web-edge:8080` again at request time. With the stock literal `proxy_pass`, nginx keeps the web container's IP from startup. Then it answers 502 after a zero-downtime web rollout gives the replacement a new IP.
2. `proxy_read_timeout` is 60s, not the stock 36000s. The app has no streaming, WebSocket, or SSE upstream to protect. A limit of 10 hours would keep an edge worker connection busy on any server-side render that hangs.
3. `proxy_next_upstream error timeout; proxy_next_upstream_tries 2;` tries a request again once, when the connection to the web container fails or times out. nginx never tries again a request that is not idempotent, such as a POST, after it was sent.

When you change the image tag in `docker-compose.yml`, copy the file again from the running container, and apply those three changes again:

```bash
docker exec qafiyah-edge-gateway cat /etc/nginx/templates/includes/proxy_backend.conf.template
```
