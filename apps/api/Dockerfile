# syntax=docker/dockerfile:1

FROM rust:1.99-alpine AS chef
RUN apk add --no-cache musl-dev && cargo install cargo-chef --locked --version 0.1.78
WORKDIR /build

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY apps/api apps/api
COPY apps/mcp apps/mcp
COPY apps/search-indexer apps/search-indexer
COPY crates crates
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --locked --package qafiyah-api --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY apps/api apps/api
COPY crates crates
COPY well-known well-known
RUN cargo build --release --locked --package qafiyah-api \
    && cp target/release/qafiyah-api /qafiyah-api

FROM alpine:3.24 AS runtime
ENV PORT=8787
RUN apk add --no-cache tini wget && adduser -D -u 1000 api
COPY --from=build /qafiyah-api /usr/local/bin/qafiyah-api
EXPOSE 8787
USER api
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD ["/bin/sh", "-c", "wget -q -O /dev/null http://127.0.0.1:8787/healthz || exit 1"]
ENTRYPOINT ["/sbin/tini", "--"]
CMD ["/usr/local/bin/qafiyah-api"]
