# M2 IPC parser and handshake checks

**Gate:** IPC-01 remains **Not Run**. This records local implementation checks, not a conformance result.

**Source commit:** `f7f7bd77faa2dd61a35cf4ea0537d2acb9258409`.
**Environment:** Linux x86_64, Arch Linux kernel 7.2.14; Rust/Cargo 1.99.1.

## Executed

- `cargo fmt --all` — completed.
- `cargo test --locked --workspace` — 35 tests passed, 0 failed. Fourteen IPC tests cover valid framing; zero/oversized/truncated header and body rejection; invalid UTF-8/JSON and trailing JSON; duplicate keys and unknown fields; invalid protocol version/kind/method; uint64 sequence overflow; UUID validation; private socket setup/cleanup, symlink parent/endpoint rejection, replacement-inode preservation and Linux peer credentials; deadlines; hello/ready-gated connection activation and response correlation; replay ordering; expired/wrong-session readiness or user events without phase/sequence advancement; framed Unix-stream handshake; and authenticated listener reconnect with fresh-session/stale-ready rejection.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.
- `git diff --check` — passed.

These tests cover parser, session, handshake and single-connection accept primitives only. They do not execute a long-running accept loop, same-UID renderer binding, dispatch authorization, renderer process restart/snapshot lifecycle, backpressure stress, fuzzing, foreign-owner or concurrent path-race scenarios, or the complete IPC-01 procedure. No acceptance condition or conformance gate is marked Passed.
