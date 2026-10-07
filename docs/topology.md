# Topology

This is a diagram-first map of the whole system. It shows who uses the system, what runs where, and what talks to what. This page gives the big picture. Each section links to the doc that has the details.

The diagrams follow the [C4 model](https://c4model.com). They all come from one model, `docs/architecture/workspace.dsl`. `bun run docs:diagrams` renders each view of the model to an SVG in `docs/architecture/generated/structurizr/`. Change the model, never the SVGs. CI fails when the SVGs no longer match the model (the `diagrams` phase, in "CI/CD topology" below).

> The rule of `docs/deployment/README.md` applies here too: keep this file and the model free of secrets. Write no credentials, tokens, real IP addresses, or Cloudflare account or tunnel IDs.

## System context

![System context: readers, API developers, and the maintainer, and the external systems Qafiyah depends on](architecture/generated/structurizr/context.gen.svg)

Readers and API developers reach Qafiyah through Cloudflare. Two kinds of traffic go directly to the two parts of Qafiyah that Cloudflare hosts: poet avatars (`cdn.qafiyah.com`) and browser error reports (`t.qafiyah.com`). Page views go to PostHog through its managed proxy (`ix.qafiyah.com`). Developers sign in with Google or GitHub.

## Containers

![Containers: the edge gateway, website, API, databases, search index, search indexer, telemetry proxy, and avatar store](architecture/generated/structurizr/containers.gen.svg)

Everything except the avatar store and the telemetry proxy runs in one `docker compose` project on the VPS. The project has five networks:

- `edge`: the edge gateway and the website.
- `backend`: the website and the API.
- `default`: the API, Postgres, Elasticsearch, the search indexer, the monitor-role job, the three exporters, and Prometheus.
- `observability`: Prometheus, Grafana, Loki, and Alloy.
- `metrics`: the website, Prometheus, and the blackbox exporter. It carries the website's metrics push and its health probe.

The website shares no network with Postgres or Elasticsearch. The API trusts forwarded visitor addresses only from `backend`, and only the website and the API are on that network.

Three hosts never touch the VPS, the WAF, or the Compose stack:

- `cdn.qafiyah.com` serves poet avatar images directly from R2 (`poets/<slug>/avatar.webp`). See `data/avatars/README.md`.
- `ix.qafiyah.com` is PostHog's own managed reverse proxy, set up on the Cloudflare side. This repo has no code for it.
- `t.qafiyah.com` is `apps/telemetry-proxy`, a first-party Worker. It forwards browser error and session telemetry to Sentry. See `apps/telemetry-proxy/AGENTS.md`.

## Production deployment

![Production deployment: Cloudflare, then the tunnel into cloudflared on the VPS, then the Compose project](architecture/generated/structurizr/deployment.gen.svg)

Nothing is reachable from outside, not even SSH. `cloudflared` dials out to Cloudflare, every request arrives through the tunnel, and every listener binds to loopback.

The observability containers run in the same Compose project (next section). A dev copy of the stack can share the VPS, under its own project, `qafiyah-dev`, with its own containers, ports, and volumes. The mechanics, the zero-downtime rolling deploy, and the security posture are in `docs/deployment/architecture.md`.

## Observability

![The observability stack: Prometheus, Grafana, Loki, Alloy, the exporters, and what they watch](architecture/generated/structurizr/observability.gen.svg)

The observability stack is private. Grafana listens on loopback. To reach it, run `bun run observe`, which forwards a port over SSH through the tunnel.

- The dashboards, the retention, and what each exporter reads are in `apps/observability/AGENTS.md`.
- The ports, the healthchecks, and the details of each service are in `docs/deployment/services.md`.

## Flows

### A website search

![A website search, from the search island through Cloudflare, the edge gateway, the website, and the API to Elasticsearch](architecture/generated/structurizr/search.gen.svg)

Cloudflare never caches `/api/`. So the website's nginx is the one shared cache for searches.

The browser never holds an API key. The website's proxy forwards only the paths on its allowlist, and adds the internal key itself (`docs/exceptions.md`, "Two API clients, and no key ever reaches the browser"). `docs/search.md` explains how ranking works.

### A poet page and its filters

![A poet page rendered on the server, then a filter change refetched in place through the proxy](architecture/generated/structurizr/poet-filters.gen.svg)

The first render is on the server. After that, a change of meter, rhyme, or theme fetches the list again in place, through the same proxy. The proxy forwards `poems` and `poems/facets` only for a request that names exactly one poet (`docs/exceptions.md`, "The poet page's poem list updates in place").

### A forced reindex

![A forced reindex: the maintainer starts the indexer, which streams the corpus into a new index and swaps the alias](architecture/generated/structurizr/reindex.gen.svg)

The indexer builds a new versioned index. It moves the alias to the new index only when the index is complete. So searches keep using the old index until then. When and how to force a reindex is in `apps/search-indexer/AGENTS.md` and `docs/deployment/services.md`.

## Components

### API

![The API: HTTP layers, routes, request parsing, contract, domain, accounts, and the Postgres and Elasticsearch adapters](architecture/generated/structurizr/api-components.gen.svg)

This diagram comes from the "Shape" section of `apps/api/AGENTS.md`, which says what each part owns.

### Website

![The website: nginx, pages, the catalog client, the API proxy, identity, islands, and request metrics](architecture/generated/structurizr/web-components.gen.svg)

This diagram comes from the "Shape" section of `apps/web/AGENTS.md`.

## Data topology

There are three independent stores, with different roles:

- **Postgres** is the source of truth for poems, poets, and every taxonomy table.
- **Elasticsearch** is a derived search index. `search-indexer` rebuilds it fully from Postgres. Nothing backs it up directly. `bun run reindex` or `bun run reindex:prod` makes it again from Postgres when necessary.
- **R2** is object storage for poet avatar images. It is independent of Postgres: `poet.has_avatar` refers to an image, but Postgres does not store it.

`data/` keeps copies of the two stores that are hard to make again:

- `data/db/`: versioned, encrypted Postgres dump snapshots. Scripts handle them (`bun run db:reset`, `bun run db:reseed`, and the database seeds itself). See `data/db/README.md` and `data/db/MAINTAINERS_GUIDE.md`.
- `data/avatars/`: versioned, encrypted snapshots of the avatar zip. They use the same directory names and encryption as `data/db/`. But they are manual today, with no dedicated scripts. See `data/avatars/README.md`.

## Codebase topology

| Package                | Language                                    | Uses                                    |
| ---------------------- | ------------------------------------------- | --------------------------------------- |
| `apps/api`             | Rust                                        | `crates/elasticsearch`, `crates/corpus` |
| `apps/search-indexer`  | Rust                                        | `crates/elasticsearch`, `crates/corpus` |
| `apps/web`             | TypeScript (Astro + React)                  | `packages/tsconfig`, `config.ts`        |
| `apps/inspector`       | TypeScript, dev-only                        | `packages/tsconfig`, `config.ts`        |
| `apps/telemetry-proxy` | TypeScript (Cloudflare Worker)              | `packages/tsconfig`                     |
| `apps/edge-gateway`    | nginx config                                | nothing                                 |
| `apps/observability`   | Prometheus, Loki, Alloy, and Grafana config | nothing                                 |

- Turborepo runs the TypeScript workspace (`apps/*` and `packages/*`).
- A separate Cargo workspace covers the Rust side (`apps/api`, `apps/search-indexer`, `crates/elasticsearch`, `crates/corpus`).
- `apps/edge-gateway` is configuration only, with no build step: an nginx image and a template override.
- `apps/observability` is also configuration only: the Prometheus, Loki, Alloy, and Grafana images, with their configuration and dashboards.

The conventions are in `docs/code-conventions.md`, `docs/typescript-conventions.md`, and `docs/rust-conventions.md`. `README.md` ("Documentation map") lists the `AGENTS.md` of each component.

## CI/CD topology

**Every push and pull request to `main` or to a version branch (`v2`, `v3`, and so on)** runs `.github/workflows/ci.yml`:

- `bun run ci --no-docker` always runs. It covers the static checks, the types and repo checks, the TypeScript and Rust tests, clippy, and the contract snapshots.
- Each Docker phase runs as its own job, only when the change touches something that the phase uses:
  - `db`: the database-backed tests.
  - `origin`: the dev smoke, with its Schemathesis run of every documented API example.
  - `stack`: the stack smoke.
- A fourth phase, `diagrams` (`bun run docs:diagrams:check`), runs only when `docs/architecture/` or `scripts/docs/` changes. It renders the diagrams on this page again, and compares them with the committed SVGs.
- A `changes` job compares the changed files with a skip list for each phase, with `dorny/paths-filter`.
  - A push that touches only these files runs the gate alone: docs, agent guides, repo templates, lint configuration that only the gate reads, or encrypted files (secrets, production dumps, avatars).
  - A change to the website only skips `db`.
  - A path on no skip list runs every phase. A manual run (`workflow_dispatch`) runs them all.
- Every job does a sparse checkout. It leaves out the encrypted production dumps and avatars (about 3.7 GB in October 2026). It keeps the committed 100-poem sample that the Docker phases run on, so they need no secrets.

Two other workflows run on pull requests:

- `.github/workflows/gitleaks.yml` scans every push and pull request for committed secrets.
- `.github/workflows/labeler.yml` adds component and topic labels to every pull request, from the files that it changes, by the rules in `.github/labeler.yml`. It uses `actions/labeler` on `pull_request_target`. That reads the list of changed files and the rules through the API, and never checks out the code of the pull request.

**Docker images build on demand** (`.github/workflows/images.yml`, `workflow_dispatch`). There is one matrix job for each service. The jobs only build; nothing goes to a registry. Locally, `bun run build:images` builds the same images alone, and `bun run ci` builds them as part of the stack smoke.

**Work merges into the open version branch, and a release merges that branch into `main`** (`docs/pull-requests.md`, "Versions and releases"). **A merge into `main` does not deploy.** The production deploy is a separate, manual, ordered step: `bun run deploy`, with the runbook in `.claude/skills/deploy/SKILL.md`. It refuses a commit whose CI run on `main` did not pass. It connects to the VPS over SSH, builds the images from source there, and replaces `api` and `web` with zero downtime. See `docs/deployment/README.md`.

## External services

| Service    | Role                                                                                 | Reached via                                                 |
| ---------- | ------------------------------------------------------------------------------------ | ----------------------------------------------------------- |
| Cloudflare | DNS, TLS, Tunnel (ingress), R2 (object storage), Workers, cache and rate limit rules | all subdomains, `cdn.`, `t.`                                |
| Sentry     | Error and session tracking for the API and the website                               | directly from the servers; `t.qafiyah.com` from the browser |
| PostHog    | Product analytics                                                                    | `ix.qafiyah.com` (Cloudflare-managed)                       |
| GitHub     | Source hosting, Actions CI, secret scanning, developer sign-in                       | `.github/workflows/`; OAuth from the website                |
| Google     | Developer sign-in                                                                    | OAuth from the website                                      |

## See also

- `docs/architecture/workspace.dsl`: the C4 model that these diagrams come from.
- `README.md`: an overview of the components, and a quick start.
- `AGENTS.md`: the repo layout and an index of the conventions.
- `docs/deployment/README.md`: the entry point for deploys and operations. It splits into the `docs/deployment/*` files linked above.
- `data/README.md`: the two `data/` subsystems.
