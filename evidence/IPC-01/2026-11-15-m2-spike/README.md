# M2 IPC primitive checks (partial evidence)

**Gate:** IPC-01 remains **Not Run**. This is local engineering evidence, not a conformance result.

**Environment:** Linux x86_64; Ubuntu 24.04 is the pinned CI development baseline; Rust/Cargo 1.99.1; libc 0.2.191. The CI runner does not select a supported desktop distribution or runtime profile.

## Executed

**Source commit:** `0420fddbbd916f3842cd9de59c63333b92aa9063` (implementation tree used for this run).

- `cargo test --locked --workspace` — 25 tests passed, 0 failed. Five IPC unit tests cover valid frame round-trip, allowlist enforcement, zero/oversized/truncated frames, invalid UTF-8/JSON, duplicate keys, unknown envelope fields, private directory/socket modes, refusal to replace an existing path, inode-tied cleanup, kernel `SO_PEERCRED` UID/PID reads, expected-UID rejection, and deadline expiry validation.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.
- `cargo fmt --all -- --check` — passed.

The spike uses the P4 draft's 4-byte big-endian frame and 262,144-byte cap. Frame parsing rejects unknown envelope keys, duplicate keys and unallowlisted method names. `PrivateSocket` binds only beneath an already 0700 owner-controlled directory and uses 0600 for the socket. Peer credentials come from Linux `SO_PEERCRED`, not JSON claims.

## Not executed / known limits

- No complete server/client presentation bridge or request/session authorization implementation.
- No separate-UID integration test, foreign-owner/symlink race test, same-UID renderer identity binding, session-generation/restart replay test, queue/backpressure stress test, fuzz run, or IPC latency/resource measurement.
- `Envelope::validate_deadline` is a primitive; dispatch code must call it with the trusted clock before handling a request.
- No Windows/macOS behavior is implemented or claimed.

No acceptance condition or conformance gate is marked Passed.
