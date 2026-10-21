# Development

This is the local development workflow for the monorepo. For the architecture and the internals of each app, see `README.md` and each app's `AGENTS.md`. For production, see `docs/deployment/README.md`.

## Prerequisites

Install Bun first. Then run `bun install` and `bun run doctor`. The doctor checks the rest of this list and the minimum versions, and prints the command that fixes each problem. `bun run dev` runs the same check first, and stops if a tool that it needs is missing or too old.

- [Bun](https://bun.sh) 1 or later.
- Bash 4 or later. The dump scripts use `mapfile`, and macOS ships Bash 3.2. On a Mac, run `brew install bash`.
- A Docker engine, such as [OrbStack](https://orbstack.dev) or Docker Desktop on a Mac. Postgres, Elasticsearch, and the app containers run through Compose.
  - It needs Docker Compose 2.24.4 or later, because `docker-compose.dev.yml` uses the `!override` tag.
  - Give Docker at least 4 GB of memory. The dev Elasticsearch alone can use 3 GB.
  - The API reaches both databases through their published `localhost` ports. It never uses OrbStack's `*.orb.local` container DNS.
  - The reason is speed: a large search response took 78 ms there on a kept-alive connection, and 23 ms through the published port.
- Rust through [rustup](https://rustup.rs). `rust-toolchain.toml` pins the version, and only rustup reads that file. `bun run check:rust-toolchain` checks that the pin matches.
- [ShellCheck](https://www.shellcheck.net), [actionlint](https://github.com/rhysd/actionlint), and [hadolint](https://github.com/hadolint/hadolint), for the static phase of the gate. Install them with `brew install shellcheck actionlint hadolint`.
- Optional: the [GitHub CLI](https://cli.github.com). Install `gh`, then run `gh auth login`. It lets an AI agent read and file issues and open pull requests itself (`.github/CONTRIBUTING.md`, "Working with an AI agent").

## Getting started

```bash
bun install
bun run dev
```

`bun run dev` (`scripts/dev/run.ts`) does these steps, in order:

1. It checks the prerequisites (`bun run doctor`, without the optional tools). It stops if a tool to run the site is missing or too old, and only warns about the tools for committing.
2. It picks the dump: the bundled `data/db/0000_default/` sample of 100 poems, even when a full dump is on disk. `--dump <folder>` picks a full dump instead, and downloads it with `bun run data:pull` if it is missing. See "Working with the full corpus" below.
3. It starts Postgres and Elasticsearch with Docker Compose, and restores the dump on a new volume. `docker-compose.dev.yml` mounts only the chosen folder at `/dumps`.
4. It runs the search indexer once to fill Elasticsearch.
5. It creates `apps/web/.env` if that file does not exist. The file holds only `PUBLIC_API_URL`, which points at the local API.
6. It builds the API and the MCP server (`cargo build -p qafiyah-api -p qafiyah-mcp`), and starts the API, then the MCP server, then the web app. The combined log gives each one a prefix and a color. To also start the inspector after the web app is ready, pass `--inspector`.

In a terminal, each step shows a live line with its elapsed time. Where there is something to report, the line also shows it:

- each container's state, and the restore phase of the database
- the build step of the indexer image, then the poems and poets written so far
- the crate that cargo compiles, with a count

When you pipe the output, each step prints one plain line.

The first run is the slow one. With the 100-poem sample, the restore and the indexing take seconds, so the two compiles and the download of the Postgres and Elasticsearch images take most of the time. Later runs keep the volumes, the indexer image, and `target/`. So they skip the restore and the indexing, and take about a minute. Most of that minute is the wait for Elasticsearch to report healthy.

### Working with the full corpus

Use the full corpus only for data work: a corpus fix, a new dump, or a check that needs every poem. It needs the four `DATA_*` values in `.env` (see below).

```bash
bun run dev --dump 0046_15_10_2026        # a fresh database restores this dump
bun run db:reset --dump 0046_15_10_2026   # switch an existing database to it
bun run db:reset                          # back to the sample
```

A database keeps its data until `db:reset`, so `bun run dev` without `--dump` does not undo a full dump. A fresh full dump takes about 20 minutes on an Apple silicon laptop (0046): about 7 minutes to restore it, and about 12 minutes to build the search index. It also takes about 3.5 GB of disk.

Default URLs:

| Service       | URL                       |
| ------------- | ------------------------- |
| Web           | http://localhost:4321     |
| API           | http://localhost:8787     |
| MCP server    | http://localhost:8788/mcp |
| Inspector     | http://localhost:4322     |
| Postgres      | localhost:5434            |
| Elasticsearch | localhost:9201            |

The ports come from `config.ts` (`DEV_WEB_PORT`, `DEV_API_PORT`, `DEV_MCP_PORT`, `DEV_INSPECTOR_PORT`, `DEV_POSTGRES_PORT`, `DEV_ES_PORT`, `DEV_EDGE_PORT`). Each app also reads its port from an environment variable: `PORT` for the API, `MCP_PORT` for the MCP server, `WEB_PORT`, `INSPECTOR_PORT`, and `DEV_POSTGRES_PORT`.

To stop every running process, press `Ctrl-C`. The runner sends SIGTERM, then SIGKILL after 5 seconds.

## Everyday commands

```bash
bun run dev:preflight   # check that the dev ports are free (bun run dev runs it for you)
bun run db:up           # start only Postgres and Elasticsearch (docker compose up -d --wait)
bun run db:reset        # wipe the local Postgres volume and restore the dump again
bun run down            # stop the Docker Compose stack
bun run clean           # stop leftover astro, qafiyah-api, and qafiyah-mcp processes from an earlier run
bun run reindex         # rebuild the indexer image, then rebuild the Elasticsearch indices from Postgres
```

`bun run dev` records no metrics, because the API, the MCP server, and the website run on the host without `METRICS_PORT` or an OTLP endpoint. To see the observability dashboards locally, run the full Docker stack:

- `bun run smoke:stack` runs it, and leaves nothing running at the end.
- For a stack that stays up, see `apps/observability/AGENTS.md`.
- Open Grafana at `http://127.0.0.1:3300`, and sign in as `admin` with `GRAFANA_ADMIN_PASSWORD`.

The sample dataset needs no credentials. `scripts/dev/compose.sh` and `scripts/dev/run.ts` set a default for every dev database and Elasticsearch password, so a new clone has no `.env` file at all. Two things change this:

- **A real dump.** Email dumps@qafiyah.com for a read-only token (`data/README.md`). Then put its four `DATA_*` values in a root `.env` file, which git ignores. `bun run dev --dump <folder>` and `bun run db:reset --dump <folder>` then download that dump (see "Working with the full corpus").
- **The maintainers' age key.** With this key, `bun run secrets:pull` generates `.env` from `secrets/dev.enc.env`, which holds the maintainer token of the data bucket. On such a machine, never edit `.env` by hand. See `docs/deployment/secrets.md`. Without the key, the `secrets:*` commands do not apply, and you write `.env` yourself.

## Quality gates

```bash
bun run lint              # oxlint, type-aware, fixes what it can
bun run lint:check        # oxlint, type-aware, read-only, warnings fail (the gate runs this)
bun run format            # oxfmt, then prettier on .astro files, writes the files
bun run format:check      # the same two, read-only (the gate runs this)
bun run check:shell       # ShellCheck on every tracked shell script, warnings fail
bun run check:workflows   # actionlint on .github/workflows
bun run check:dockerfiles # hadolint on every Dockerfile, warnings fail
bun run check:sql         # the PostgreSQL 18 parser on every .sql file, and squawk on the migrations
bun run types             # turbo run types (TypeScript, for each app and package)
bun run test              # turbo run test (TypeScript, for each app and package)
bun run rust:fmt          # cargo fmt --check
bun run rust:lint         # cargo clippy on the workspace with -D warnings; cargo's own warnings also fail
bun run rust:test         # cargo test on the workspace
bun run rust:test:db      # the database-backed API and indexer tests against the dev stack (needs Docker)
bun run smoke:dev         # black-box HTTP probes against a dev server that the run manages
bun run docs:diagrams     # render docs/architecture/workspace.dsl to the C4 SVGs (needs Docker); docs:diagrams:check compares them
bun run ci                # the full gate; --no-docker skips the diagram, db, and smoke phases, --docker-only runs only those
```

The "Static checks" section of `docs/exceptions.md` explains the deliberate, non-obvious behavior of the static checks.

`smoke:dev` ends with `bun run api:conformance`. This sends every documented API example to the dev API through Schemathesis in Docker.

The smoke runs leave your dev environment as they found it:

- `smoke:dev` starts its own `bun run dev`, and stops it at the end. If the dev web and API already answer, it uses them, and leaves them running.
- The `stack` phase of `bun run ci` (`smoke:stack`) runs the production-mode stack in the same Compose project, and removes it at the end. It then starts again the dev `db` and `elasticsearch` containers that were running before.
- When a check fails or you press Ctrl-C, `bun run ci` stops only the tasks that it started, because each task runs in its own process group. So a running `bun run dev`, here or in another worktree, keeps running.

`rust:test:db` starts the dev Postgres and Elasticsearch, and runs the one-shot indexer. It then runs `apps/api/tests/db.rs` and the indexer's tests, with the same connection strings that `bun run dev` uses. Without the `QAFIYAH_TEST_*` variables, the database-backed tests skip themselves. That is why plain `cargo test` needs no infrastructure. The API under test keeps its 5 second statement timeout. The harness's own checks, some of which scan the whole corpus, run on a separate pool with 60 seconds (#233).

Run `bun run ci` before you open a pull request, because it is the full gate. GitHub Actions runs the same gate, and skips a Docker phase when the change touches nothing that the phase uses (`docs/topology.md`, "CI/CD topology"). If Docker is not available, `bun run ci --no-docker` skips the Docker steps: the diagram check, the database-backed tests, and both smoke runs. `bun run ci --docker-only` runs only those steps. The stack smoke builds the three images through `compose up --build`, so a full run also proves that every image builds.

## Committing

Commit messages follow `docs/pull-requests.md`: one subject line, and nothing else.

- The pre-commit hook (`.husky/pre-commit`) runs lint-staged on the staged files only, in about a second. It runs oxlint, oxfmt, prettier for `.astro` files, rustfmt, and ShellCheck, as the `lint-staged` block in `package.json` sets.
- The pre-push hook (`.husky/pre-push`) runs `bun run ci --no-docker`, usually in less than a minute.
- GitHub Actions runs the gate on every push and pull request to `main` or a version branch. It runs each Docker phase only when the change touches what that phase uses.
- `bun run deploy` refuses a commit whose CI run did not pass. A skipped phase is not a failure. After a release, it accepts the release pull request's passed run, because `main` then has exactly the same files.
- The Docker jobs keep image layers in the GitHub Actions cache (`docker-compose.ci.yml`, which `scripts/dev/compose.sh` adds when `COMPOSE_CI_FILE` is set). So a job rebuilds only the images whose files changed. The runs on version branches fill the cache that pull requests into them read.
- If you already ran the gate, `HUSKY=0 git commit ...` or `HUSKY=0 git push ...` skips the hook.

Both hooks first run `scripts/check/commit-identity.sh`. It is an optional guard against commits with the wrong email:

- It does nothing until you turn it on for a clone: `git config --local qafiyah.allowedEmail "$(git config --local user.email)"`. The address stays in `.git/config`, which is never committed.
- After that, it refuses a commit unless its author and committer use that address.
- It refuses a push if a commit that is not yet on a remote has another address as author, committer, or `*-by:` trailer (such as `Co-authored-by:`).
- It does not check commits that are already on a remote, because they are already public. An example is a squash merge that GitHub made on `main`, after you merge `main` into a branch.
- It reads the key with `git config --local`, so a `git -c` override cannot satisfy it. `--no-verify` and `HUSKY=0` skip it, as they skip every hook.
- On the server side, GitHub's "Block command line pushes that expose my email" setting covers the addresses on your account.

The hooks also run `scripts/check/forbidden-terms.sh`, an optional guard against publishing names that must stay private:

- It does nothing until a clone has a list in `.git/info/forbidden-terms`: one extended regular expression on each line, matched without regard to case, with `#` for comments. The file is inside `.git`, so it is never committed.
- The pre-commit hook refuses a staged text file or file name that matches. The commit-msg hook (`.husky/commit-msg`) refuses a matching commit message. The pre-push hook refuses a new commit whose message or added lines match.
- `forbidden-terms.sh text` checks text on standard input, so a local tool can run it before it publishes an issue or a pull request.
- Its messages never print the matched term.

`AGENTS.md` is the guide for its directory. The `CLAUDE.md` and `GEMINI.md` files next to each one are committed symlinks to it, so every agent harness reads the same file. After you add an `AGENTS.md`, run `bun run agents:link` to create the links. On Windows, check out with `git config core.symlinks true` from a Developer Mode or admin shell. Otherwise, the links appear as one-line text files.

## Troubleshooting

- **Port already in use:** run `bun run clean` to stop leftover `astro` and `qafiyah-api` processes from an earlier run, then try again.
- **Docker not running:** start Docker Desktop or OrbStack. `dev` and `ci` both check for Docker first, and stop with a clear message if it is not running.
- **"Postgres unreachable" warning during preflight:** run `bun run db:up`.
- **Script tests time out, often while `cargo test` runs:** an endpoint security agent with application control can hold each new process for 5 to 12 seconds while it checks newly built binaries. The script tests launch `git` and shell scripts, so `bun run test:scripts` allows each test 30 seconds. If a test still times out, measure how long `git --version` takes while `cargo test --workspace` runs.
