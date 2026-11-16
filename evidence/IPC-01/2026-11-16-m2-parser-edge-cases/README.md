# M2 IPC parser edge-case checks

**Gate:** IPC-01 remains **Not Run**. This records local implementation checks, not a conformance result.

**Source commit:** `f4cce4070877f3d91e2a606f8bc93161d1d212e1`.
**Environment:** Linux x86_64, Arch Linux kernel 7.2.14; Rust/Cargo 1.99.1.

## Executed

- `cargo fmt --all` — completed.
- `cargo test --locked --workspace` — 30 tests passed, 0 failed. Nine IPC tests cover valid framing; zero/oversized/truncated header and body rejection; invalid UTF-8/JSON and trailing JSON; duplicate keys and unknown fields; invalid protocol version/kind/method; uint64 sequence overflow; UUID validation; private socket setup/cleanup and Linux peer credentials; deadlines; session replay order; and expired-event rejection before sequence consumption.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.
- `git diff --check` — passed.

These tests cover parser primitives only. They do not execute a full service, same-UID renderer binding, dispatch authorization, restart lifecycle, backpressure, fuzzing, or the complete IPC-01 procedure. No acceptance condition or conformance gate is marked Passed.
