# Development

Local development workflow for this monorepo. For architecture and per-app internals, see
`README.md` and each app's `AGENTS.md`. For production, see `docs/deployment/README.md`.

## Prerequisites

- [Bun](https://bun.sh) ≥ 1
- A Docker Engine (Postgres, Elasticsearch, and the app containers run through Compose). On a
  Mac, use [OrbStack](https://orbstack.dev) rather than Docker Desktop: `scripts/dev/run.ts`
  auto-detects it (`docker info` reporting `OrbStack`) and, when present, talks to Elasticsearch
  over OrbStack's `*.orb.local` container DNS instead of a mapped `localhost` port, so `dev`
  doesn't need Elasticsearch's host port at all in that case. Docker Desktop still works, just
  without that shortcut.
- Rust (`rust-toolchain.toml` pins the version; `bun run check:rust-toolchain` verifies it matches)
- [ShellCheck](https://www.shellcheck.net), [actionlint](https://github.com/rhysd/actionlint), and [hadolint](https://github.com/hadolint/hadolint) for the static phase of the gate (`brew install shellcheck actionlint hadolint`)
- Optional: the [GitHub CLI](https://cli.github.com) (`gh`, then `gh auth login`). Recommended when working with an AI agent, so it can read and file issues and open pull requests itself (`.github/CONTRIBUTING.md`, "Working with an AI agent").

## Getting started

```bash
bun install
bun run dev
```

`bun run dev` (`scripts/dev/run.ts`) does the following, in order:

1. Checks Docker is running.
2. Resolves and decrypts a database dump (`scripts/db/resolve-dump.sh`); falls back to the
   bundled `data/db/0000_default/` sample (100 poems) if no passphrase is set. See
   `data/db/README.md`.
3. Brings up Postgres + Elasticsearch via Docker Compose and restores the dump on a fresh volume.
4. Runs the search indexer once to populate Elasticsearch.
5. Creates `apps/web/.env` (just `PUBLIC_API_URL`, pointing at the local API) if it doesn't exist yet.
6. Builds and starts the API (`cargo build -p qafiyah-api`), then the web app, each prefixed and
   color-coded in the combined log output. Pass `--inspector` to also start the inspector after
   the web app reports ready.

Default URLs:

| Service       | URL                   |
| ------------- | --------------------- |
| Web           | http://localhost:4321 |
| API           | http://localhost:8787 |
| Inspector     | http://localhost:4322 |
| Postgres      | localhost:5434        |
| Elasticsearch | localhost:9201        |

Ports come from `config.ts` (`DEV_WEB_PORT`, `DEV_API_PORT`, `DEV_INSPECTOR_PORT`,
`DEV_POSTGRES_PORT`, `DEV_ES_PORT`, `DEV_EDGE_PORT`) and can be overridden via the matching env
vars (`PORT`, `WEB_PORT`, `INSPECTOR_PORT`, `DEV_POSTGRES_PORT`, ...).

`Ctrl-C` stops all running processes (SIGTERM, then SIGKILL after a 5s grace period).

## Everyday commands

```bash
bun run dev:preflight   # check dev ports are free before starting (runs automatically in dev)
bun run db:up           # start just Postgres + Elasticsearch (docker compose up -d --wait)
bun run db:reset        # wipe the local Postgres volume, restore the dump fresh
bun run down            # stop the Docker Compose stack
bun run clean           # kill stray astro/qafiyah-api processes from a previous run
bun run reindex         # rebuild the indexer image, then force-rebuild the Elasticsearch indices from Postgres
```

The sample dataset needs no credentials: `scripts/dev/compose.sh` and `scripts/dev/run.ts` default every dev database and Elasticsearch password, so a fresh clone has no `.env` at all. Two things change that:

- **A real dump.** Email dumps@qafiyah.com for a passphrase (`data/db/README.md`), then put `DUMP_KEY__<dump-dir>=<passphrase>` in a root `.env` (gitignored). `bun run db:reset` restores that dump instead of the sample. `bun run dump:key:check` verifies a passphrase without restoring.
- **The maintainers' age key.** With it, `.env` is generated from `secrets/dev.enc.env` by `bun run secrets:pull` and dump passphrases are stored with `bun run dump:key:set`; never edit `.env` by hand on such a machine. See `docs/deployment/secrets.md`. Without the key, none of the `secrets:*` commands apply and `.env` is yours to write.

## Quality gates

```bash
bun run lint              # oxlint, type-aware, --fix
bun run lint:check        # oxlint, type-aware, read-only, warnings fail (what the gate runs)
bun run format            # oxfmt, then prettier on .astro files, writes
bun run format:check      # the same two, read-only (what the gate runs)
bun run check:shell       # ShellCheck on every tracked shell script, warnings fail
bun run check:workflows   # actionlint on .github/workflows
bun run check:dockerfiles # hadolint on every Dockerfile, warnings fail
bun run check:sql         # PostgreSQL 18 parser on every .sql file, squawk on migrations
bun run types             # turbo run types (TypeScript, per app/package)
bun run test              # turbo run test (TypeScript, per app/package)
bun run rust:fmt          # cargo fmt --check
bun run rust:lint         # cargo clippy, workspace, -D warnings; cargo's own warnings fail too
bun run rust:test         # cargo test, workspace
bun run rust:test:db      # database-backed API tests against the dev stack (needs Docker)
bun run smoke:dev         # black-box HTTP probes against a locally-managed dev server
bun run ci                # the full gate (GitHub Actions runs it scoped to the change); --no-docker skips db and smoke, --docker-only runs just those
```

Deliberate, non-obvious behavior of the static checks is in the Static checks section of `docs/exceptions.md`.

The smoke runs leave your dev environment as they found it. `smoke:dev` starts its own `bun run dev`
and stops it afterwards, unless the dev web and API already answer, in which case it reuses them and
leaves them running. The `stack` phase of `bun run ci` (`smoke:stack`) runs the production-mode
stack in the same Compose project and removes it at the end, then starts again whichever of the dev
`db` and `elasticsearch` containers were running before it. When a check fails or you press Ctrl-C,
`bun run ci` stops only the tasks it started (each runs in its own process group), so a running
`bun run dev`, here or in another worktree, keeps running.

`rust:test:db` brings up the dev Postgres and Elasticsearch, runs the one-shot indexer, and runs
`apps/api/tests/db.rs` with the same connection strings `bun run dev` uses. Without the
`QAFIYAH_TEST_*` variables that target skips itself, which is why plain `cargo test` stays pure.

Run `bun run ci` before opening a PR; it's the full gate. GitHub Actions runs the same gate, skipping a Docker
phase when the change touches nothing it uses (`docs/topology.md`, "CI/CD topology"). `bun run ci --no-docker`
skips the Docker-dependent steps (the database-backed tests and both smokes) if Docker isn't
available, and `bun run ci --docker-only` runs only those. The stack smoke builds the three images
through `compose up --build`, so a full run still proves every image builds.

## Working in a git worktree

If you're working out of a git worktree (not the primary checkout, e.g. under `.worktrees/`) and
want it fully isolated from a dev stack already running in the primary checkout, **pass
`--worktree` to `dev`, `db:reset`, and `reindex`**:

```bash
bun run dev --worktree
./scripts/dev/db-reset.sh --worktree
./scripts/es/reindex.sh --worktree
bun run smoke:dev --worktree     # against a --worktree dev server
```

Without `--worktree`, a worktree checkout shares the same Docker Compose project name and ports
as the primary checkout, so running `dev` in both at once collides. With `--worktree`, the
Compose project name and every port (Postgres, Elasticsearch, edge gateway, API, web, inspector)
get a per-worktree offset derived from the worktree's directory name, so multiple worktrees can
run their own dev stacks side by side without clashing.

Do not assume `--worktree` is implied by being in a worktree, it is not: `scripts/dev/worktree.ts`
only _detects_ whether you're in a worktree; nothing isolates ports or the Compose project unless
you pass the flag explicitly. Passing `--worktree` from the primary checkout is a hard error
("nothing to isolate against").

A worktree's root `.env` is copied from the primary checkout's `.env` automatically on first
`bun run dev --worktree` if it doesn't already have one (`apps/web/.env` is generated with the
worktree's own API port, handled automatically).

## Committing

Commit messages follow `docs/pull-requests.md` (one subject line, nothing else). The pre-commit hook (`.husky/pre-commit`) runs lint-staged on the staged files only (oxlint, oxfmt, prettier for `.astro`, rustfmt, and ShellCheck, per the `lint-staged` block in `package.json`), in about a second. The pre-push hook (`.husky/pre-push`) runs `bun run ci --no-docker`, about 30 seconds. GitHub Actions runs the gate on every push and PR to `main`, and each Docker phase when the change touches what it uses; `bun run deploy` refuses a commit whose CI run did not pass (a skipped phase does not count as a failure). `HUSKY=0 git commit ...` or `HUSKY=0 git push ...` skips the hook when you have already run the gate.

Both hooks first run `scripts/check/commit-identity.sh`, an optional guard against committing with the wrong email. It does nothing until you opt in, per clone, with `git config --local qafiyah.allowedEmail "$(git config --local user.email)"`; the address stays in `.git/config`, which is never committed. From then on a commit is refused unless its author and committer use that address, and a push is refused if any commit that isn't already on a remote has an author, committer, or `*-by:` trailer (such as `Co-authored-by:`) with another one (commits already on a remote, such as main's GitHub-made squash merges after you merge main into a branch, are public already and not checked). It reads the key with `git config --local`, so `git -c` overrides cannot satisfy it, but `--no-verify` or `HUSKY=0` skip it like any hook; GitHub's "Block command line pushes that expose my email" setting covers the addresses on your account on the server side.

`AGENTS.md` is the per-directory guide; `CLAUDE.md` and `GEMINI.md` next to each one are committed symlinks to it, so every agent harness reads the same file. `bun run agents:link` recreates them after adding an `AGENTS.md`. On Windows, check out with `git config core.symlinks true` from a Developer Mode or admin shell, or the links appear as one-line text files.

## Troubleshooting

- **Port already in use**: `bun run clean` kills stray `astro`/`qafiyah-api` processes from a
  previous run, then retry.
- **Docker not running**: start Docker Desktop/OrbStack; `dev` and `ci` both check for this
  up front and fail fast with a clear message.
- **Postgres unreachable warning during preflight**: run `bun run db:up`.
