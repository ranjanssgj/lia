# M2 IPC parser and handshake checks

**Gate:** IPC-01 remains **Not Run**. This records local implementation checks, not a conformance result.

**Source commit:** `0aa6ec7315cc93e085f55660d750cc746c9d1c0f`.
**Environment:** Linux x86_64, Arch Linux kernel 7.2.14; Rust/Cargo 1.99.1.

## Executed

- `cargo fmt --all` — completed.
- `cargo test --locked --workspace` — 32 tests passed, 0 failed. Eleven IPC tests cover valid framing; zero/oversized/truncated header and body rejection; invalid UTF-8/JSON and trailing JSON; duplicate keys and unknown fields; invalid protocol version/kind/method; uint64 sequence overflow; UUID validation; private socket setup/cleanup and Linux peer credentials; deadlines; hello/ready-gated connection activation and response correlation; replay ordering; expired/wrong-session readiness or user events without phase/sequence advancement; and a framed Unix-stream hello/ready exchange.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.
- `git diff --check` — passed.

These tests cover parser, in-memory session and handshake primitives only. They do not execute a listening service, same-UID renderer binding, dispatch authorization, restart/snapshot lifecycle, backpressure stress, fuzzing, or the complete IPC-01 procedure. No acceptance condition or conformance gate is marked Passed.
