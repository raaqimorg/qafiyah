# Scripts Agent Guide

This directory holds the repo tooling and the CI checks: Bun and TypeScript, with some bash wrappers.

- Every routine entry point is a `bun run <name>` script in the root `package.json`. Use those names, not paths, so the flags and the working directory are handled in one place.
- `bun run test:scripts` and `bun run types:scripts` cover this directory. Tests are next to their module, as `*.test.ts`.
- `scripts/ci.ts` is the whole quality gate (`bun run ci`).
  - `--no-docker` skips the Docker phases, `--docker-only` runs only those, and `--phase <name>` runs one phase. `ci/phases.ts` selects the phases from these flags.
  - GitHub Actions runs it on every push and pull request. The non-Docker phases run in one job. Each Docker phase runs in its own job, and is skipped when the change touches nothing that the phase uses.
- `scripts/observe.sh` (`bun run observe`) opens an SSH port forward to the production Grafana (`apps/observability/AGENTS.md`).

| Directory     | What is here                                                                                          | `bun run` names                                                                                                                    |
| ------------- | ----------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `check/`      | repo-shape checks and the wrappers for the external linters (see below)                               | `check:*`                                                                                                                          |
| `dev/`        | the local stack (see below)                                                                           | `dev`, `doctor`, `dev:preflight`, `db:up`, `db:reset`, `up`, `down`, `clean`, `dump:key:*`, `agents:link`, `quota`, `rust:test:db` |
| `db/`         | the dump lifecycle, the Diesel schemas, and the SQL functions (see below)                             | `db:reseed`, `db:schema`, `db:schema:check` (see below for the rest)                                                               |
| `es/`         | Elasticsearch: reindex (dev and prod), live-index snapshot and diff, the query-builder drift snapshot | `reindex`, `reindex:prod`, `es:*`                                                                                                  |
| `docs/`       | the C4 diagrams (see below)                                                                           | `docs:diagrams`, `docs:diagrams:check`                                                                                             |
| `openapi/`    | the OpenAPI document snapshot, and the generated TypeScript client types                              | `openapi:*`                                                                                                                        |
| `well-known/` | renders the `well-known/*` templates into the web app's generated module                              | `well-known:generate*`                                                                                                             |
| `secrets/`    | the SOPS and age wrappers, and the secrets schema (`schema.ts` is the single list of allowed keys)    | `secrets:*`                                                                                                                        |
| `smoke/`      | black-box HTTP probes against a dev or prod stack (see below)                                         | `smoke:dev`, `smoke:prod`                                                                                                          |
| `api/`        | the Schemathesis conformance run, and the comparison of two API builds (see below)                    | `api:conformance`, `api:compare`                                                                                                   |
| `deploy/`     | the VPS deploy, and the image build for each service that the Images workflow uses                    | `deploy`, `build:images`                                                                                                           |
| `ci/`         | phase selection for `ci.ts` (`--phase`, `--no-docker`, `--docker-only`)                               | `ci`                                                                                                                               |
| `accounts/`   | the accounts-database backup, and the systemd timer that runs it on the VPS                           | none (a systemd timer, see `docs/deployment/services.md`)                                                                          |
| `corpus/`     | offline validation of the classical form, and the verse row repair (see below)                        | `corpus:*`                                                                                                                         |
| `lib/`        | shared helpers (see below)                                                                            | none                                                                                                                               |

## Details

- `check/` holds these checks:
  - the import boundaries between apps, and the constants that must stay the same in `config.ts` and the Rust crates
  - file naming, and parent-relative imports
  - the Rust toolchain pin and its keys
  - ShellCheck over every tracked shell script (found by `.sh` or by shebang), hadolint over every Dockerfile, and the SQL syntax check
  - the clippy wrapper behind `rust:lint`
  - the optional commit identity guard and forbidden-terms guard. The husky hooks call them directly (`docs/development.md`, "Committing").
- `dev/` holds these parts:
  - `run.ts` (`bun run dev`), and the Compose wrapper with the default dev credentials
  - the prerequisites check (`doctor.ts`, `bun run doctor`): each tool and its minimum version, in three groups (to run the site, to commit, optional). `bun run dev` runs the first two groups first. The checks take an injected `probe`, so the tests spawn nothing (`doctor-checks.ts`)
  - the port preflight, the database reset, and the management of dump keys
  - worktree isolation, and the `CLAUDE.md` and `GEMINI.md` symlinks
  - a rate-limit probe, and the runner for the database-backed API and indexer tests
- `db/` holds these parts:
  - the dump lifecycle: resolve and decrypt, restore (on the first boot, and in place by `db:reseed`), split, encrypt, the fallback sample, and the before and after state diffs
  - `diesel-schema.ts`, which writes the Diesel schemas (`crates/corpus`, and the API's accounts) from the dev databases
  - `sql/`: the refresh functions and the sample query

  Compose mounts most of these scripts, or the `dev/` scripts and the `local-db-edit` skill call them. `data/db/MAINTAINERS_GUIDE.md` documents the dump scripts.

- `docs/` exports `docs/architecture/workspace.dsl` with Structurizr, and renders it with PlantUML into `docs/architecture/generated/structurizr/`. `diagrams.ts` pins both images. `diagram-files.ts` holds the rules for names, headers, and stale files.
- `smoke/` probes a dev or prod stack from outside.
  - A run that starts or reuses its own server sends three searches first. So the warm-up of Elasticsearch after a restart does not count against the probes.
  - Every real poem or poet that a probe names comes from `smoke/fixtures.ts`, which the fallback sample is built from. The `llms.txt` link probes use the examples in the API docs, and `fixtures.test.ts` keeps those in the sample.
  - On the stack surface, `observability.ts` also checks the Prometheus and Grafana chain from end to end.
- `api/` holds two tools:
  - `conformance.ts` runs Schemathesis in Docker (its image is pinned there, and its settings are in `schemathesis.toml`). It sends the documented examples to every OpenAPI operation. It fails unless each one answers 2xx and matches the spec.
  - `compare.ts` compares two running API builds across every endpoint, byte for byte and by latency. It is for refactors such as #138.
- `corpus/` reads the dev database, and writes under `reports/`, which git ignores.
  - The classical-form validation: `export-corpus.sh`, `verify-classical.ts`, and `apply-verdicts.ts`.
  - The verse row repair: `repair-rows.ts`, over `row-repair.ts`.
    - `plan` measures each row against the half-line of the poem's meter, or the poem's own half-line when the two agree.
      - It splits a verse with no `*`, or with its `*` far from the middle, where a copy in the corpus shows the split.
      - It joins a stray `*` inside one half-line, in a regular poem of a four-foot meter. A sample of these joins goes to review.
      - It merges half-line pairs, and skips the rows that an earlier review answered (its third argument).
      - It exports the rest as CSV batches for review, and lists the rows longer than a verse in `long.csv`.
    - `score` measures the review on verses that are known to be good.
    - `sql` turns the accepted answers into one transaction, which never edits a shared verse. It refuses the stray joins until at least 98% of their sample is correct.
      - It also takes hand edits from `hand-edits.json`: the final rows of a poem, and the texts dropped from it (notes and headings). A hand edit owns its poem, so the other fixes skip that poem. The letters must stay the same, apart from the dropped texts.
      - It also takes title fixes from `titles.json`: a poem's old title and its new one. A title holds letters and single spaces only, and it changes only while the poem still holds the old title.
  - The promotion of a new primary reading: `promote-reading.ts`, over `reading-promotion.ts`. It turns a transcription (JSON, kept out of git) into one transaction. The transaction moves each primary's old text to a new recension, and writes the new text and its `source` into the primary.
- `lib/` holds the shared helpers: the repo root, file walking, the list of tracked files, the runner for external linters, snapshot writing, cargo dumps, the remote and VPS helpers, and the tag of the database container.
