# Strict presentation body schemas

**Gate:** IPC-01 remains **Not Run**. This is a focused implementation check, not an IPC-01 conformance result.

**Scope:** P4 package 0.3.2 strict protocol/1 schemas for hello, ready and state replacement bodies; unknown fields and invalid snapshot ranges are rejected. Renderer controls still have no authorization/dispatch handler.

**Environment:** Arch Linux x86_64; Rust/Cargo 1.99.1. `raw.txt` records the command, environment, test run and lint output.

## Executed

- `cargo fmt --all -- --check` — passed.
- `cargo test --locked --workspace` — 51 passed, 0 failed, 1 namespace-only test ignored in the normal run. The IPC-specific schema and direction regressions passed.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.

## Limits

No real Godot renderer was launched. This run does not cover production lifecycle integration, separate-UID peer authentication over the reachable service socket, authorization policy integration, sustained flood/contention, slow-reader high-water resource bounds, coverage-guided fuzzing, or full IPC-01. No conformance gate or acceptance condition is marked Passed.
