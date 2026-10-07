# Code Conventions

These conventions apply to every language in the repo. For rules that apply to one language only, see `docs/typescript-conventions.md` and `docs/rust-conventions.md`.

## Architecture

- Keep the core pure and put mutations at the edges. Pass dependencies as arguments; use no globals or singletons. Compute derived values instead of storing them.
- Add an abstraction only when the code repeats three times, is hard to test, allows invalid states, or makes immutable updates painful. Delete dead code.
- Validate input where it enters, trust the types after that, and fail loudly at boundaries. Dependencies point inward.
- Put one concern in each file, and keep it next to its only user until two or more places share it. TypeScript and Rust draw the file boundary differently; see their conventions.
- Design an interface for its caller, and hide the internals. Prefer push (events, callbacks) to pull.

## Naming

- A name shows intent and uses the domain vocabulary. Use the same name on both sides of a boundary. If an abstraction is hard to name, do not add it.
- Booleans start with `is`, `has`, or `can`. Functions are a verb and a noun. React handlers start with `handle`, and their props with `on`.
- Use only these abbreviations: `req`, `res`, `id`, framework idioms (`ref`, `props`, `ctx`), `ok` and `err`, and `i` for a loop.

## Errors

- Fallible logic returns `Result<T, E>` (`neverthrow` in TypeScript, `thiserror` in Rust). Use `throw` or `panic!` only for the unexpected.

## Generated files

- A script that produces a file from another source of truth writes it under a `generated/` directory, never next to hand-written source. The web app uses `apps/web/src/lib/generated/`, and the API crate uses `apps/api/generated/`.
- Each kind of generated file gets its own subdirectory, named for its source: `generated/openapi/`, `generated/well-known/`, `generated/es/`. Never put a file directly in `generated/`, because that makes the same flat dumping ground one level down.
- Keep the `.gen.<ext>` suffix, so a generated file is clear even outside its folder. A file format with no comments, such as a committed JSON snapshot, is the exception: its commit message and its generator document it instead.
- The generating script holds the exact output path as a constant. To move a generated file, change that constant and any header text that contains the old path, then run the generator again. Do not edit the moved file by hand.

## Comments

- Code has no comments. Names, types, tests named as full sentences, and the nearest `AGENTS.md` carry the intent. A module's non-obvious "why" goes in `docs/exceptions.md`, not in a doc comment. Do not use `//!` or JSDoc, and use `///` only as the next point allows.
- One kind of doc comment is allowed: `///` on the fields of the API's public types. These are the response types in `apps/api/src/contract/`, `apps/api/src/envelope.rs`, and `apps/api/src/routes/`, and `ProblemDetail` in `apps/api/src/openapi.rs`. They are also the request parameter structs (`#[derive(utoipa::IntoParams)]`) in `apps/api/src/routes/` and `apps/api/src/params.rs`. utoipa has no other way to describe a field, so these lines are the published API documentation, not notes about the code. They say what the field means to a caller and follow the "Writing" rules below. They also change the committed OpenAPI document, so the drift test catches every change.
- Code may have two other kinds of comment, each on one line. The first is a lint directive (`// oxlint-disable-next-line ...`) with its reason on the same line. The second is a "why" that names a constraint outside the code, such as a query planner rule, a Postgres restriction, or a runtime quirk. Use it only when a reader cannot find that constraint in the names, tests, or docs. If it needs a second line, it is documentation and goes in `AGENTS.md`.
- Configuration that has no types or tests to carry intent may have short comments on directives that are not obvious. This applies to `nginx*.conf`, `docker-entrypoint.sh`, the Compose files, Dockerfiles, and shell scripts. Prefer a one-line header that points to the doc that explains the file. A comment that describes a plan or a past state is a bug: fix it when the code changes.
- Generated files keep their generator header.

## Writing

These rules apply to docs, `AGENTS.md` files, issues, pull requests, commit messages, and the `///` field descriptions. They go about 80% of the way to [ASD-STE100](https://www.asd-ste100.org) (Simplified Technical English), not the full specification. Many readers of this repo read Arabic first and English second.

- Write an instruction in 20 words or fewer, and a description in 25 words or fewer.
- Put one instruction in each sentence, and write it as a command: "Run `bun run dev`."
- Use the active voice.
- Use one word for one meaning, and use the same word every time. The terms in `docs/domain.md` are the names for the domain.
- Keep one topic in each paragraph, and use six sentences or fewer.
- Do not use em-dashes. Use a period, a comma, or parentheses.
