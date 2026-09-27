# Code Conventions

Cross-language engineering conventions (not TypeScript- or Rust-specific, see `docs/typescript-conventions.md` / `docs/rust-conventions.md` for those).

## Architecture

- Pure core, mutations at edges. Inject deps as args; no globals/singletons. Compute derived, don't store it.
- Abstract only at 3x repeat, hard to test, invalid states possible, or painful immutable updates. Delete dead code.
- Validate at entry, trust types downstream, fail loudly at boundaries. Deps flow inward.
- One primary export per file (4+ → split). Co-locate until shared 2+, then `types/`|`utils/`.
- Design interfaces for the caller; hide internals. Push (events/callbacks) > pull.

## Naming

- Reveal intent; domain vocabulary; consistent across boundaries. Skip abstractions that resist naming.
- Booleans `is/has/can`; fns verb+noun; React handlers `handle`, props `on`. Abbreviations only: `req`/`res`/`id`, framework idioms (`ref`, `props`, `ctx`), `ok`/`err`, loop `i`.

## Errors

- `Result<T,E>` for fallible logic (`neverthrow` in TS, `thiserror` in Rust); `throw`/`panic!` only for the unexpected.

## Generated files

- Anything mechanically produced by a script from another source of truth (not hand-authored) lives under a `generated/` directory, never flat alongside hand-written source: `apps/web/src/lib/generated/` for the web app, `apps/api/generated/` for the API crate.
- One subdirectory per kind of generated artifact, named for what it's generated from, e.g. `generated/openapi/`, `generated/well-known/`, `generated/es/`. Files never sit directly under `generated/` itself. That just recreates the flat dumping-ground problem one level down.
- Keep the `.gen.<ext>` suffix (or, for a non-code artifact like a committed JSON snapshot, a `// Do not edit` / commit-message-documented header where the format allows comments) so a generated file is unambiguous even outside its folder.
- The generating script owns the exact output path as a constant; when moving a generated file, update that constant (and any header text embedding the old path) and rerun the generator rather than hand-editing the moved file.

## Comments

- Code carries no comments. Intent lives in names, types, tests named as full sentences, and the nearest `AGENTS.md`. A module's non-obvious "why" belongs in `docs/exceptions.md`, not in a doc comment (`///`, `//!`, and JSDoc are not used).
- Two exceptions in code, both single-line: a lint directive (`// oxlint-disable-next-line ...`) with its reason on the same line, and a WHY that names a constraint outside the code (a query planner rule, a Postgres restriction, a runtime quirk) that a reader could not recover from names, tests, or docs. If it needs a second line, it is documentation and goes in `AGENTS.md`.
- Configuration with no types or tests to carry intent (`nginx*.conf`, `docker-entrypoint.sh`, the Compose files, Dockerfiles, shell scripts) may carry short comments on non-obvious directives, and a one-line header pointing at the doc that explains the file is preferred over re-explaining. A comment that describes a plan or a past state is a bug: fix it the moment the code moves on.
- Generated files keep their generator header.

## Style

- No em-dashes; use a period, comma, or parentheses.
