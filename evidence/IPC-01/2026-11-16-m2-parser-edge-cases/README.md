# M2 IPC parser edge-case checks

**Gate:** IPC-01 remains **Not Run**. This records local implementation checks, not a conformance result.

**Source commit:** `40cc38365856c3cfd4069145ea13bfc2e074b0bb`.
**Environment:** Linux x86_64, Arch Linux kernel 7.2.14; Rust/Cargo 1.99.1.

## Executed

- `cargo fmt --all -- --check` — passed.
- `cargo test --locked --workspace` — 29 tests passed, 0 failed. Eight IPC tests cover valid framing, zero/oversized/truncated header and body rejection, invalid UTF-8/JSON, trailing JSON, duplicate keys, unknown fields, invalid protocol version/kind/method, uint64 sequence overflow, UUID validation, private socket setup/cleanup and Linux peer credentials, deadlines, and session replay order.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.
- `git diff --check` — passed.

These tests cover parser primitives only. They do not execute a full service, same-UID renderer binding, dispatch authorization, restart lifecycle, backpressure, fuzzing, or the complete IPC-01 procedure. No acceptance condition or conformance gate is marked Passed.
