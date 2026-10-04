# Topology

A single-page, diagram-first map of the whole system: what talks to what, in code and in
production. Each section links to the doc that carries the actual depth, this page's only job is
the big picture.

> Same rule as `docs/deployment/README.md`: this file stays secret-free. No credentials, tokens, real IPs, or
> Cloudflare account/tunnel IDs.

## Traffic into the VPS

```mermaid
flowchart LR
    browser["Browser"] -->|"qafiyah.com / www<br/>api.qafiyah.com<br/>ssh.qafiyah.com"| cf["Cloudflare edge<br/>(TLS, cache, rate limit)"]
    cf --> tunnel["Cloudflare Tunnel<br/>(cloudflared, egress-only)"]
    tunnel -->|"127.0.0.1:80"| edge["edge-gateway<br/>ModSecurity + OWASP CRS"]
    tunnel -->|"127.0.0.1:22"| sshd["sshd"]
    edge -->|"Host: qafiyah.com / api.qafiyah.com"| web["web<br/>nginx + Astro SSR"]
    web -->|"Host: api.qafiyah.com<br/>+ /v1 (SSR fetch)"| api["api<br/>axum"]
```

Nothing is reachable inbound, SSH included, everything arrives over the tunnel and every listener
binds to loopback. Full mechanics, the zero-downtime rolling deploy, and the security posture:
`docs/deployment/architecture.md`.

## Traffic that bypasses the VPS entirely

```mermaid
flowchart LR
    browser["Browser"] -->|"cdn.qafiyah.com"| r2["R2 bucket<br/>qafiyah-assets"]
    browser -->|"ix.qafiyah.com"| ixproxy["PostHog managed proxy<br/>(Cloudflare, no repo code)"]
    browser -->|"t.qafiyah.com"| worker["apps/telemetry-proxy<br/>Cloudflare Worker"]
    ixproxy --> posthog[("PostHog")]
    worker --> sentry[("Sentry")]
```

Three Cloudflare-only paths, none of them touch the WAF or the docker compose stack:

- `cdn.qafiyah.com` serves poet avatar images straight from R2 (`poets/<slug>/avatar.webp`). See
  `data/avatars/README.md`.
- `ix.qafiyah.com` is PostHog's own managed reverse proxy, provisioned on the Cloudflare side,
  there's no code for it in this repo.
- `t.qafiyah.com` is `apps/telemetry-proxy`, a first-party Worker that forwards browser error and session telemetry to Sentry. Details: `apps/telemetry-proxy/AGENTS.md`.

## Inside the VPS

```mermaid
flowchart LR
    edge["edge-gateway"] -->|"edge net"| web["web"]
    web -->|"backend net"| api["api"]
    api -->|"default net"| db[("Postgres")]
    api -->|"default net"| es[("Elasticsearch")]
    indexer["search-indexer<br/>(one-shot init job)"] -->|"default net"| db
    indexer -->|"default net"| es
```

Six containers, one `docker compose` stack, prod and dev coexist on the same VPS under separate
project namespaces. They are segmented into three networks so each hop trusts only its immediate
upstream: `edge` carries `edge-gateway` and `web` only, `backend` carries `web` and `api` only, and
`default` carries `api`, `db`, `es`, and the indexer. Ports, healthchecks, and per-service details:
`docs/deployment/services.md` and `docs/deployment/architecture.md`.

## Data topology

Three independent stores, different roles:

- **Postgres**: source of truth for poems, poets, and every taxonomy table.
- **Elasticsearch**: derived search index, fully rebuilt from Postgres by `search-indexer`.
  Nothing backs it up directly, `bun run reindex`/`reindex:prod` regenerates it from Postgres on
  demand.
- **R2**: object storage for poet avatar images, independent of Postgres (referenced by
  `poet.has_avatar`, not stored in it).

`data/` mirrors the two stores that can't be trivially regenerated:

- `data/db/`: versioned, encrypted Postgres dump snapshots. Automated (`bun run db:*`,
  self-seeding). See `data/db/README.md` and `data/db/MAINTAINERS_GUIDE.md`.
- `data/avatars/`: versioned, encrypted avatar-zip snapshots, same directory-naming and
  encryption pattern as `data/db/`, but manual today, no dedicated scripts yet. See
  `data/avatars/README.md`.

## Codebase topology

```mermaid
flowchart LR
    subgraph apps
        api["apps/api (Rust)"]
        web["apps/web (Astro + React)"]
        indexer["apps/search-indexer (Rust)"]
        edge["apps/edge-gateway (nginx config)"]
        telemetry["apps/telemetry-proxy (TS)"]
        inspector["apps/inspector (TS, dev-only)"]
    end
    tsconfig["packages/tsconfig"]
    es_crate["crates/elasticsearch"]
    corpus_crate["crates/corpus"]
    config["config.ts (root)"]

    web --> tsconfig
    telemetry --> tsconfig
    inspector --> tsconfig
    web --> config
    inspector --> config
    api --> es_crate
    indexer --> es_crate
    api --> corpus_crate
    indexer --> corpus_crate
```

Turborepo orchestrates the TypeScript workspace (`apps/*` + `packages/*`); a separate Cargo
workspace covers the Rust side (`apps/api`, `apps/search-indexer`, `crates/elasticsearch`, `crates/corpus`).
`apps/edge-gateway` is config only (no build step, an nginx image plus a template override).
Conventions: `docs/code-conventions.md`, `docs/typescript-conventions.md`,
`docs/rust-conventions.md`. Each component's `AGENTS.md` is listed in `README.md` ("Documentation map").

## CI/CD topology

- **Every push and PR to `main`** (`.github/workflows/ci.yml`): `bun run ci --no-docker` (static checks, types and repo checks, TypeScript and Rust tests, clippy, and the contract snapshots) always runs. Each Docker phase (`--phase db`, `origin`, and `stack`: the database-backed tests, the dev smoke with its Schemathesis run of every documented API example, and the stack smoke) runs as its own job only when the change touches something that phase uses. A `changes` job matches the changed files against per-phase skip lists with `dorny/paths-filter`: a push that touches only docs, agent guides, repo templates, lint config only the gate reads, or encrypted files (secrets, production dumps, avatars) runs the gate alone, and a web-only change skips `db`. A path on no skip list runs every phase, and a manual run (`workflow_dispatch`) runs them all. Every job checks out sparsely, leaving out the encrypted production dumps and avatars (about 2.2 GB) and keeping the committed 100-poem sample the Docker phases run on, so they need no secrets. `.github/workflows/gitleaks.yml` scans every push and PR for committed secrets. `.github/workflows/labeler.yml` adds component and topic labels to every PR from the files it changes, by the rules in `.github/labeler.yml` (`actions/labeler` on `pull_request_target`, which reads the changed-file list and the rules through the API and never checks out the PR's code).
- **Docker images are built on demand** (`.github/workflows/images.yml`, `workflow_dispatch`): one matrix job per service, build-only, nothing is pushed to a registry. Locally, `bun run build:images` builds the same images on their own; `bun run ci` builds them as part of the stack smoke.
- **Merging to `main` does not deploy.** Production deploy is a separate, manual, ordered step (`bun run deploy`, runbook in `.claude/skills/deploy/SKILL.md`) that refuses a commit whose CI run on `main` did not pass: it SSHes to the VPS, rebuilds images from source there, and rolls `api`/`web` with zero downtime. See `docs/deployment/README.md`.

## External services

| Service    | Role                                                                                 | Reached via                              |
| ---------- | ------------------------------------------------------------------------------------ | ---------------------------------------- |
| Cloudflare | DNS, TLS, Tunnel (ingress), R2 (object storage), Workers, cache and rate limit rules | all subdomains, `cdn.`, `t.`             |
| Sentry     | Error/session tracking for api + web                                                 | `t.qafiyah.com` (`apps/telemetry-proxy`) |
| PostHog    | Product analytics                                                                    | `ix.qafiyah.com` (Cloudflare-managed)    |
| GitHub     | Source hosting, Actions CI, secret scanning                                          | `.github/workflows/`                     |

## See also

- `README.md`: component overview and quick start
- `AGENTS.md`: repo layout and conventions index
- `docs/deployment/README.md`: deploy/ops entry point (splits into the `docs/deployment/*` files linked above)
- `data/README.md`: the two `data/` subsystems
