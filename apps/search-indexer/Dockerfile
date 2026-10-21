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
RUN cargo chef cook --release --locked --package qafiyah-search-indexer --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY apps/search-indexer apps/search-indexer
COPY crates crates
RUN cargo build --release --locked --package qafiyah-search-indexer \
    && cp target/release/qafiyah-search-indexer /qafiyah-search-indexer

FROM alpine:3.24 AS runtime
RUN apk add --no-cache tini && adduser -D -u 1000 indexer
COPY --from=build /qafiyah-search-indexer /usr/local/bin/qafiyah-search-indexer
USER indexer
ENTRYPOINT ["/sbin/tini", "--"]
CMD ["/usr/local/bin/qafiyah-search-indexer"]
