# P4 Conformance Matrix

**Revision:** 0.2.0  
**Status:** Draft — all implementation proofs are Not Run  
**Rule:** A contract is not Verified by documentation, linting, code generation, or a vendor statement alone.

## Status vocabulary

- **Draft:** Interface/schema is proposed and reviewable.
- **Not Run:** Test has not been executed against a pinned implementation.
- **Passed:** Required fixtures and raw results are retained and reviewed.
- **Blocked:** A required dependency or decision prevents execution.
- **Failed:** The acceptance criteria were not met; remediation is recorded.

## Gate matrix

| Gate | Contract artifacts | Required procedure | Required evidence | Current status |
|---|---|---|---|---|
| RPC-01 | proto, security profile | Generate Rust and TypeScript clients from the same proto revision; unary and bidi calls; version skew; message limits; malformed payloads; signature validation | Compiler/codegen versions, generated-client hashes, interop logs, fixtures, error mapping, signature test vectors | Not Run |
| RPC-02 | proto, security profile | TLS 1.3/mTLS on local LAN and relayed/remote paths; reconnect; deadline/cancel; HTTP/2 backpressure; target-device proof and revocation | Certificate/profile, peer identity, expiry/revocation, trace IDs, latency/backpressure and relay-tamper results | Not Run |
| API-01 | OpenAPI contract, security profile | Validate OpenAPI 3.1 document; test OIDC issuer/audience/expiry/client/scope validation, object-level authorization, idempotency replay, If-Match conflicts, pagination and error mapping | Validator/generator versions, schema report, positive/negative auth fixtures, cross-account access results, replay/idempotency logs | Not Run |
| ADAPTER-01 | Adapter contract, manifest JSON Schema, Hyprland and Windows UIA fixtures | Run ADP-001–ADP-024 against a reference harness; schema/expiry/resource/permission/evidence tests; execute profile-specific capability tests on the exact Hyprland and Windows build | Schema validator version, fixture hashes, OS/compositor/UIA provider versions, test report, operation/evidence logs and exact supported-capability matrix | Not Run |
| IPC-01 | IPC contract, security profile | Linux UDS peer UID/PID, Windows named-pipe ACL/SID, malformed/oversized frames, renderer restart, stale-frame handling, malicious UI client | OS/build matrix, frame fixtures, peer identity logs, restart and backpressure report | Not Run |
| DB-01 | PostgreSQL migrations 001/002 | Apply both migrations to clean PG16 database; upgrade from prior schema; ownership/constraint tests; retention-work-item idempotency; account isolation; transaction crash windows; backup/restore | Exact PG minor, migration hashes, SQL test output, query/lock measurements, retention-work attempt evidence and restore report | Not Run |
| SQLITE-02 | SQLite migrations 001/002 | Multiple readers/writers, SQLITE_BUSY, WAL growth/checkpoint, crash recovery, online backup/restore, and tombstone-before-sync ordering with stale outbox rejection | SQLite runtime version, filesystem, PRAGMA settings, contention/recovery logs and tombstone anti-resurrection fixtures | Not Run |
| TEMP-01 | Temporal contracts and Search Attribute catalog | Crash/restart worker; replay historical runs; register/query scoped Search Attributes; timers, signals, cancellation, lease fencing and workflow-code versioning | Temporal Server/SDK versions, namespace/attribute configuration, synthetic history fixtures, replay results and account-scoped visibility assertions | Not Run |
| TEMP-02 | Temporal and retention workflows | Timeout after possible effect; duplicate Activity; lost response; reconcile before retry; retention sweep/propagation across owner tombstones and non-authoritative projections | Target/provider simulator logs, operation IDs, owner tombstone revisions, per-replica verification and no-resurrection assertions | Not Run |
| NATS-01 | Event contract, subjects, PG migrations 001/002 | Outbox crash windows, duplicate/reordered delivery, sequence gaps, consumer crash, quarantine, per-account ACL, bounded transport retention and replay from canonical owner history | NATS version/config, finite stream limits, event fixture IDs, inbox/outbox DB state, owner-history replay and tenant-isolation report | Not Run |
| SECURITY-01 | Security profile, OpenAPI, proto, IPC | OIDC token checks; mTLS identity binding; Ed25519/JCS valid/invalid/tampered signatures; key rotation/revocation; stale policy; replay; log/secret inspection | Threat model, certificate/key profile, deterministic signing vectors, revocation race results, secret scan and reviewer sign-off | Not Run |
| MODEL-01 | Provider interfaces and synthetic evaluation harness | Synthetic providers for refusal, malformed output, prompt injection, locality/retention mismatch, timeout, cancellation, rate limits and fallback constraints | Harness/build and fixture hashes, provider policy profiles, schema-fuzz results, data-egress traces, quality/latency/cost/retention report | Not Run |
| SPEECH-01 | Provider interfaces | Synthetic/opt-in audio, transcript confidence, language/accent, endpointing, interruption and failure fallback | Consent fixture inventory, transcript metrics, interruption P95, retention/deletion evidence | Not Run |
| HYPR-01 | Proto, adapter manifest | Discover Hyprland session and event socket; reconnect/event loss; capability manifest; window generation/freshness | Hyprland version, session variables, manifest, event and reconnect logs | Not Run |
| HYPR-02 | Adapter operation contract | Same-display move preserving size; stale window ID reuse; post-move geometry verification | Before/after geometry, display/workspace, operation ID and independent verification evidence | Not Run |
| GODOT-01 | IPC contract | Upgrade a controlled copy to Godot 4.7.2; VRM import/retarget, animation, transparent window, focus/input, DPI/scaling, idle/interactive frame budget | Asset/import report, animation evidence, OS/compositor matrix, resource report | Not Run |
| MEM-01 | Provider/context interfaces | Exact vector search baseline and HNSW comparison with hybrid FTS; filter by account/project/task scope | Recall@k and filtered recall, latency distribution, index cost, leakage/isolation fixtures | Not Run |
| CONTEXT-01 | Context package JSON Schema and provider interfaces | Validate context package schema, canonical digest, source-reference closure, per-reference access decision, expiry, scope, sensitivity/locality, provider egress and revocation/deletion between retrieval and invocation | Schema/validator versions, synthetic fixture hashes, digest vectors, cross-scope denial, expired/revoked context traces and egress-decision evidence | Not Run |
| TOOL-01 | Tool registry schema and validation pipeline | Test registry versioning, per-invocation capability filtering, partial/malformed streams, unknown tools/fields, oversized arguments, stale resources, approval digest mismatch, revocation race and output trust labels | Registry revision, proposal fixtures, broker decisions, denied-effect assertions, operation/evidence IDs and redacted traces | Not Run |
| RETENTION-01 | PostgreSQL/SQLite retention migrations, Temporal workflows, NATS tombstone events | Tombstone at each canonical owner; propagate to derived context, indexes, artifacts, projections, device caches and restore paths; prove delete-wins and no stale resurrection | Owner tombstone IDs/revisions, workflow attempts, per-target purge/verification reports, restore drill and unresolved-owner handling | Not Run |
| SUPPLY-01 | Supply-chain/release contract | Generate artifact-derived SPDX SBOM; review direct/transitive licenses and bundled asset/model/font/audio rights; scan vulnerabilities; verify build provenance/signature and update/rollback | SBOM/tool version/hash, license notices, asset rights inventory, vulnerability disposition, provenance and rollback evidence | Not Run |
| OTel-01 | P4 observability compatibility profile | Validate trace context propagation across API, gRPC, workflow Activity, NATS and device agent; per-signal SDK maturity; redaction and cardinality limits | Exact JS/Rust per-signal versions, trace examples, export results, privacy/log inspection and overhead report | Not Run |
| ART-01 | Artifact interface and lifecycle policy | Verify content-addressed digest, owner/scope access, integrity, retention, deletion tombstones, backup exclusion of secrets and authorized restore | Storage adapter version/config, digest/access/deletion fixtures, restore report and object lifecycle evidence | Not Run |
| OPS-01 | SQL, deployment notes | Deploy/upgrade/rollback single-host Compose profile; encrypted backup and restore | Image digests, config, secret isolation, restore/rollback drill | Not Run |
| OPS-02 | Deployment architecture | Independently test managed-hosted failure domains, recovery, RTO/RPO and availability objectives | Topology, failure injection, database/event/workflow recovery, restore evidence against NFR-036–044 | Not Run |

## Cross-contract invariants to assert in every relevant test

1. A capability manifest reports support/availability; it does not grant permission.
2. A valid authenticated session does not imply an operation is authorized.
3. A stale target/resource generation, expired command, revoked grant, changed policy version or wrong device fails closed.
4. Duplicate command/event/Activity delivery cannot produce a second side effect.
5. An acknowledgement is not evidence of an effect; verified completion requires resource-appropriate evidence.
6. A possible side effect with missing evidence enters reconciliation, not blind retry.
7. A worker lease fences stale workers but is not an authorization grant.
8. A model tool proposal is fully received, structurally validated, policy-checked and broker-dispatched before execution; partial stream tokens never execute.
9. Godot cannot invoke OS capabilities or mutate canonical LIA state.
10. Tenant/account isolation is tested at HTTP, gRPC, SQL, NATS, Temporal signals and artifact-reference boundaries.
11. Secrets, raw screen/audio/camera and unrestricted memory are absent from ordinary logs, event payloads and workflow history.
12. Unsupported platform features return an explicit typed unavailable/restricted result.

## Evidence record format

Every test report must retain:
- gate ID and contract revision/hash;
- test source revision and build artifact digest;
- exact OS, runtime, database, broker, Temporal, TLS and generated-client versions;
- fixture and account/device identities using synthetic data;
- procedure, expected result and actual result;
- raw output/logs with secrets and protected content redacted;
- failed assertions, issue references and remediation;
- reviewer and date.

A test is Passed only after the evidence is reproducible and reviewed. This matrix intentionally reports no test as Passed at contract-drafting time. Synthetic JSON fixtures receive structural schema checks as part of draft review; that check is not counted as a passed conformance gate.
