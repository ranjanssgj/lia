# M1 local SQLite implementation checks (partial evidence)

**Gate:** SQLITE-02, full gate status remains **Not Run**.
**Code revision:** local branch `feat/p5-m1-device-agent-core`; capture the final commit SHA after commit.
**Procedure:** M1 core checks against P4 draft package 0.2.2 and SQLite migrations 001–005.
**Environment:** Linux x86_64; Rust/Cargo 1.99.1; rusqlite 0.41.1 with bundled SQLite 3.54.0. The contract-CI SQLite WASM 3.54.1 runtime is separate.

The Rust unit suite applies the complete migration chain to both in-memory and file-backed databases, verifies WAL and foreign keys, validates the first-zero-once/strictly-increasing sequence behavior and retention triggers, checks operation/lease exact-claim rejection, nonce replay rejection, and confirms that the returned effect permit follows a committed possible-effect marker. The fixture tests cover one RFC 8032 Ed25519 primitive vector, command/lease domain separation, a simple RFC 8785 property-ordering vector, and SHA-256.

Commands and raw output are retained alongside this README. These checks do not cover process-kill crash windows, backup/restore or anti-resurrection, cross-runtime interoperability, full Protobuf ProtoJSON mapping, OIDC/mTLS, production key handling, live adapters, or all SQLITE-02/SECURITY-01 procedures. No conformance gate or acceptance condition is marked Passed.
