# M1 command expiry and replay-order follow-up

**Gate:** SQLITE-02 remains **Not Run**. This records selected local unit checks, not a conformance result.

**Environment:** Linux x86_64; Rust/Cargo 1.99.1; rusqlite 0.41.1 with bundled SQLite 3.54.0; migration runner SQLite WASM 3.54.1.
**Source commit:** `09bfea8bd0600eda8121eef3d06dff046536c4b4`.

## Executed

- `cargo fmt --all -- --check` — passed.
- `cargo test --locked --workspace` — 26 passed, 0 failed. Includes the M1 test for expired-command rejection, first sequence zero, forward sequence advancement, out-of-order sequence rejection, and rollback of rejected command state, plus existing consume-before-effect and nonce-replay tests.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.
- `npm run check:sqlite` — migrations 001–008 applied with SQLite WASM 3.54.1; duplicate/backward sequence changes and premature watermark deletion were rejected.

The same workspace run includes five M2 IPC primitive tests; those are partial IPC evidence and do not contribute to SQLITE-02 completion. Simultaneous store startup exposed a `SQLITE_BUSY` from concurrent WAL-mode setup; startup now retries busy/locked results for up to five seconds before failing. The concurrent-first-open test passes locally.

## Limits

This does not repeat the full SQLITE-02 crash/restore, contention, long-reader, WAL-growth, production trusted-time, coordinated-restore or independent review procedures. No gate or acceptance condition is marked Passed.
