# P4 Runtime and Toolchain Compatibility Matrix

**Revision:** 0.1.0  
**Status:** Draft — exact lockfile and proof evidence pending  
**Reference date:** 14 November 2026

This matrix separates the approved architecture baseline from exact dependency pins. A major line or an upstream support statement is not a reproducible build. P4 may select a candidate for a proof spike, but a production lockfile requires exact version, source, license, target OS/runtime, build result, security review and upgrade/rollback evidence.

| Area | Approved baseline | Exact pin status | Compatibility/proof rule | Gate |
|---|---|---|---|---|
| Godot presentation | Godot 4.7.2 stable; GDScript primary presentation logic | Exact engine release selected; export templates and asset/plugin hashes not pinned | Freeze editor/export-template pair; verify VRM/retarget, transparent window, focus/input, DPI and frame/resource budget | GODOT-01 |
| Rust device agent | Stable Rust + Tokio | rustc, cargo, target triples, Tokio and crate lock not pinned | rust-toolchain.toml, Cargo.lock, license/advisory review and Linux/Windows compile matrix required | RPC-01, IPC-01 |
| Rust gRPC | Tonic 0.14.x; P3.8 audit records Tonic 0.14.9 as the released candidate | 0.14.9 is the proof-spike candidate; final Tonic/prost/protoc/plugin lock pending | Generate Rust and TS clients from one proto revision; test unary/streaming, deadline/cancel, message limits, TLS, signature metadata and version skew. Successor grpc crate is not selected unless production client/server and interoperability suite are ready | RPC-01/02 |
| TypeScript runtime | Node.js 24 LTS | Exact 24.x patch and package manager version not pinned | Pin runtime image/toolchain and lockfile; Node 26 is not the baseline until Temporal SDK support and conformance are demonstrated | TEMP-01/02 |
| TypeScript compiler | TypeScript | Exact compiler version not selected | Pin compiler, tsconfig strictness and generated-client build; no hand-edited generated code | RPC-01, TEMP-01 |
| HTTP service | Fastify 5 + TypeScript | Exact Fastify/plugin versions pending | Pin core and security plugins as one compatible set; OpenAPI validation and auth hooks must be tested with authorization fixtures | API-01, SECURITY-01 |
| Temporal | Temporal TypeScript SDK + Temporal Service | Exact SDK, Server and worker package versions pending | Pin Node 24 + SDK + Server together. Keep Temporal persistence isolated from LIA application data; replay histories and migration/restore against the exact combination | TEMP-01/02, DB-01 |
| PostgreSQL | PostgreSQL 16 for LIA domain data; Temporal persistence isolated | Exact 16.x minor, extensions and server config pending | Select exact minor supported by pinned Temporal Server. Test migrations, role/schema boundaries, isolation, backup/restore, query plans and upgrade path | DB-01, OPS-01 |
| pgvector and FTS | PostgreSQL FTS + pgvector; exact vector search first | Extension release and index settings pending | Exact search remains correctness baseline. Enable HNSW only after scope-filtered recall@k, latency and isolation evidence | MEM-01 |
| SQLite | SQLite WAL for one device host only | Exact SQLite library/runtime version and PRAGMA settings pending | Confirm single-writer behavior, busy handling, long reader/checkpoint behavior and consistent online backup on supported local filesystems; never network FS | SQLITE-02 |
| NATS | NATS JetStream | Server image, client library and stream/consumer configuration pending | Pin NATS server/client together. At-least-once semantics, outbox/inbox, explicit ack, bounded retries, dedupe and retention required | NATS-01 |
| Protobuf toolchain | Proto3, package lia.v1 | protoc, Buf (if adopted), Rust/TS plugins and generated runtime versions pending | Pin schema compiler and plugins; deterministic generation diff in CI; preserve field numbers/reserved fields and compatibility checks | RPC-01 |
| OpenAPI | OpenAPI 3.1.0 | Validator, generator and Fastify adapter versions pending | Validate the checked-in document and test generated/served schema parity, auth and error contract | API-01 |
| Local IPC | Linux UDS + peer credentials; Windows named pipe + ACL; supervised stdio only as dev/test fallback | Exact Rust IPC crate/API and Windows implementation not pinned | Do not use unauthenticated localhost TCP. Test UID/PID and SID/process checks, framing, bounded buffers and restart | IPC-01 |
| TLS / identity | TLS 1.3 mTLS for device control; OIDC Authorization Code + PKCE for user sign-in | TLS provider, certificate authority, OIDC issuer, algorithms/lifetimes and rotation tooling not selected | Bind certificate SAN/key fingerprint to device ID and credential generation; token identity never substitutes for LIA grants | SECURITY-01 |
| Command signatures | Versioned Ed25519 + RFC 8785 JCS draft profile | Cryptographic implementation crate/library and key store not pinned | Publish fixed test vectors; verify domain separation, ProtoJSON canonicalization, target binding, key generation, revocation and replay | SECURITY-01 |
| OpenTelemetry | OTel API/semantic conventions; JS SDK 3.0 signal maturity; Rust signal-specific maturity gates | Exact per-signal package pins and exporter/backend pending | Pin Rust and JS SDKs separately per signal; Rust Traces Beta status must be explicitly accepted; preserve trace context end-to-end | OTel-01 |
| Deployment | OCI images + Docker Compose single-host reference profile | Base images, Compose plugin, image digests and production deployment topology pending | Pin image digests and build provenance. Compose is not an HA/DR guarantee; managed hosting needs separate RTO/RPO proof | OPS-01/02 |
| Artifact storage | Local content-addressed files; hosted S3-compatible interface | Filesystem policy, object-store vendor and KMS/HSM not selected | Content digest, scope, retention, deletion/tombstone, encryption and restore tested; no vendor selected by this matrix | ART-01, OPS-01 |
| Model/speech providers | Provider-neutral interfaces only | No model, embedding, ASR or TTS vendor/model selected | Candidate evaluation must satisfy locality, data policy, retention, quality, tool schema, safety, latency and cost. No implicit hosted fallback | MODEL-01, SPEECH-01 |
| Mobile and home adapters | Android Kotlin/Compose, iOS Swift/SwiftUI; optional Home Assistant connector | Exact SDK, OS/device matrix and connector dependency versions pending | Separate P6 compatibility profiles; no claim of general background execution or physical device support | MOBILE-01, HOME-01 |

## Lockfile and release eligibility

A dependency is release-eligible only when its exact version and transitive dependency tree are captured; its license and notices are reviewed; its supported OS/runtime matrix covers the target; deprecated or pre-stable APIs have an approved exception; security advisories are triaged; reproducible build and conformance pass; migration/upgrade/rollback is documented; and the build artifact and SBOM are retained.

P4 must record:
- exact Rust toolchain, Cargo.lock, Node patch, package manager and JS lockfile;
- exact protoc/generator/plugin versions and generated-code hashes;
- exact PostgreSQL minor, pgvector extension, SQLite runtime, NATS server/client, Temporal Server/SDK pair;
- exact TLS/crypto library, OTel per-signal versions and deployment base-image digests;
- platform OS/build, compositor, Godot editor/export template and asset/plugin provenance;
- licenses, direct/transitive SBOM, vulnerability review, model/asset/font/audio license and data-rights record.

## Candidate status

The P3.8 selected families remain unchanged. Tonic 0.14.9 is the initial RPC-01 spike candidate based on the P3.8 upstream audit, not a production lock. Node 24, Godot 4.7.2, PostgreSQL 16, Fastify 5 and protocol major baselines remain as selected. All missing patch-level pins are explicitly **Pending P4 proof** rather than guessed.

A change to an approved P3.8 technology family requires a new ADR including evidence, alternatives, compatibility impact, migration/exit plan and rollback strategy.
