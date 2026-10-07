# Rust Conventions

- Tooling enforces the lints. Each crate takes them from the workspace with `[lints] workspace = true`. The build fails on any violation of `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, or `cargo test --locked`.
- Production code never panics. These lints are denied: `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `string_slice`, `unreachable`, `todo`, `unimplemented`, and `dbg_macro`. `clippy.toml` allows `unwrap`, `expect`, `panic`, and indexing in tests.
- Fix a finding. Do not suppress it.
  - The only allowed suppression is `#[expect(lint, reason = "...")]`, on the one site, with the invariant in its reason.
  - It fails again when it is no longer necessary.
  - `allow_attributes` is denied.
- Make numeric code explicit.
  - Use `checked_*` and return the error, or use `saturating_*` or `wrapping_*` where the value has a known bound.
  - Convert with `TryFrom`, `.cast_signed()`, or `.cast_unsigned()`, never with `as`.
  - `as_conversions`, `cast_possible_truncation`, `cast_sign_loss`, `cast_possible_wrap`, and `arithmetic_side_effects` are denied. `overflow-checks` stays on in release builds.
- Name files and modules in snake_case.
  - A pure domain module holds the logic, with no I/O.
  - Handlers stay thin.
  - Each external system gets one module, which owns its wire format.
- A file is a module, and privacy applies to each module. So a module holds one concern, with all of its items. An example is a domain's types, rules, and repository trait together, or an adapter with its row types.
  - Split a module into submodules when it holds two concerns, or when it becomes hard to navigate. Never split by the number of items.
  - Never make an item more visible only to move it into its own file.
- Write inline tests in `#[cfg(test)] mod tests`. Give each test a full-sentence name (for example, `a_single_term_is_counted_from_its_stats_table`), never `test_x`.
