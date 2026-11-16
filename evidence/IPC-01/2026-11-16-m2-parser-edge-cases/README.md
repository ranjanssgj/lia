# M2 IPC parser and handshake checks

**Gate:** IPC-01 remains **Not Run**. This records local implementation checks, not a conformance result.

**Source commit:** `f5aebd212e2fe8ccc26696362ca920481d488edb`.
**Environment:** Linux x86_64, Arch Linux kernel 7.2.14; Rust/Cargo 1.99.1.

## Executed

- `cargo fmt --all` — completed.
- `cargo test --locked --workspace` — 31 tests passed, 0 failed. Eleven IPC tests cover valid framing; zero/oversized/truncated header and body rejection; invalid UTF-8/JSON and trailing JSON; duplicate keys and unknown fields; invalid protocol version/kind/method; uint64 sequence overflow; UUID validation; private socket setup/cleanup and Linux peer credentials; deadlines; hello-gated connection activation and response correlation; replay ordering; and expired-hello/user-event rejection without activation or sequence consumption.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.
- `git diff --check` — passed.

These tests cover parser and in-memory session primitives only. They do not execute a full socket service, same-UID renderer binding, dispatch authorization, socket-level restart/snapshot lifecycle, backpressure, fuzzing, or the complete IPC-01 procedure. No acceptance condition or conformance gate is marked Passed.
