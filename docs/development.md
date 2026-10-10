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
2. It resolves and decrypts a database dump (`scripts/db/resolve-dump.sh`). If no passphrase is set, it uses the bundled `data/db/0000_default/` sample of 100 poems. See `data/db/README.md`.
3. It starts Postgres and Elasticsearch with Docker Compose, and restores the dump on a new volume.
4. It runs the search indexer once to fill Elasticsearch.
5. It creates `apps/web/.env` if that file does not exist. The file holds only `PUBLIC_API_URL`, which points at the local API.
6. It builds and starts the API (`cargo build -p qafiyah-api`), then the web app. The combined log gives each one a prefix and a color. To also start the inspector after the web app is ready, pass `--inspector`.

In a terminal, each step shows a live line with its elapsed time. Where there is something to report, the line also shows it:

- each container's state, and the restore phase of the database
- the build step of the indexer image, then the poems and poets written so far
- the crate that cargo compiles, with a count

When you pipe the output, each step prints one plain line.

The first run is the slow one. On an Apple silicon laptop with a full dump (0037, about 349,000 poems), it took about seven and a half minutes to "web ready":

- about 1 minute 45 seconds to restore the dump into a new volume
- about 4 minutes 45 seconds for the indexer, which builds its image and then indexes every poem
- about 50 seconds to compile the API
- the download of the Postgres and Elasticsearch images, on the very first run only

With the 100-poem sample, the restore and the indexing take seconds, so the two compiles take most of the time. Later runs keep the volumes, the indexer image, and `target/`. So they skip the restore and the indexing, and take about a minute. Most of that minute is the wait for Elasticsearch to report healthy.

Default URLs:

| Service       | URL                   |
| ------------- | --------------------- |
| Web           | http://localhost:4321 |
| API           | http://localhost:8787 |
| Inspector     | http://localhost:4322 |
| Postgres      | localhost:5434        |
| Elasticsearch | localhost:9201        |

The ports come from `config.ts` (`DEV_WEB_PORT`, `DEV_API_PORT`, `DEV_INSPECTOR_PORT`, `DEV_POSTGRES_PORT`, `DEV_ES_PORT`, `DEV_EDGE_PORT`). Each app also reads its port from an environment variable: `PORT` for the API, `WEB_PORT`, `INSPECTOR_PORT`, and `DEV_POSTGRES_PORT`. `--worktree` sets these variables with an offset (see "Working in a git worktree").

To stop every running process, press `Ctrl-C`. The runner sends SIGTERM, then SIGKILL after 5 seconds.

## Everyday commands

```bash
bun run dev:preflight   # check that the dev ports are free (bun run dev runs it for you)
bun run db:up           # start only Postgres and Elasticsearch (docker compose up -d --wait)
bun run db:reset        # wipe the local Postgres volume and restore the dump again
bun run down            # stop the Docker Compose stack
bun run clean           # stop leftover astro and qafiyah-api processes from an earlier run
bun run reindex         # rebuild the indexer image, then rebuild the Elasticsearch indices from Postgres
```

`bun run dev` records no metrics, because the API and the website run on the host without `METRICS_PORT` or an OTLP endpoint. To see the observability dashboards locally, run the full Docker stack:

- `bun run smoke:stack` runs it, and leaves nothing running at the end.
- For a stack that stays up, see `apps/observability/AGENTS.md`.
- Open Grafana at `http://127.0.0.1:3300`, and sign in as `admin` with `GRAFANA_ADMIN_PASSWORD`.

The sample dataset needs no credentials. `scripts/dev/compose.sh` and `scripts/dev/run.ts` set a default for every dev database and Elasticsearch password, so a new clone has no `.env` file at all. Two things change this:

- **A real dump.** Email dumps@qafiyah.com for a passphrase (`data/db/README.md`). Then put `DUMP_KEY__<dump-dir>=<passphrase>` in a root `.env` file, which git ignores. `bun run db:reset` then restores that dump instead of the sample. To check a passphrase without a restore, run `bun run dump:key:check`.
- **The maintainers' age key.** With this key, `bun run secrets:pull` generates `.env` from `secrets/dev.enc.env`, and `bun run dump:key:set` stores dump passphrases. On such a machine, never edit `.env` by hand. See `docs/deployment/secrets.md`. Without the key, the `secrets:*` commands do not apply, and you write `.env` yourself.

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

## Working in a git worktree

You may work in a git worktree, not the primary checkout (for example, under `.worktrees/`). To isolate it from a dev stack that already runs in the primary checkout, **pass `--worktree` to `dev`, `db:reset`, and `reindex`**:

```bash
bun run dev --worktree
./scripts/dev/db-reset.sh --worktree
./scripts/es/reindex.sh --worktree
bun run smoke:dev --worktree     # against a --worktree dev server
```

Without `--worktree`, a worktree uses the same Docker Compose project name and ports as the primary checkout. So if you run `dev` in both at the same time, they collide. With `--worktree`, the Compose project name and every port get an offset that comes from the worktree's directory name. This covers Postgres, Elasticsearch, the edge gateway, the API, the web app, and the inspector. Several worktrees can then run their own dev stacks side by side.

Being in a worktree does not turn on `--worktree`. `scripts/dev/worktree.ts` only _detects_ a worktree. Nothing isolates the ports or the Compose project unless you pass the flag. If you pass `--worktree` in the primary checkout, the command fails with "nothing to isolate against".

On the first `bun run dev --worktree`, the worktree gets a copy of the primary checkout's root `.env`, unless it already has one. `apps/web/.env` is generated with the worktree's own API port.

## Committing

Commit messages follow `docs/pull-requests.md`: one subject line, and nothing else.

- The pre-commit hook (`.husky/pre-commit`) runs lint-staged on the staged files only, in about a second. It runs oxlint, oxfmt, prettier for `.astro` files, rustfmt, and ShellCheck, as the `lint-staged` block in `package.json` sets.
- The pre-push hook (`.husky/pre-push`) runs `bun run ci --no-docker`, usually in less than a minute.
- GitHub Actions runs the gate on every push and pull request to `main` or a version branch. It runs each Docker phase only when the change touches what that phase uses.
- `bun run deploy` refuses a commit whose CI run did not pass. A skipped phase is not a failure.
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
