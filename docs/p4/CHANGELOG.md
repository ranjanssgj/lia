## 0.2.9 — 16 November 2026

**Baseline:** Blueprint v1.14.13. The package remains Draft; no implementation conformance gate is marked Passed.

- Requires one matching, non-expired `presentation.ready` request after hello and before user-event acceptance. Repeated, expired, pre-hello, and wrong-session readiness messages do not activate user controls.
- Adds an in-memory hello → ready session phase transition. User-event sequencing remains replay protection only; renderer identity binding, state snapshot delivery, dispatch authorization and the socket service remain open.

## 0.2.8 — 16 November 2026

**Baseline:** Blueprint v1.14.12. The package remains Draft; no implementation conformance gate is marked Passed.

- Defines hello as the required first, non-expired request on a connection; the successful response supplies a fresh connection-scoped session UUID and echoes the request's optional request/correlation IDs.
- Defines one-time hello activation and deadline checking before user-event sequence consumption. Expired hello leaves the connection inactive; expired user events do not advance the replay watermark.
- Adds a Rust session handshake state machine and parser/lifecycle unit checks. Socket-level service, renderer identity binding, dispatch policy, restart/backpressure integration and IPC-01 remain Not Run.

## 0.2.7 — 15 November 2026

**Baseline:** Blueprint v1.14.11. The package remains Draft; no implementation conformance gate is marked Passed.

- Resolves the draft IPC replay-field mismatch: a successful hello issues a random connection-scoped session UUID; user-event requests carry canonical decimal session sequences beginning at zero and increasing strictly.
- Defines disconnect/reconnect to discard the old session and sequence watermark. The sequence is replay control only and does not authenticate the renderer or authorize user events.
- Adds Rust session UUID issuance and user-event sequence validation primitives; service handshake, same-UID renderer binding, dispatch policy, restart/backpressure integration and IPC-01 remain Not Run.

## 0.2.6 — 15 November 2026

**Baseline:** Blueprint v1.14.10. The package remains Draft; no implementation conformance gate is marked Passed.

- Records the owner-authorized exact P5 M3 Hyprland Wayland baseline as stable v0.57.2 ([official release](https://github.com/hyprwm/Hyprland/releases/tag/v0.57.2), signed release commit `11cbc6c577b84ffb1edf4df3fc5b6128c3ae6947`).
- Notes the Hyprland 0.57 configuration migration: `.conf` files are unsupported, so the test profile must use Lua configuration. HYPR-01/02 remain Not Run; no compositor capability is claimed supported.

## 0.2.5 — 15 November 2026

**Baseline:** Blueprint v1.14.8. The package remains Draft; no implementation conformance gate is marked Passed.

- Adds SQLite migration 008 to rebuild legacy lease/session tables without floating-point `julianday()` CHECK constraints; Rust startup backfills legacy timestamps to exact integer microseconds before applying it.
- Adds a development filesystem sidecar integrity/time anchor. It advances before protected commits, rejects SQLite-only stale restores and clock rollback, and fails closed on mismatch. Coordinated database-plus-anchor restore and production protected monotonic storage remain open.
- Extends local evidence through migrations 001–008 and retains SQLITE-02 as Not Run pending extended workload, recovery and integration procedures.

# P4 contract package changelog

## 0.2.4 — 15 November 2026

**Baseline:** Blueprint v1.14.7. The package remains Draft; no implementation conformance gate is marked Passed.

- Adds SQLite migration 007 with integer microsecond columns and integer-valued nonce/session retention guards. Runtime command/lease admission now parses RFC 3339 to integer microseconds; complete legacy-row backfill and removal of older floating-point CHECK constraints remain open.
- Records device-store outcome/evidence/outbox writes and explicit `synchronous=FULL`; migrations acquire `BEGIN IMMEDIATE` before checking their version.
- Adds signed-message canonicalization profile and local RFC 8785 number/non-BMP key-order and negative signature checks. Generated-language interoperability remains Not Run.
- Adds local process-kill windows, two-reader/writer contention, SQLITE_BUSY, checkpoint and WAL-truncation tests. No conformance gate is marked Passed; stale-restore anchor integration, extended workload measurements and full SECURITY-01 procedures remain open.

## 0.2.3 — 15 November 2026

**Baseline:** Blueprint v1.14.7. The package remains Draft; no implementation conformance gate is marked Passed.

- Adds SQLite migration 006 for immutable outcome evidence metadata and database guards requiring evidence for verified success/failure, plus a durable possible-effect marker for unknown outcomes.
- Defines the store boundary: not attempted is an accepted operation with no attempt; verified success/failure require matching evidence; any ambiguous possible effect is unknown and reconciled rather than replayed.
- Clarifies that outcomes, attempt state, evidence metadata and the corresponding local outbox event commit atomically.

## 0.2.2 — 15 November 2026

**Baseline:** Blueprint v1.14.5. The package remains Draft; no implementation conformance gate is marked Passed.

- Adds SQLite migration 005. A new session's first CommandEnvelope sequence is zero exactly once; subsequent accepted values strictly increase. Retention and immutable session identity are preserved.
- Updates the SQLITE-02 procedure to test zero-first, duplicate-zero, backward/equal updates and expiry-plus-skew retention.
- This is a forward migration; migrations 001–004 are unchanged.

## 0.2.1 — 14 November 2026

**Baseline:** Blueprint v1.14.2. The branch incorporates Claude's supplied `lia-fix-p4.bundle` commit and `p4-contract-consistency.patch`, after independent review and additional corrections. The patch and bundle contain the same seven-file commit (`df9a3e6`).

Reconciles cross-contract fields and closes draft persistence gaps identified during independent review. This revision remains Draft; no conformance gate is marked Passed.

- Normalizes manifest revision as a monotonic unsigned Protobuf field, bounded to the JSON/JCS-safe integer range `1..9007199254740991` for JSON and SQL surfaces.
- Aligns `AcceptedCommand.statusUrl` with `GET /v1/requests/{requestId}` and an explicit rebuildable request-status projection. Defines HTTP 429 with a delay-seconds `Retry-After` header.
- Renames operation-evidence response identifiers to `*_attempt_ids`; adds event `source_event_id` and `sensitivity` fields.
- Adds PostgreSQL account-consistent references and an issuance ledger whose claims must match the operation's target, resource generation/revision/observation time/owner, arguments digest, purpose, grant, policy/revocation version and risk tier. Adds device-side SQLite consume-before-effect and nonce/session replay ledgers; operation claims cannot be changed after lease issuance, and consumption requires both a recorded nonce and a durable possible-effect marker.
- Adds PostgreSQL migration 004 for rebuildable objective, run, grant-metadata and request-status projections plus bounded API idempotency records, with JSON-safe integer constraints. SQLite migration 004 enforces the interoperable manifest-revision bound. Canonical ownership remains with Temporal and Permission Authority.
- Documents cross-contract mappings and distinguishes P4 gates from MOBILE-01/HOME-01, which remain P6 work.

- Separates offline-lease nonce consumption from CommandEnvelope session-sequence replay protection; the lease has no session-sequence field. The first command sequence in a new session is zero exactly once; subsequent values strictly increase. The watermark is retained through session expiry plus configured clock skew.

### Review verification

Independent checks executed in this review: SQLite migrations 001–004 on SQLite 3.46.1; exact lease-to-operation binding tests (target, resource revision, observation timestamp, owning device and argument digest), rejection of R2 leases, nonce replay rejection, consume-before-effect ordering and operation-claim immutability; JSON Schema Draft 2020-12 meta-validation and all four fixture validations; YAML parsing plus structural assertions for OpenAPI 3.1, the request-status route and HTTP 429 on all 11 operations; strict TypeScript checking of the provider interface file; relative-link checks for the P4 README and blueprint. These checks are not full conformance gates. After the final replay-hardening edit, the additional SQLite nonce immutability, pre-deadline deletion guard and monotonic session-sequence triggers were separately exercised against SQLite 3.46.1 in an isolated trigger fixture; the full updated migration was not rerun after that last trigger-only edit.

Claude's supplied validation note reports that its sandbox compiled the Protobuf file, validated OpenAPI 3.1, applied PostgreSQL migrations to a real PostgreSQL 16 server, applied SQLite migrations, validated schemas/fixtures and ran strict TypeScript checking. These are reported by Claude, not independently reproduced in this review. The current review environment did not have PostgreSQL or `protoc`; it therefore did not independently execute PostgreSQL migrations, Protobuf code generation/compilation, a full OpenAPI 3.1 validator, generated-client interoperability, security, Temporal, NATS, live adapters or end-to-end conformance. All conformance gates remain Not Run.
