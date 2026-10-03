# Rust Conventions

- Lints are enforced by tooling and inherited per crate with `[lints] workspace = true`. The build fails on any violation: `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo test --locked`.
- Production code never panics: `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `string_slice`, `unreachable`, `todo`, `unimplemented`, `dbg_macro` are denied. `clippy.toml` allows `unwrap`/`expect`/`panic`/indexing inside tests.
- Fix a finding, don't suppress it. The only suppression is `#[expect(lint, reason = "...")]`, scoped to the one site, with the invariant stated; it errors again once it is no longer needed. `allow_attributes` is denied.
- Numeric code is explicit: `checked_*` and propagate the error, or `saturating_*`/`wrapping_*` where bounded. Convert with `TryFrom`/`.cast_signed()`/`.cast_unsigned()`, never `as`. `as_conversions`, `cast_possible_truncation`, `cast_sign_loss`, `cast_possible_wrap`, `arithmetic_side_effects` are denied, and `overflow-checks` stays on in release.
- snake_case files/modules. Pure domain module holds logic with no I/O; handlers stay thin; each external system gets one module owning its wire format.
- A file is a module, and privacy is per module, so a module holds one concern with all of its items: a domain's types, rules, and repository trait together, or an adapter with its row types. Split into submodules when a module holds two concerns or grows hard to navigate, never by item count, and never widen an item's visibility only to move it into its own file.
- Inline tests: `#[cfg(test)] mod tests`, named as full sentences (e.g. `a_single_term_is_counted_from_its_stats_table`), never `test_x`.
