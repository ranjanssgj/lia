# P4 — Integration Contracts

**Contract package:** P4.1  
**Revision:** 0.1.0  
**Status:** Draft — review and conformance evidence required  
**Baseline:** LIA Project Blueprint v1.14.0 (P4 in progress)  
**Date:** 14 November 2026

## Purpose

P4 turns the approved P2 requirements and P3 architecture into concrete, versioned interfaces. This package defines the first reviewable contract baseline for device/control-plane RPC, authenticated user APIs, the Godot presentation bridge, PostgreSQL and SQLite persistence, Temporal workflows and Activities, NATS events, model-provider adapters, and conformance testing.

This is a contract design package. It does not claim that the services, adapters, migrations, generated clients, security profile, or integrations have been implemented or tested.

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
| Device/control-plane RPC | [control_plane.proto](proto/lia/v1/control_plane.proto) | lia.v1 | Device registry, session authority, and operation executor |
| User/coordinator HTTP API | [lia-v1.yaml](openapi/lia-v1.yaml) | /v1 | Authenticated API plus domain authorization |
| Presentation IPC | [presentation-agent-v1.md](ipc/presentation-agent-v1.md) | 1 | Rust local agent; Godot is presentation-only |
| Security profile | [security-profile-v1.md](security/security-profile-v1.md) | 1 | OIDC, device mTLS, signed commands and local peer identity |
| PostgreSQL schema | [001-control-plane.sql](sql/postgres/001-control-plane.sql) | 001 | Device/control projection, operation journal, event delivery |
| SQLite schema | [001-device-agent.sql](sql/sqlite/001-device-agent.sql) | 001 | One device installation's local journal/cache |
| Event envelope and delivery | [event-contract-v1.md](events/event-contract-v1.md) | 1 | Producing domain and durable consumer |
| Event subjects | [subjects.yaml](events/subjects.yaml) | lia.v1 | Domain event owner |
| Temporal boundaries | [temporal-contracts-v1.md](workflows/temporal-contracts-v1.md) | 1 | Temporal workflow authority |
| Provider interfaces | [provider-interfaces-v1.ts](providers/provider-interfaces-v1.ts) | 1 | LIA model gateway |
| P4 gate matrix | [p4-conformance-matrix.md](conformance/p4-conformance-matrix.md) | 0.1 | Procedure, expected evidence and status for architecture proof gates |
| Adapter contract | [adapter-conformance-v1.md](adapters/adapter-conformance-v1.md) | 1 | Manifest semantics, resource lifecycle, execution checks and 24 common adapter conformance cases |
| Manifest schema | [capability-manifest-v1.schema.json](adapters/capability-manifest-v1.schema.json) | 1 | Strict JSON Schema for adapter capability manifests |
| Hyprland fixture | [hyprland-wayland-manifest.example.json](adapters/fixtures/hyprland-wayland-manifest.example.json) | Example | Synthetic first-Linux-profile manifest fixture |
| Compatibility matrix | [compatibility-matrix.md](compatibility-matrix.md) | 0.1 | Approved stack baselines, exact-pin status and proof-gate mapping |

## Authentication and signing baseline proposed for P4 review

This package proposes the following protocol-level contract. These choices are not vendor selections and remain subject to SECURITY-01 evidence and review.

- Interactive user authentication uses OpenID Connect with Authorization Code + PKCE. User-facing API calls use an access token whose issuer, audience, validity, client and applicable scopes are validated. The identity-provider vendor and issuer URL remain deployment configuration.
- Device/control-plane gRPC uses TLS 1.3 with a device-specific X.509 client certificate. The certificate identity is mapped to the enrolled installation and credential generation; an authenticated connection is not a standing operation grant.
- Command and eligible offline-lease envelopes use a versioned Ed25519 signature profile over RFC 8785 canonical JSON, with domain separation and a key identifier. The signing input is ASCII `LIA-COMMAND-V1\n` followed by the UTF-8 RFC 8785 canonical JSON form of the ProtoJSON CommandEnvelope; signature fields are excluded. The exact key-provisioning/rotation implementation and protected-storage vendor remain open. TLS keys and command-signing keys are distinct.
- Local IPC authenticates the peer using OS process credentials and restrictive endpoint ACLs. It is local-only; no unauthenticated localhost TCP listener is permitted.
- OIDC issuer, certificate authority, key-management/KMS/HSM vendor, hardware-backed-key availability, hosted secret store, and deployment region are not selected here.

The Ed25519 profile must be reconsidered before production if a FIPS or other cryptographic compliance requirement is introduced. The repository currently does not establish such a requirement. The security conformance suite must include valid, invalid, expired, rotated, revoked, replayed, altered, and out-of-order credentials and commands.

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

The PostgreSQL migration deliberately does not create canonical memory, grant, Temporal-internal, or full run/step tables. Those domains have separate owners. The SQLite schema is local to one device and is not a cross-device database.

## Required proof gates

| Gate | P4 contract work |
|---|---|
| RPC-01 / RPC-02 | Compile generated Rust and TypeScript clients; interoperability, TLS/device identity, deadlines, streaming, cancellation, limits and version skew |
| IPC-01 | Peer authentication, framing, schema validation, backpressure, disconnect/restart and authority-separation tests |
| DB-01 | PostgreSQL migration, transaction, account-isolation, restore and pinned Temporal/PostgreSQL compatibility tests |
| SQLITE-02 | One-writer contention, busy handling, WAL growth/checkpoint, crash recovery and backup/restore |
| TEMP-01 / TEMP-02 | Deterministic replay, worker restart, timer/signal/cancel, timeout-after-effect reconciliation and no duplicate effect |
| NATS-01 | Outbox/inbox atomicity, duplicate/reorder/redelivery, aggregate sequence gaps, quarantine and tenant isolation |
| SECURITY-01 | OIDC, mTLS/device PoP, command-signature, key rotation/revocation, permission race and secret/log inspection |
| MODEL-01 / SPEECH-01 | Synthetic provider conformance, policy routing, schema validation, locality, retention, cancellation and fallback |
| HYPR-01 / GODOT-01 | Hyprland capability discovery and Godot migration/renderer evidence; these are not implied by contract drafting |
| OPS-01 / OPS-02 | Deployment, secret isolation, backup/restore, rollback and separate managed-host recovery evidence |

No gate is marked passed in this draft. See the [conformance matrix](conformance/p4-conformance-matrix.md) for required evidence and current status.

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
