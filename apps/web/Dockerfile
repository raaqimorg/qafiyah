# syntax=docker/dockerfile:1

FROM oven/bun:1.4.2-alpine AS base
WORKDIR /app
ENV NODE_ENV=production

FROM base AS deps
COPY package.json bun.lock ./
COPY apps/web/package.json apps/web/package.json
COPY packages/tsconfig/package.json packages/tsconfig/package.json
COPY apps/telemetry-proxy/package.json apps/telemetry-proxy/package.json
COPY apps/inspector/package.json apps/inspector/package.json
RUN bun install --frozen-lockfile

FROM deps AS build
COPY . .
ARG SENTRY_RELEASE=""
RUN --mount=type=secret,id=sentry_auth_token \
    SENTRY_AUTH_TOKEN="$(cat /run/secrets/sentry_auth_token 2>/dev/null || true)"; \
    if [ -n "$SENTRY_RELEASE" ] && [ -z "$SENTRY_AUTH_TOKEN" ]; then \
      echo "ERROR: SENTRY_RELEASE=$SENTRY_RELEASE is set but the sentry_auth_token build secret is missing; refusing to ship a release without source maps." >&2; \
      exit 1; \
    fi; \
    if [ -n "$SENTRY_RELEASE" ]; then \
      export SENTRY_AUTH_TOKEN SENTRY_RELEASE PUBLIC_SENTRY_RELEASE="$SENTRY_RELEASE"; \
    fi; \
    bun run --filter @qafiyah/web build

FROM base AS prod-deps
COPY package.json bun.lock ./
COPY apps/web/package.json apps/web/package.json
COPY packages/tsconfig/package.json packages/tsconfig/package.json
COPY apps/telemetry-proxy/package.json apps/telemetry-proxy/package.json
COPY apps/inspector/package.json apps/inspector/package.json
RUN bun install --frozen-lockfile --production --ignore-scripts --linker hoisted --filter @qafiyah/web
COPY packages ./packages

FROM oven/bun:1.4.2-alpine AS serve
RUN apk add --no-cache nginx tini && apk upgrade --no-cache libssl3 libcrypto3
WORKDIR /app
COPY --from=build     /app/apps/web/dist  ./apps/web/dist
COPY --from=prod-deps /app/node_modules   ./node_modules
COPY --from=prod-deps /app/packages       ./packages
COPY apps/web/nginx.conf /etc/nginx/nginx.conf
COPY apps/web/nginx-security-headers.conf /etc/nginx/nginx-security-headers.conf
COPY apps/web/nginx-csp.conf /etc/nginx/nginx-csp.conf
COPY apps/web/nginx-csp-api.conf /etc/nginx/nginx-csp-api.conf
COPY apps/web/nginx-bad-bots.conf /etc/nginx/nginx-bad-bots.conf
COPY apps/web/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh \
    && mkdir -p /var/cache/nginx/astro \
    && chown -R nginx:nginx /var/cache/nginx
ENV NODE_ENV=production HOST=127.0.0.1 SSR_PORT=4321
EXPOSE 8080
USER nginx
HEALTHCHECK --interval=30s --timeout=3s --start-period=15s --retries=3 \
  CMD ["/bin/sh", "-c", "wget -q -O /dev/null http://127.0.0.1:8080/healthz || exit 1"]
ENTRYPOINT ["/sbin/tini", "--", "/docker-entrypoint.sh"]
