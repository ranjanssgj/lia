# M1 local SQLite implementation checks (partial evidence)

**Gate:** SQLITE-02, full gate status remains **Not Run**.
**Code snapshot:** tree `fef6df0476b5aaea754ae66b6792ad8bdbc7e0b8`, created by PR code commit `8cab9002b5eecb683cdfc138cc0201eacd1241d0`. `inputs.sha256` records the tested source hashes.
**Procedure:** M1 core checks against P4 draft package 0.2.2 and SQLite migrations 001–005.
**Environment:** Linux x86_64; Rust/Cargo 1.99.1; rusqlite 0.41.1 with bundled SQLite 3.54.0. The contract-CI SQLite WASM 3.54.1 runtime is separate.

The Rust unit suite applies the complete migration chain to both in-memory and file-backed databases, verifies WAL and foreign keys, validates the first-zero-once/strictly-increasing sequence behavior and retention triggers, checks operation/lease exact-claim rejection, cross-session nonce replay rejection, transaction rollback, and confirms that the returned effect permit follows a committed possible-effect marker. A backup/restore test preserves an in-flight effect marker and nonce, converts the operation/attempt to `reconciliation_required`, then rejects replay. The fixture tests cover one RFC 8032 Ed25519 primitive vector, command/lease domain separation, a simple RFC 8785 property-ordering vector, and SHA-256.

Commands and raw output are retained alongside this README. These checks do not cover process-kill crash windows, rollback to a stale backup/anti-resurrection, cross-runtime interoperability, full Protobuf ProtoJSON mapping, OIDC/mTLS, production key handling, live adapters, or all SQLITE-02/SECURITY-01 procedures. No conformance gate or acceptance condition is marked Passed.
