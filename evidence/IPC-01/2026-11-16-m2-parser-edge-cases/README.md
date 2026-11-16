# M2 IPC parser and handshake checks

**Gate:** IPC-01 remains **Not Run**. This records local implementation checks, not a conformance result.

**Source commit:** `1a18237a2bec2b5702d9deaef89561715a4f1b55`.
**Environment:** Linux x86_64, Arch Linux kernel 7.2.14; Rust/Cargo 1.99.1.

## Executed

- `cargo fmt --all` — completed.
- `cargo test --locked --workspace` — 39 tests passed, 0 failed. Eighteen IPC tests cover valid framing; zero/oversized/truncated header and body rejection; invalid UTF-8/JSON and trailing JSON; duplicate keys and unknown fields; invalid protocol version/kind/method; uint64 sequence overflow; UUID validation; private socket setup/cleanup, symlink parent/endpoint rejection, replacement-inode preservation and Linux peer credentials; wrong expected UID/PID rejected before frame parsing; absolute monotonic hello/ready handshake deadline and slow-drip rejection; disconnect during header/body; hello/ready-gated activation and response correlation; replay ordering; expired/wrong-session messages without phase/sequence advancement; framed Unix-stream handshake; and authenticated reconnect with fresh-session/stale-ready rejection.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.
- `git diff --check` — passed.
- [CI run 43826960924](https://github.com/ranjanssgj/lia/actions/runs/43826960924) — device-agent Rust and contract-hygiene jobs passed for the expected-UID/PID accept implementation.

These tests cover parser, session, handshake and single-connection accept primitives only. The accept caller must supply an expected UID/PID; actual renderer supervision/process binding is not implemented. The handshake uses one absolute monotonic timeout across hello, response and ready. The procedure does not cover a long-running accept loop, real separate-UID or same-UID sibling processes, dispatch authorization, reply-side disconnect, renderer restart/snapshot lifecycle, slow-reader writes, message-flood/backpressure stress, fuzzing, foreign-owner or concurrent path-race scenarios, or the complete IPC-01 procedure. No acceptance condition or conformance gate is marked Passed.
