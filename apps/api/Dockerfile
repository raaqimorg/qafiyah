# syntax=docker/dockerfile:1

FROM rust:1.99-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY apps/api apps/api
COPY apps/search-indexer/Cargo.toml apps/search-indexer/Cargo.toml
RUN mkdir -p apps/search-indexer/src && echo 'fn main() {}' > apps/search-indexer/src/main.rs
COPY crates crates
COPY well-known well-known
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target,sharing=locked,target=/build/target \
    cargo build --release --locked --package qafiyah-api \
    && cp target/release/qafiyah-api /qafiyah-api

FROM alpine:3.21 AS runtime
ENV PORT=8787
RUN apk add --no-cache tini wget && adduser -D -u 1000 api
COPY --from=build /qafiyah-api /usr/local/bin/qafiyah-api
EXPOSE 8787
USER api
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD ["/bin/sh", "-c", "wget -q -O /dev/null http://127.0.0.1:8787/healthz || exit 1"]
ENTRYPOINT ["/sbin/tini", "--"]
CMD ["/usr/local/bin/qafiyah-api"]
