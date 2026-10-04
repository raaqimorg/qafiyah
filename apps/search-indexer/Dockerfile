# syntax=docker/dockerfile:1

FROM rust:1.99-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY apps/search-indexer apps/search-indexer
COPY apps/api/Cargo.toml apps/api/Cargo.toml
RUN mkdir -p apps/api/src && echo 'fn main() {}' > apps/api/src/main.rs
COPY crates crates
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target,sharing=locked,target=/build/target \
    cargo build --release --locked --package qafiyah-search-indexer \
    && cp target/release/qafiyah-search-indexer /qafiyah-search-indexer

FROM alpine:3.21 AS runtime
RUN apk add --no-cache tini && adduser -D -u 1000 indexer
COPY --from=build /qafiyah-search-indexer /usr/local/bin/qafiyah-search-indexer
USER indexer
ENTRYPOINT ["/sbin/tini", "--"]
CMD ["/usr/local/bin/qafiyah-search-indexer"]
