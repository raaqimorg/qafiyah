# Qafiyah Agent Guidelines

Monorepo for qafiyah.com, an Arabic poetry catalog: a Rust/axum API over Postgres + Elasticsearch, and an Astro/React web frontend, built with Turborepo across TypeScript and Rust workspaces.

## Layout

- `apps/`: deployable services, each with its own `AGENTS.md`: `api` (Rust), `web` (Astro/React), `search-indexer` (Rust), `edge-gateway` (nginx config only), `observability` (Prometheus and Grafana config only), `telemetry-proxy` (Cloudflare Worker), `inspector` (TypeScript, dev-only).
- `crates/elasticsearch/`: the Elasticsearch schema and client shared by `api` and `search-indexer` (see its `AGENTS.md`).
- `crates/corpus/`: the corpus database's Diesel schema shared by `api` and `search-indexer` (see its `AGENTS.md`).
- `packages/tsconfig/`: the TypeScript config bases (`base`, `astro`, `bun`).
- `config.ts` (root): constants shared by every TypeScript app and script, imported as `@qafiyah/config` through each `tsconfig.json`'s `paths`. `scripts/check/constants.ts` keeps the Rust side in sync.
- `scripts/`: repo tooling and CI checks, one `bun run` name per entry point (see `scripts/AGENTS.md`).
- `docs/`: everything that is not a component guide: `development.md`, `topology.md`, `identity.md`, the conventions files, `domain.md`, `search.md`, `exceptions.md`, and `deployment/` (entry point `docs/deployment/README.md`).
- `data/`: versioned encrypted snapshots (DB dumps, avatar images), see `data/README.md`.
- `well-known/`: templates for `robots.txt`, `llms.txt`, and `security.txt`. `llms.api.md`, `robots.api.txt`, and `security.txt` are embedded by the API (`apps/api/src/routes/site.rs`); `llms.web.md`, `robots.web.txt`, and `security.txt` are rendered into `apps/web/src/lib/generated/well-known/` by `bun run well-known:generate`.
- `secrets/`: SOPS-encrypted env files, see `docs/deployment/secrets.md`.

## Style

- No code comments beyond the two single-line exceptions in `docs/code-conventions.md` ("Comments"). No em-dashes anywhere: code, docs, commit messages, PR descriptions, or chat responses. Use a period, comma, or parentheses instead.

## Keep it boring

- This is a standard data app: read from a database, present the data. Code should look like what an experienced developer expects in any similar app. Treat anything unusual as a warning sign: custom solutions to solved problems, needless abstraction, tight coupling, clever tricks, non-standard patterns, homemade versions of what a common library or framework provides, or UI that behaves differently from what users expect.
- If a request would introduce unusual code, do not build it yet. Say plainly that it is unusual and why, show how apps like ours normally do it, and wait for the user's decision.
- Check `docs/exceptions.md` before building. Anything not listed there should be standard; point out unlisted unusual code you find. When an unusual approach is approved, add its entry there in the same change.

## References

- Topology: see `docs/topology.md` for a diagram-first map of the whole system, code and production.
- Domain: see `docs/domain.md` for what a poem, poet, meter, rhyme, era, theme, and collection mean.
- Search: see `docs/search.md` for Arabic text handling, relevance tiers, and snippet selection.
- TypeScript: see `docs/typescript-conventions.md`, read before writing or changing TypeScript code.
- Rust: see `docs/rust-conventions.md`, read before writing or changing Rust code.
- Code conventions (architecture, naming, errors, style): see `docs/code-conventions.md`, read before structuring or reviewing any code.
- Testing: see `docs/testing.md`, read before writing or changing tests.
- Pull requests: see `docs/pull-requests.md`, read before filing an issue, opening a PR, or finishing a feature.
- Deployment: when changing CI/CD, infrastructure, or environment config, read `docs/deployment/`.

## Agent workflow

- Hard rule: never spawn subagents, workflows, or any other multi-agent tooling unless the user explicitly asks for it in that request. Do the work yourself in the current session.
- Hard rule: never deploy unless the user explicitly asks for that deploy. This covers production deploys, `reindex:prod`, reseeds, and anything else that changes a live environment. Approval for one deploy does not carry over to the next.
- When executing a written implementation plan, prefer `superpowers:executing-plans` (inline, batch execution with checkpoints in the current session) over `superpowers:subagent-driven-development` (fresh subagent per task), unless the user asks for the latter.
- When the user asks for a review, use the `reviewing` skill (`.claude/skills/reviewing/SKILL.md`): one light sanity pass done yourself in the current session, never a subagent, workflow, `/code-review`, or `superpowers:requesting-code-review` unless the user names them.
