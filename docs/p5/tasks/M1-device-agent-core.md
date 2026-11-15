# M1 task plan — device-agent core

**Status:** In progress; scope recorded before implementation on 15 November 2026.
**Acceptance conditions:** AC-005, AC-006, AC-008, AC-009, AC-011 foundations.
**Gates:** SQLITE-02; local SECURITY-01 vector subset.
**P4 baseline:** Draft package 0.2.5 on the P5 branch; P4 implementation gates remain Not Run.

## Contract inputs

- `docs/p4/sql/sqlite/001-device-agent.sql` — operation/attempt journal and local metadata.
- `docs/p4/sql/sqlite/002-retention-tombstones.sql` — local tombstone/cursor markers.
- `docs/p4/sql/sqlite/003-consumption-ledger.sql` — exact lease-to-operation binding, distinct command/lease replay paths, consume-before-effect and retained watermarks.
- `docs/p4/sql/sqlite/004-manifest-revision-json-safe.sql` — JSON-safe manifest revision checks.
- `docs/p4/sql/sqlite/005-sequence-zero-initial-use.sql` — first command sequence zero exactly once, then strict increase.
- `docs/p4/sql/sqlite/006-operation-outcomes-and-evidence.sql` — verified/unknown outcome evidence metadata and integrity guards.
- `docs/p4/sql/sqlite/007-integer-time-and-integrity-anchor.sql` — integer time columns and retention triggers.
- `docs/p4/sql/sqlite/008-integer-time-table-rebuild.sql` — rebuilds legacy lease/session time constraints without floating-point `julianday()`.
- `docs/p4/security/security-profile-v1.md` §4–6 — Ed25519/JCS domains, replay, local peer boundary.
- `docs/p4/proto/lia/v1/control_plane.proto` — `CommandEnvelope`, `OfflineAuthorizationLease`, `ResourceReference` and `RiskTier` fields.

## Scope and first implementation

Create a small Rust library under `crates/lia-device-agent/` that bootstraps the existing SQLite migrations, enables foreign keys and WAL, records operations/attempts, and makes a durable consume-before-effect decision before returning an effect permit. Command replay uses its session sequence and nonce. Offline leases use their own nonce and exact claim-bound lease row; they do not use a session sequence. Both paths commit the possible-effect marker before any caller may invoke an adapter.

Verify the public RFC 8032 Ed25519 primitive vector and the checked-in JCS/SHA-256 fixture. Implement the P4 signature domain prefixes and strict Ed25519 verification as primitives. This first task does not claim complete ProtoJSON/Protobuf interoperability, OIDC, mTLS, authorization freshness or an OS adapter; local SECURITY-01 remains a subset and all gates remain Not Run pending full evidence/review.

## Sequence-zero clarification

The owner requires the first `CommandEnvelope.session_sequence` in a new session to be zero. Migration 003 initializes the watermark to zero but also requires every accepted value to be greater than that watermark, which rejects a first value of zero. Do not edit migration 003. Forward migration 005 adds an explicit `has_accepted_sequence` marker: the first acceptance may be exactly zero; all later values must strictly increase. The watermark and its retention deadline remain immutable except for monotonic sequence advancement, and deletion remains blocked until session expiry plus configured clock skew.

## Pins and license review

The pinned Rust compiler is 1.99.1. Exact direct crate pins for this slice are exact versions: `rusqlite 0.41.1` with bundled SQLite 3.54.0 and the online-backup feature; `ed25519-dalek 3.0.0`; `serde_jcs 0.2.0`; `sha2 0.11.1`; `serde 1.0.229`; and `serde_json 1.0.154`. `rusqlite 0.42.0` was released 11 November 2026 and is inside the seven-day cool-down, so it is excluded for this pin. The chosen pins are within the owner-approved license triage allowlist; their upstream metadata and generated dependency lock are recorded in `toolchain/LICENSE-REGISTER.md`.

Sources checked 15 November 2026: [rusqlite 0.41.1](https://docs.rs/crate/rusqlite/0.41.1), [ed25519-dalek 3.0.0](https://docs.rs/crate/ed25519-dalek/3.0.0), [serde_jcs 0.2.0](https://docs.rs/crate/serde_jcs/0.2.0), [sha2 0.11.1](https://docs.rs/crate/sha2/0.11.1), [serde 1.0.229](https://docs.rs/crate/serde/1.0.229), and [serde_json 1.0.154](https://docs.rs/crate/serde_json/1.0.154). The project license remains deferred; this task does not add a `LICENSE` file or grant redistribution rights.

## Expected artifacts

- Workspace manifest, exact direct dependencies and `Cargo.lock`.
- Forward SQLite migrations 005–008 and a recorded migration chain through 008, including exact legacy timestamp backfill.
- Store/bootstrap/claim/replay code plus tests for migrations, WAL/foreign keys, claim mismatches, nonce replay, zero-first/monotonic sequence behavior, retention, consume-before-effect atomicity, and backup/restore reconciliation.
- RFC 8032 and JCS/SHA-256 fixture checks.
- Redacted run evidence under `evidence/SQLITE-02/<run-id>/` and `evidence/SECURITY-01-local/<run-id>/`; statuses stay Not Run until procedures and review are complete.

## Stale restore and trusted-time decision

**Decision:** use an external monotonic integrity/time anchor. A production implementation must advance it before each protected SQLite commit and fail closed on startup mismatch; the external anchor also keeps a UTC-microsecond high-water mark and rejects clock rollback. Restoring a stale SQLite backup must require coordinator reconciliation. Restoring both database and anchor together remains outside this protection and requires a separate recovery control.

**Implementation status:** migration 007 creates database-side integer high-water state and retention guards. Migration 008 rebuilds the affected tables after Rust startup backfills exact integer microseconds from legacy RFC 3339 values. The crate prototype uses a mode-restricted filesystem sidecar to anchor protected SQLite commits and UTC-microsecond high-water time; it rejects SQLite-only stale restore and clock rollback. Restoring both files together remains undetected, and the filesystem sidecar is not a production protected monotonic store. These remain SQLITE-02 limitations; no gate is Passed.

The M2 threat-to-test map is drafted at `docs/p5/tasks/M2-threat-model-test-map.md`; no IPC implementation or IPC-01 evidence exists yet.
