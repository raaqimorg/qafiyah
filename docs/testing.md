# Testing Conventions

- Test behavior, with one concept in each test.
- Use unit tests for pure functions, and integration tests for side effects.
- Mock only the boundaries: the network, the database, and time.

For where Rust inline tests go and how to name them, see `docs/rust-conventions.md`.
