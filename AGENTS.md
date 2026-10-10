# Qafiyah Agent Guidelines

This monorepo holds qafiyah.com, an Arabic poetry catalog. It has a Rust/axum API over Postgres and Elasticsearch, and an Astro/React web frontend. Turborepo builds the TypeScript and Rust workspaces.

## Layout

- `apps/`: the deployable services. Each one has its own `AGENTS.md`:
  - `api` (Rust)
  - `web` (Astro/React)
  - `search-indexer` (Rust)
  - `edge-gateway` (nginx configuration only)
  - `observability` (Prometheus, Loki, Alloy, and Grafana configuration only)
  - `inspector` (TypeScript, for development only)
- `crates/elasticsearch/`: the Elasticsearch schema and client that `api` and `search-indexer` share. See its `AGENTS.md`.
- `crates/corpus/`: the corpus database's Diesel schema that `api` and `search-indexer` share. See its `AGENTS.md`.
- `packages/tsconfig/`: the TypeScript configuration bases (`base`, `astro`, `bun`).
- `config.ts` (root): constants that every TypeScript app and script shares. Each `tsconfig.json` imports it as `@qafiyah/config` through `paths`. `scripts/check/constants.ts` keeps the Rust side in sync.
- `scripts/`: repo tooling and CI checks, with one `bun run` name for each entry point. See `scripts/AGENTS.md`.
- `docs/`: every document that is not a component guide:
  - `development.md`, `topology.md`, `identity.md`, `domain.md`, `search.md`, and `exceptions.md`
  - the conventions files
  - `architecture/`: the C4 model `workspace.dsl` and the diagrams that `bun run docs:diagrams` renders from it
  - `deployment/`: start at `docs/deployment/README.md`
  - `changelog/`: one file for each released version (`v1.md`, `v2.md`, and so on)
- `data/`: versioned, encrypted snapshots (database dumps and avatar images). See `data/README.md`.
- `well-known/`: templates for `robots.txt`, `llms.txt`, and `security.txt`. The API embeds `llms.api.md`, `robots.api.txt`, and `security.txt` (`apps/api/src/routes/site.rs`). `bun run well-known:generate` renders `llms.web.md`, `robots.web.txt`, and `security.txt` into `apps/web/src/lib/generated/well-known/`.
- `secrets/`: SOPS-encrypted environment files. See `docs/deployment/secrets.md`.

## Style

- Code has no comments, except the two single-line kinds in `docs/code-conventions.md` ("Comments").
- Write docs, `AGENTS.md` files, issues, pull requests, and commit messages by the rules in `docs/code-conventions.md` ("Writing"). In short: use short sentences, commands for instructions, the active voice, and one word for one meaning.
- Do not use em-dashes anywhere: not in code, docs, commit messages, pull request descriptions, or chat responses. Use a period, a comma, or parentheses.

## Keep it boring

- This is a standard data app: it reads from a database and presents the data. Code must look like what an experienced developer expects in any similar app.
- Treat anything unusual as a warning sign. This includes custom solutions to solved problems, needless abstraction, tight coupling, clever tricks, and non-standard patterns. It also includes homemade versions of what a common library or framework provides, and UI that behaves differently from what users expect.
- If a request would add unusual code, do not build it yet. Say plainly that it is unusual and why, and show how apps like ours normally do it. Then wait for the user's decision.
- Read `docs/exceptions.md` before you build. Anything that it does not list must be standard. Point out any unlisted unusual code that you find. When the user approves an unusual approach, add its entry to `docs/exceptions.md` in the same change.

## References

- Topology: `docs/topology.md` is a diagram-first map of the whole system, both code and production.
- Domain: `docs/domain.md` says what a poem, poet, meter, rhyme, era, theme, and collection mean.
- Search: `docs/search.md` covers Arabic text handling, relevance tiers, and snippet selection.
- TypeScript: read `docs/typescript-conventions.md` before you write or change TypeScript code.
- Rust: read `docs/rust-conventions.md` before you write or change Rust code.
- Code conventions (architecture, naming, errors, comments, writing): read `docs/code-conventions.md` before you structure or review code, and before you write docs.
- Testing: read `docs/testing.md` before you write or change tests.
- Pull requests and releases: read `docs/pull-requests.md` before you file an issue, open a pull request, or finish a feature.
- Deployment: read `docs/deployment/` before you change CI/CD, infrastructure, or environment configuration.

## Agent workflow

- Hard rule: never start subagents, workflows, or any other multi-agent tooling unless the user asks for it in that request. Do the work yourself in the current session.
- Work goes on the open version branch (`v2`, `v3`, and so on). Branch off it and open pull requests into it, never into `main`. `main` changes only when a version is released, or for an urgent production fix that the user asks to ship on its own. See `docs/pull-requests.md` ("Versions and releases").
- Hard rule: never deploy unless the user asks for that deploy. This covers production deploys, `reindex:prod`, reseeds, and anything else that changes a live environment. Approval for one deploy does not carry over to the next.
- To carry out a written implementation plan, prefer `superpowers:executing-plans`. It works inline, in batches with checkpoints, in the current session. Use `superpowers:subagent-driven-development` (a new subagent for each task) only if the user asks for it.
- When the user asks for a review, use the `reviewing` skill (`.claude/skills/reviewing/SKILL.md`). It is one light sanity pass that you do yourself in the current session. Never use a subagent, a workflow, `/code-review`, or `superpowers:requesting-code-review` for it, unless the user names them.
- When a local setup step fails because the repo assumes a tool, a step, or a setting that is missing, report it:
  1. Finish the workaround, so the user can keep working.
  2. Search the open issues (`gh issue list --search`). If one already covers it, add a comment there instead.
  3. Ask the user, then open an issue with the bug template, with "Local development setup" as the area.
  4. Give the command, the full error, the OS and tool versions, and the workaround that worked. Leave out secrets and personal data.
