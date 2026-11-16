# M2 IPC parser and handshake checks

**Gate:** IPC-01 remains **Not Run**. This records local implementation checks, not a conformance result.

**Source commit:** `6109276189873a74480ed2130459e691078f0a6f`.
**Environment:** Linux x86_64, Arch Linux kernel 7.2.14; Rust/Cargo 1.99.1.

## Executed

- `cargo fmt --all` — completed.
- `cargo test --locked --workspace` — 40 tests passed, 0 failed. Nineteen IPC tests cover valid framing; zero/oversized/truncated header and body rejection; invalid UTF-8/JSON and trailing JSON; duplicate keys and unknown fields; invalid protocol version/kind/method; uint64 sequence overflow; UUID validation; private socket setup/cleanup, symlink parent/endpoint rejection, replacement-inode preservation and Linux peer credentials; wrong expected UID/PID rejected before frame parsing, including a same-UID sibling process; absolute monotonic hello/ready handshake deadline and slow-drip rejection; disconnect during header/body; hello/ready-gated activation and response correlation; replay ordering; expired/wrong-session messages without phase/sequence advancement; framed Unix-stream handshake; and authenticated reconnect with fresh-session/stale-ready rejection.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.
- `git diff --check` — passed.
- [CI run 43827412005](https://github.com/ranjanssgj/lia/actions/runs/43827412005) — device-agent Rust and contract-hygiene jobs passed for the same-UID sibling-process rejection test.

These tests cover parser, session, handshake and single-connection accept primitives only. The accept caller must supply an expected UID/PID; actual renderer supervision/process binding is not implemented. The handshake uses one absolute monotonic timeout across hello, response and ready. The procedure does not cover a long-running accept loop, real separate-UID peers, renderer-supervisor integration, dispatch authorization, reply-side disconnect, renderer restart/snapshot lifecycle, slow-reader writes, message-flood/backpressure stress, fuzzing, foreign-owner or concurrent path-race scenarios, or the complete IPC-01 procedure. No acceptance condition or conformance gate is marked Passed.
