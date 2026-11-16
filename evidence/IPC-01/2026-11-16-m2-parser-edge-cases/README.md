# M2 IPC parser and handshake checks

**Gate:** IPC-01 remains **Not Run**. This records local implementation checks, not a conformance result.

**Source commit:** `1b7b246fdce3558c5fcc1ec1fb36c1f7e0681202`.
**Environment:** Linux x86_64, Arch Linux kernel 7.2.14; Rust/Cargo 1.99.1.

## Executed

- `cargo fmt --all` — completed.
- `cargo test --locked --workspace` — 42 tests passed, 0 failed. Twenty-one IPC tests cover valid framing; zero/oversized/truncated header and body rejection; invalid UTF-8/JSON and trailing JSON; duplicate keys and unknown fields; invalid protocol version/kind/method; uint64 sequence overflow; UUID validation; private socket setup/cleanup, symlink parent/endpoint rejection, replacement-inode preservation and Linux peer credentials; wrong expected UID/PID rejected before frame parsing, including a same-UID sibling process; absolute monotonic hello/ready handshake deadline and slow-drip rejection; disconnect during header/body; hello/ready-gated activation and response correlation; replay ordering; expired/wrong-session messages without phase/sequence advancement; typed user-event bodies including fabricated outcome/unknown-field rejection; framed Unix-stream handshake; authenticated reconnect with fresh-session/stale-ready rejection; and wire-level acceptance of one event followed by replay rejection and connection closure.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.
- `git diff --check` — passed.
- [CI run 43855895639](https://github.com/ranjanssgj/lia/actions/runs/43855895639) — device-agent Rust and contract-hygiene jobs passed for the typed user-event parser and early-close test correction.
- [CI run 43854174538](https://github.com/ranjanssgj/lia/actions/runs/43854174538) — device-agent Rust and contract-hygiene jobs passed for the framed user-event receive implementation.

These tests cover parser, session, handshake, one-session receive and single-connection accept primitives only. The accept caller must supply an expected UID/PID; actual renderer supervision/process binding is not implemented. The handshake and event receive each use an absolute monotonic timeout. Returned event envelopes remain untrusted and are not dispatched. The procedure does not cover a long-running accept loop, real separate-UID peers, renderer-supervisor integration, dispatch authorization, reply-side disconnect, renderer restart/snapshot lifecycle, slow-reader writes, message-flood/backpressure stress, fuzzing, foreign-owner or concurrent path-race scenarios, or the complete IPC-01 procedure. No acceptance condition or conformance gate is marked Passed.

## Initial follow-up on 16 November 2026 (before accept-loop/supervisor increment)

The IPC primitive now has two additional local checks: a ready-session snapshot send verifies `presentation.state.replace`, event direction and session binding; a bounded synchronous user-event queue reports `WouldBlock` and closes the connection when full without enqueueing or dispatching the new event. `cargo test --locked --workspace` passed 44 tests on Rust 1.99.1, including these checks. The implementation and review commit are recorded by the enclosing PR branch.

This remains partial evidence. The actual renderer supervisor/PID binding, long-running multi-connection service, reply-side disconnect test, separate-UID and foreign-owner/path-race coverage, fuzzing, service-level resource high-water measurements and the complete IPC-01 procedure are still open; a local framing baseline is retained separately. No user-event dispatch handler exists; the queue carries untrusted intents only.


A further local follow-up adds the multi-connection accept loop, monotonic one-slot snapshot coalescing, and a renderer supervisor that binds peer credentials to a launched child and refreshes the expected PID after restart with bounded attempts and backoff. Snapshot send after a peer disconnect is also covered. The full Rust workspace now has 47 passing tests (26 IPC tests). The supervisor test launches the test harness as a synthetic renderer; Godot integration and full renderer reply-flow integration remain open.
