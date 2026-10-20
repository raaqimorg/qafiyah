# syntax=docker/dockerfile:1

FROM rust:1.99-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY apps/mcp apps/mcp
COPY apps/api/Cargo.toml apps/api/Cargo.toml
RUN mkdir -p apps/api/src && echo 'fn main() {}' > apps/api/src/main.rs
COPY apps/search-indexer/Cargo.toml apps/search-indexer/Cargo.toml
RUN mkdir -p apps/search-indexer/src && echo 'fn main() {}' > apps/search-indexer/src/main.rs
COPY crates crates
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target,sharing=locked,target=/build/target \
    cargo build --release --locked --package qafiyah-mcp \
    && cp target/release/qafiyah-mcp /qafiyah-mcp

FROM alpine:3.24 AS runtime
ENV MCP_PORT=8788
RUN apk add --no-cache tini wget && adduser -D -u 1000 mcp
COPY --from=build /qafiyah-mcp /usr/local/bin/qafiyah-mcp
EXPOSE 8788
USER mcp
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD ["/bin/sh", "-c", "wget -q -O /dev/null http://127.0.0.1:8788/healthz || exit 1"]
ENTRYPOINT ["/sbin/tini", "--"]
CMD ["/usr/local/bin/qafiyah-mcp"]
