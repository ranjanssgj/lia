# M1 follow-up checks (partial evidence)

**Gate:** SQLITE-02 remains **Not Run**. This is local engineering evidence, not a conformance result.

**Environment:** Linux x86_64; Rust/Cargo 1.99.1; rusqlite 0.41.1 with bundled SQLite 3.54.0; migration script SQLite WASM 3.54.1.

## Executed

- `CARGO_HOME=/tmp/lia-cargo cargo test --locked --workspace` — 17 tests passed, 0 failed. Includes outcomes/evidence/outbox atomicity, verified and unknown outcome states, integer RFC3339 parsing, migration 001–007 startup, `synchronous=FULL`, immediate migration serialization, command/lease replay checks, process termination after consume-before-effect commit and during an uncommitted transaction, two pinned WAL readers, a contending writer with observed `SQLITE_BUSY`, checkpoint progress and WAL truncation.
- `CARGO_HOME=/tmp/lia-cargo cargo clippy --locked --workspace --all-targets -- -D warnings` — passed.
- `node scripts/check-sqlite-migrations.mjs` — applied migrations 001–007 under SQLite 3.54.1; zero-first, duplicate/backward sequence and premature deletion checks behaved as expected.
- Local SECURITY-01 tests cover RFC 8032, JCS number formatting, non-BMP UTF-16 key ordering, wrong payload/domain/key, truncated signature and noncanonical Ed25519 S rejection. See `docs/p4/security/signed-message-canonicalization-v1.md` for the draft field mapping.

## Not executed / known limits

- No measured production-scale contention/load profile or prolonged WAL growth/long-lived-reader run; the unit test exercises bounded local WAL mechanics only.
- No stale-backup restore anti-rollback proof. The external monotonic anchor was chosen but is not implemented; restoring SQLite alone is not detected by the crate yet.
- Migration 007 adds integer columns and integer retention guards, but legacy timestamp data is not fully backfilled and legacy floating-point `julianday()` CHECK constraints in migration 003 remain. Do not treat the migration as complete clock hardening.
- No generated Rust/TypeScript ProtoJSON interoperability, OIDC/mTLS, device key lifecycle, Temporal/NATS, live adapter, or full IPC/security conformance tests.

No acceptance condition or conformance gate is marked Passed.
