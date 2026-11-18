# Reachable separate-UID peer credential check

**Gate:** IPC-01 remains **Not Run**. This is a focused Linux peer-auth test, not a conformance result.

**Procedure:** Run the ignored mapped-user-namespace case under `unshare --map-auto -Ur`. A child mapped to UID 65534 connects to a test-only reachable socket; the test accepts the connection at the kernel boundary and confirms `SO_PEERCRED` rejects it before any frame is parsed. It separately confirms a foreign-owned runtime directory is refused. The temporary test relaxes the disposable socket directory and socket mode only to make that peer reachable; production endpoint policy remains directory 0700 and socket 0600.

**Environment:** Arch Linux x86_64; Rust/Cargo 1.99.1. `raw.txt` contains the exact command, test results and workspace checks.

## Results

- Focused mapped-user-namespace peer test — passed.
- `cargo fmt --all -- --check` — passed.
- `cargo test --locked --workspace` — 51 passed, 0 failed, 1 namespace-only test ignored in the ordinary run.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.

## Limits

This validates `SO_PEERCRED` using the Rust test listener and a deliberately reachable test socket. It does not prove the production Godot process binding, authorization policy, service restart behavior, sustained resource bounds or complete IPC-01. No conformance gate or acceptance condition is marked Passed.
