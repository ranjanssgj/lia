# M2 IPC parser and handshake checks

**Gate:** IPC-01 remains **Not Run**. This records local implementation checks, not a conformance result.

**Source commit:** `5e05fee5b2db070b762f732becc5f3843a1d2bc8`.
**Environment:** Linux x86_64, Arch Linux kernel 7.2.14; Rust/Cargo 1.99.1.

## Executed

- `cargo fmt --all` — completed.
- `cargo test --locked --workspace` — 33 tests passed, 0 failed. Twelve IPC tests cover valid framing; zero/oversized/truncated header and body rejection; invalid UTF-8/JSON and trailing JSON; duplicate keys and unknown fields; invalid protocol version/kind/method; uint64 sequence overflow; UUID validation; private socket setup/cleanup and Linux peer credentials; deadlines; hello/ready-gated connection activation and response correlation; replay ordering; expired/wrong-session readiness or user events without phase/sequence advancement; framed Unix-stream handshake; and authenticated listener reconnect with fresh-session/stale-ready rejection.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.
- `git diff --check` — passed.

These tests cover parser, session, handshake and single-connection accept primitives only. They do not execute a long-running accept loop, same-UID renderer binding, dispatch authorization, renderer process restart/snapshot lifecycle, backpressure stress, fuzzing, or the complete IPC-01 procedure. No acceptance condition or conformance gate is marked Passed.
