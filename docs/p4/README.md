# P4 — Integration Contracts

**Contract package:** P4.1  
**Revision:** 0.3.2
**Status:** Draft — review and conformance evidence required  
**Baseline:** LIA Project Blueprint v1.14.28 (P4 in progress)
**Date:** 17 November 2026

## Purpose

P4 turns the approved P2 requirements and P3 architecture into concrete, versioned interfaces. This package defines reviewable contracts for device/control-plane RPC, authenticated user APIs, the Godot presentation bridge, PostgreSQL and SQLite persistence, Temporal workflows and retention jobs, NATS events, governed context, tool mediation, model-provider adapters, adapter manifests, runtime packaging and conformance testing.

This remains a contract design package. P5 M1 now consumes the draft SQLite migrations in a local Rust library and has partial apply/replay checks; this is not a claim that P4 migrations, generated clients, service integrations, or security are fully implemented or that SQLITE-02 has passed.

## Governing invariants

1. **Authority is not transport.** gRPC, NATS, Temporal, local IPC, and provider SDKs carry requests or facts; none independently grants permission.
2. **Reasoning is not authority.** Model output remains an untrusted proposal. The deterministic broker, current permission authority, workflow authority, and resource-owning executor validate and authorize effects.
3. **The resource owner executes and verifies.** A desktop action is executed by the local desktop agent; an external write by its connector; a physical action by the authorized gateway/device adapter.
4. **Delivery is not completion.** RPC acceptance, NATS acknowledgement, notification delivery, and provider acceptance are not verified operation success.
5. **At-least-once is assumed.** Stable IDs, durable inbox/outbox records, idempotent handlers, and reconciliation are required. Exactly-once side effects are not claimed.
6. **Current authority is rechecked.** Every side effect validates target, resource generation, capability version, purpose, grant, policy version, risk, expiry, and stop/revocation state.
7. **Data remains domain-owned.** Workflow, identity, permissions, memory, device state, and provider state retain their P2/P3 owners. Caches and projections are non-authoritative.
8. **Privacy is enforced at the boundary.** Raw screen/audio/camera streams and secrets do not enter generic event payloads, Temporal histories, provider prompts, or ordinary sync.
9. **Platform capability is explicit.** A missing or unsupported OS capability is represented as unavailable/restricted; no parity is inferred.
10. **P4 artifacts remain Draft until reviewed.** A schema file or passing linter is not an integration proof.

## Contract inventory

| Contract | Artifact | Version | Authority |
|---|---|---:|---|
| Device/control-plane RPC | [control_plane.proto](proto/lia/v1/control_plane.proto) | lia.v1 | Sessions, capabilities, signed commands, single-use offline leases and execution evidence |
| User/coordinator HTTP API | [lia-v1.yaml](openapi/lia-v1.yaml) | /v1 | Authenticated API plus domain authorization |
| Presentation IPC | [presentation-agent-v1.md](ipc/presentation-agent-v1.md) | 1 | Rust local agent; Godot is presentation-only |
| Security profile | [security-profile-v1.md](security/security-profile-v1.md) | 1 | OIDC, device mTLS, signed commands and local peer identity |
| PostgreSQL schema | [001-control-plane.sql](sql/postgres/001-control-plane.sql) | 001 | Device/control projection, operation journal, event delivery |
| SQLite schema | [001-device-agent.sql](sql/sqlite/001-device-agent.sql) | 001 | One device installation's local journal/cache |
| Event envelope and delivery | [event-contract-v1.md](events/event-contract-v1.md) | 1 | Producing domain and durable consumer |
| Event subjects | [subjects.yaml](events/subjects.yaml) | lia.v1 | Domain event owner |
| Temporal boundaries | [temporal-contracts-v1.md](workflows/temporal-contracts-v1.md) | 1 | Temporal workflow authority |
| Provider interfaces | [provider-interfaces-v1.ts](providers/provider-interfaces-v1.ts) | 1 | LIA model gateway |
| Changelog | [CHANGELOG.md](CHANGELOG.md) | 0.3.2 | Review history and verification provenance for the P4 contract package |
| P4 gate matrix | [p4-conformance-matrix.md](conformance/p4-conformance-matrix.md) | 0.3.2 | Procedure, expected evidence and status for architecture proof gates |
| Adapter contract | [adapter-conformance-v1.md](adapters/adapter-conformance-v1.md) | 1 | Manifest semantics, resource lifecycle, execution checks and 24 common adapter conformance cases |
| Manifest schema | [capability-manifest-v1.schema.json](adapters/capability-manifest-v1.schema.json) | 1 | Strict JSON Schema for adapter capability manifests |
| Hyprland fixture | [hyprland-wayland-manifest.example.json](adapters/fixtures/hyprland-wayland-manifest.example.json) | Example | Synthetic first-Linux-profile manifest fixture |
| Compatibility matrix | [compatibility-matrix.md](compatibility-matrix.md) | 0.1.2 | Approved stack baselines, exact-pin status and proof-gate mapping |
| PostgreSQL retention jobs | [002-retention-deletion.sql](sql/postgres/002-retention-deletion.sql) | 002 | Orchestration metadata only; canonical tombstones remain owner-domain data |
| SQLite tombstones | [002-retention-tombstones.sql](sql/sqlite/002-retention-tombstones.sql) | 002 | Local replay-prevention markers; not the canonical deletion authority |
| PostgreSQL integrity and lease ledger | [003-integrity-and-lease-ledger.sql](sql/postgres/003-integrity-and-lease-ledger.sql) | 003 | Account-consistent references, exact lease-to-operation claim matching and coordinator issuance ledger |
| PostgreSQL API projections and revision bound | [004-api-projections-and-revision-bound.sql](sql/postgres/004-api-projections-and-revision-bound.sql) | 004 | Rebuildable objective/run/grant/request summaries plus bounded API idempotency records; never canonical workflow or permission state |
| SQLite consumption ledger | [003-consumption-ledger.sql](sql/sqlite/003-consumption-ledger.sql) | 003 | Device-local single-use lease, nonce and session-sequence replay ledgers, matched to the operation claim |
| SQLite revision bound | [004-manifest-revision-json-safe.sql](sql/sqlite/004-manifest-revision-json-safe.sql) | 004 | Enforces the JSON/JCS-safe manifest revision range |
| SQLite sequence-zero correction | [005-sequence-zero-initial-use.sql](sql/sqlite/005-sequence-zero-initial-use.sql) | 005 | Accepts session sequence zero exactly once, then requires strictly increasing values |
| SQLite time-constraint rebuild | [008-integer-time-table-rebuild.sql](sql/sqlite/008-integer-time-table-rebuild.sql) | 008 | Forward rebuild of lease/session tables without floating-point time checks; integer timestamps are retained |
| SQLite outcome/evidence records | [006-operation-outcomes-and-evidence.sql](sql/sqlite/006-operation-outcomes-and-evidence.sql) | 006 | Requires evidence for verified success/failure and defines immutable evidence metadata |
| SQLite integer-time retention | [007-integer-time-and-integrity-anchor.sql](sql/sqlite/007-integer-time-and-integrity-anchor.sql), [008-integer-time-table-rebuild.sql](sql/sqlite/008-integer-time-table-rebuild.sql) | 007–008 | Adds integer microsecond columns, integer retention guards, exact legacy timestamp backfill and rebuilds affected tables to remove floating-point `julianday()` checks; crate prototype uses a filesystem monotonic anchor, with coordinated restore and production protected storage still open |
| Context package | [context-package-v1.md](context/context-package-v1.md), [schema](context/context-package-v1.schema.json), [example](context/fixtures/context-package-v1.example.json) | 1 | Immutable scoped invocation context with trust, digest, expiry, access and egress rules |
| Tool registry | [tool-registry-v1.schema.json](tools/tool-registry-v1.schema.json), [example](tools/tool-registry-v1.example.json) | 1 | Capability-derived tool affordance metadata, not permission |
| Tool validation | [tool-validation-pipeline-v1.md](tools/tool-validation-pipeline-v1.md) | 1 | Registry projection, proposal validation, permission recheck and broker dispatch |
| Synthetic model evaluation | [synthetic-evaluation-harness-v1.md](providers/synthetic-evaluation-harness-v1.md) | 1 | Provider-independent synthetic conformance and adversarial test harness |
| Windows UIA fixture | [windows-uia-manifest.example.json](adapters/fixtures/windows-uia-manifest.example.json) | Example | Synthetic Windows UI Automation capability profile; no support claim |
| Runtime/packaging | [runtime-profile-v1.md](operations/runtime-profile-v1.md) | 1 | Local-first/distributed profiles, config, keystore, secrets, health and telemetry |
| Supply chain/release | [supply-chain-release-v1.md](operations/supply-chain-release-v1.md) | 1 | SPDX SBOM, licenses, asset rights, vulnerability and update/rollback evidence |

## Authentication and signing baseline proposed for P4 review

This package proposes the following protocol-level contract. These choices are not vendor selections and remain subject to SECURITY-01 evidence and review.

- Interactive user authentication uses OpenID Connect with Authorization Code + PKCE. User-facing API calls use an access token whose issuer, audience, validity, client and applicable scopes are validated. The identity-provider vendor and issuer URL remain deployment configuration.
- Device/control-plane gRPC uses TLS 1.3 with a device-specific X.509 client certificate. The certificate identity is mapped to the enrolled installation and credential generation; an authenticated connection is not a standing operation grant.
- Online commands and offline authorization leases use two allowlisted Ed25519/RFC 8785 profiles with distinct message types and domain-separation prefixes (`LIA-COMMAND-V1` and `LIA-OFFLINE-LEASE-V1`). The signing input appends a newline byte then the canonical UTF-8 ProtoJSON payload; signature fields are excluded. The exact key-provisioning/rotation implementation and protected-storage vendor remain open. TLS keys and command-signing keys are distinct.
- Local IPC authenticates the peer using OS process credentials and restrictive endpoint ACLs. It is local-only; no unauthenticated localhost TCP listener is permitted.
- OIDC issuer, certificate authority, key-management/KMS/HSM vendor, hardware-backed-key availability, hosted secret store, and deployment region are not selected here.

The Ed25519 profile must be reconsidered before production if a FIPS or other cryptographic compliance requirement is introduced. The repository currently does not establish such a requirement. The security conformance suite must include valid, invalid, expired, rotated, revoked, replayed, altered, and out-of-order credentials and commands.

## Offline lease consumption and replay barrier

The coordinator-side `offline_lease_issuance` table records issuance, delivery and reported consumption; it is not the device replay barrier. The resource-owning device verifies issuer/audience, device and credential generation, signature, time window, current locally enforceable policy/revocation state, exact capability/resource generation and observation freshness before effect.

Offline leases and online command envelopes use distinct replay controls because the Protobuf OfflineAuthorizationLease has a lease nonce but no session-sequence field.

For a single-use offline lease, the device verifies the signed claims and then uses one SQLite BEGIN IMMEDIATE transaction to (1) insert the exact account/message-domain/nonce/lease reference, (2) mark the operation as possibly having taken effect with a reconciliation-safe status, and (3) advance that exact stored lease to consumed. The transaction commits before the adapter performs the effect. If the process crashes after commit, the operation is reconciled; the lease is never replayed. A consumed lease is not proof that the effect succeeded.

For a signed CommandEnvelope, the device separately inserts the command nonce, requires the first session_sequence in a new session to equal zero and every later value to be strictly greater than the accepted watermark, advances it with a compare-and-swap predicate, and records the possible-effect operation marker in one transaction before the adapter effect. The session watermark is created at zero with `has_accepted_sequence = 0` when an authenticated session opens. Migration 005 distinguishes that unused zero from an already accepted zero. Its immutable retain_until is the session expiry plus the configured maximum accepted clock skew; the row cannot be deleted before that deadline.

The signed lease and operation claims, including their identifiers, are immutable once issued/stored. Consumed nonce records are immutable and cannot be deleted before the signed message expiry plus the configured maximum accepted clock skew. Backup/restore and downgrade procedures must preserve nonce and session-sequence ledgers and revocation state. The clock-skew allowance is a deployment compatibility/security parameter and must be recorded with the device runtime profile.

## Versioning and compatibility rules

- Protobuf package and service version: `lia.v1`. Add fields with new field numbers; never reuse a retired field number or name. Reserve removed fields. Enum zero values mean unspecified. Unknown fields are ignored only when the operation's required semantics remain understood.
- HTTP API: path version `/v1`. Additive optional response fields are compatible; changing meaning, requiredness, authorization or side-effect semantics requires a new version or an explicit compatibility plan.
- IPC: `protocol_version = "1"`. Unknown methods or major versions fail closed. IPC is not a general RPC tunnel into the OS agent.
- Events: `schema_version` is explicit and event type selects a versioned payload schema. Consumers quarantine unknown required semantics.
- Database: monotonic, immutable migration IDs. Use expand/migrate/contract for deployed schema changes. Rollback must not resurrect revoked grants, credentials or deleted data.
- Temporal: workflow type, input/output schema, task queue and workflow-code version are recorded. Incompatible workflow changes require replay/versioning tests and a compatible worker rollout.
- Provider adapters: normalized LIA types are canonical. Provider-specific payloads and IDs stay namespaced and are never persisted as the domain model.
- Generated clients are produced from checked-in schemas in CI; generated code must not be edited by hand.

## Data ownership boundaries

| Domain | Canonical owner | P4 representation |
|---|---|---|
| Account identity and login | Identity authority | OIDC principal and verified token claims |
| Device identity, trust, capability revision | Device identity registry | PostgreSQL device registry and gRPC session |
| Grants, policy, approvals | Permission/risk authority | Opaque references and current decision result; no duplicate grant table |
| Objective/campaign/run/step state | Workflow authority | Temporal workflow state; PostgreSQL read projections only |
| Operation attempts and execution evidence | Workflow/resource owner | Stable operation journal and evidence references |
| Domain events and delivery | Producing domain | PostgreSQL outbox/inbox plus JetStream transport |
| Canonical memory | Governed Memory Authority | Scoped context references only; no memory schema is introduced here |
| Live desktop/window/process state | Owning desktop node | Fresh, scoped observation/capability references |
| External provider state | Provider | Timestamped LIA projection with provenance |
| Secrets/private keys | OS secure store or credential authority | Opaque secret/key handles only |

The PostgreSQL migrations deliberately do not create canonical memory, grant, or Temporal-internal tables, nor full run/step aggregates. Migration 004 adds rebuildable API read projections for objectives, runs, grant metadata and accepted-request status. Temporal remains the canonical objective/run execution authority; Permission Authority remains the canonical grant owner. Projections cannot authorize or advance workflow state and must be rebuilt from their owners. The SQLite schema is local to one device and is not a cross-device database.

## Cross-contract field mappings

The same concept is spelled differently across the JSON, protobuf and SQL surfaces. These mappings are normative for P4; a generator or test must not infer another one.

| Concept | Manifest JSON Schema | Protobuf | OpenAPI | PostgreSQL / SQLite |
|---|---|---|---|---|
| Manifest revision | `manifest_revision` integer 1…9007199254740991 | `uint64 manifest_revision` (binary; ProtoJSON string; `known_manifest_revision`: 0 = none) | `manifestRevision` integer 1…9007199254740991 | signed `BIGINT` / SQLite `INTEGER`, bounded to 2^53−1 |
| Manifest expiry | `expires_at` | `valid_until` | not exposed | `valid_until` |
| Risk tier | `"R0"`…`"R4"` strings | `RiskTier` enum: `R0 = 1` … `R4 = 5` (0 is unspecified) | not exposed | `SMALLINT` 0…4 = R0…R4. Convert as `enum − 1`; never store the enum number |
| Sensitivity | `"S0"`…`"S3"` strings | `uint32` 0…3 | not exposed | `SMALLINT` 0…3 = S0…S3 |
| Evidence/attempt identity | n/a | `OperationResult.attempt_id` | n/a | `operation_attempt.attempt_id` |

All `int64`/`uint64` values exposed as JSON numbers (for example, device credential/revocation generations, API projection revisions and policy versions) are bounded to the same exact-integer ceiling; values outside the bound must use a decimal-string representation in a future API version.

Event digest note: `payload_digest_sha256` is the SHA-256 of the RFC 8785 canonical form of the event `data`. PostgreSQL `JSONB` does not preserve key order, whitespace or numeric text, so the outbox digest is verified by re-canonicalizing the decoded value, not by hashing stored bytes. Event payload numbers must therefore be integers in the safe range −9007199254740991…9007199254740991 or strings; floating-point values and larger integers are not permitted in signed/digested event data. NATS-01 fixtures must include canonical digest vectors for the PostgreSQL JSONB round trip.

## Required proof gates

| Gate | P4 contract work |
|---|---|
| API-01 | Validate the OpenAPI 3.1 document; OIDC claims, object-level authorization, idempotency replay, If-Match conflicts, pagination, rate-limit and error mapping, accepted-command status polling |
| ADAPTER-01 | ADP-001–ADP-024 against a reference harness; manifest schema, expiry, resource, permission and evidence tests on the exact Hyprland and Windows builds |
| RPC-01 / RPC-02 | Compile generated Rust and TypeScript clients; interoperability, TLS/device identity, deadlines, streaming, cancellation, limits and version skew |
| IPC-01 | Peer authentication, framing, schema validation, backpressure, disconnect/restart and authority-separation tests |
| DB-01 | PostgreSQL migration, transaction, account-isolation, restore and pinned Temporal/PostgreSQL compatibility tests |
| SQLITE-02 | One-writer contention, busy handling, WAL growth/checkpoint, crash recovery and backup/restore |
| TEMP-01 / TEMP-02 | Deterministic replay, worker restart, timer/signal/cancel, timeout-after-effect reconciliation and no duplicate effect |
| NATS-01 | Outbox/inbox atomicity, duplicate/reorder/redelivery, aggregate sequence gaps, quarantine and tenant isolation |
| SECURITY-01 | OIDC, mTLS/device PoP, command-signature, key rotation/revocation, permission race and secret/log inspection |
| MODEL-01 / SPEECH-01 | Synthetic provider conformance, policy routing, schema validation, locality, retention, cancellation and fallback |
| HYPR-01 / HYPR-02 / GODOT-01 | Hyprland capability discovery, same-display move verification and Godot migration/renderer evidence; these are not implied by contract drafting |
| MEM-01 | Exact vector baseline versus HNSW with hybrid FTS; scope-filtered recall and isolation |
| CONTEXT-01 / TOOL-01 | Context-package digest/closure/expiry/egress tests; tool registry filtering, malformed-stream, stale-resource and approval-digest tests |
| RETENTION-01 / ART-01 | Owner tombstone propagation, delete-wins and restore drills; content-addressed artifact integrity, scope, retention and restore |
| SUPPLY-01 / OTel-01 | Artifact-derived SPDX SBOM, license, provenance and rollback evidence; per-signal trace-context propagation, redaction and cardinality limits |
| OPS-01 / OPS-02 | Deployment, secret isolation, backup/restore, rollback and separate managed-host recovery evidence |

No gate is marked passed in this draft. See the [conformance matrix](conformance/p4-conformance-matrix.md) for required evidence and current status.

## P6-only compatibility gates

`MOBILE-01` and `HOME-01` remain P6 gates. They appear in the compatibility matrix as future platform profiles and are intentionally excluded from P4 exit criteria and the P4 conformance status table. Their absence from the P4 matrix is not a lost requirement.

## Deliberately deferred

No permanent LLM, embedding, ASR, TTS, identity-provider, hosting, object-storage, backup, certificate-authority, KMS/HSM, or telemetry-backend vendor is chosen. Exact dependency patch versions, generated-code plugin versions, PostgreSQL minor version, Temporal Server version, and deployment resource limits must be pinned in a compatibility record after the RPC/Temporal/DB proof environment is selected.

The first implementation target remains the narrow Linux-first P5 vertical slice. Windows contracts are defined, but Windows platform implementation is not implied by the Linux proof. macOS remains a later platform adapter. Mobile and Home Assistant remain later, separately authorized adapters.

## Review checklist

- [ ] P3 authority and data ownership invariants preserved.
- [ ] Protobuf package and API semantics reviewed.
- [ ] Device certificate and command-signature profile reviewed.
- [ ] Godot/Rust IPC boundary reviewed.
- [ ] Database ownership and migration scope reviewed.
- [ ] Temporal determinism and Activity idempotency reviewed.
- [ ] NATS subject/tenant/retention policy reviewed.
- [ ] Model-provider egress and tool-proposal rules reviewed.
- [ ] Conformance fixtures and test owners assigned.
- [ ] All implementation and verification claims backed by repository evidence.


## Data lifecycle, context and tool mediation

The context package is immutable for one invocation and binds principal, purpose, scope, context revision/digest, instruction bundle, expiry, source references, per-reference access decisions, sensitivity, locality, retention and token/tool-cycle budgets. A context reference is not a grant; it is rechecked at retrieval and egress boundaries. Source content remains governed by its owner and is referenced rather than copied into generic workflow/event records.

The tool registry is a versioned projection of P3.6 adapter capabilities. The broker filters it per invocation, validates a completed structured proposal against strict schema and semantic limits, resolves exact resource references, rechecks current permission/approval and then dispatches a signed operation envelope. Model text and partial streams cannot execute tools. Tool results remain untrusted until the resource owner supplies evidence.

RetentionSweepWorkflow and RetentionPropagationWorkflow orchestrate bounded retention work, but each domain keeps its own canonical tombstones and deletion. PostgreSQL work items are orchestration metadata; SQLite tombstones are local anti-resurrection markers. JetStream has finite transport retention and is not the permanent audit archive. Temporal history retention uses Temporal-supported namespace/API controls and never rewrites an active execution history.

The runtime profile separates the P5 local-first deployment from the full distributed profile. Keystore, identity, certificate, secret injection and artifact storage vendors remain open. Release eligibility requires an artifact-derived SPDX SBOM, license/notice and asset-rights review, vulnerability disposition, build provenance, signatures, and a tested update/rollback path.
