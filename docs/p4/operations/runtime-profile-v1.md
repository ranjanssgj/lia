# Runtime, Packaging, Configuration and Observability Contract v1

**Status:** Draft — OPS-01/02, SECURITY-01 and OTel-01 evidence pending  
**Profiles:** P5 local-first device profile and full distributed coordinator profile

## 1. Deployment profiles

### P5 local-first profile
- Godot 4.7.2 presentation process.
- Rust/Tokio device agent and platform adapters.
- SQLite WAL local journal/cache.
- TypeScript/Node 24 conversation/tool orchestration where required by the selected P5 application packaging.
- No mandatory Temporal, PostgreSQL, NATS, hosted LLM, hosted speech, or cloud storage dependency for baseline desktop operation.
- Screen/window/process observations remain device-local by default.
- Network loss degrades remote capabilities but does not prevent local presentation or authorized local operations that remain within their grants and expiry.

### Full distributed profile
- Fastify 5 + TypeScript API and coordinator services.
- Temporal workflow service/worker fleet with its own persistence boundary.
- PostgreSQL 16 application data and separate Temporal persistence.
- NATS JetStream event distribution.
- Enrolled Rust device agents and isolated provider connectors.
- OCI container packaging; Docker Compose is a single-host reference deployment only.
- Managed-hosted availability/recovery must be proven separately from the Compose profile.

## 2. Configuration contract

Configuration is typed, versioned, validated at startup and exposed through a redacted effective-configuration report. Each setting declares owner, profile, default, allowed range, reloadability, sensitivity and whether it requires restart.

Precedence is: built-in safe defaults, deployment configuration file, non-secret environment overrides, then explicitly permitted operator CLI overrides. A secret value is never embedded in a command-line argument, container image, source file or ordinary environment variable. Environment variables may contain opaque secret references but not credential bytes.

Unknown required configuration keys and invalid security-sensitive values fail closed. Optional unknown metadata may be ignored only under a compatible config version. Runtime must expose config revision/digest, not secret values.

## 3. Keystore and secret-injection boundary

The keystore interface exposes create/import under policy, sign, decrypt where required, public-key metadata, rotate, revoke and attestation/status operations. Private keys should remain non-exportable where the target OS/store supports it. The key algorithm, hardware-backed capability and secure-store implementation are adapter capabilities, not assumed equal across Linux, Windows and future macOS.

Secret injection is just-in-time and scoped to the owning executor. The process receives a handle or bounded secret material only when required and clears transient buffers after use where runtime permits. Connector refresh credentials do not enter desktop agent, Temporal history, event payload, memory, prompts or normal logs. Device signing key, TLS key, user token, connector credential and model-provider key are distinct credential classes.

Concrete OIDC provider, certificate authority, KMS/HSM vendor, Linux/Windows keystore backend and hosted secret store remain unselected pending SECURITY-01.

## 4. Health and lifecycle

Health is separate from permission, trust and capability availability. Each process reports:
- liveness: event loop/process is responsive;
- readiness: mandatory dependencies and schemas are ready to accept the relevant workload;
- degraded: some capabilities/dependencies are unavailable while a bounded subset remains usable;
- draining: no new work accepted; in-flight work reaches safe checkpoint/reconciliation;
- failed: process cannot safely provide its contract.

Health includes component/version, config revision, last successful dependency check, queue/backpressure, event freshness, capability degradation reason and correlation ID. It excludes prompts, secrets, raw capture and private content. A healthy adapter can lack a grant; a permissioned adapter can be offline.

Shutdown is bounded and cooperative. Before exit, the agent flushes durable journals/outboxes, closes IPC/gRPC connections and reports in-flight operations. An uncertain side effect remains unknown and is reconciled after restart.

## 5. Packaging and update

Each release has immutable build identity, version, target OS/architecture, artifact digest, source revision, build provenance, SBOM, license notices, vulnerability scan summary and signature-verification metadata. Local agent, Godot app/assets, native libraries and container services are versioned as separate components with compatibility constraints.

Updates are authenticated, integrity-checked, staged and health-checked. A failed update can roll back application binaries/configuration to the previous compatible version. Rollback must not roll back revocation epochs, consumed operation/lease IDs, deletion tombstones, user corrections or external effects. Incompatible protocol/schema downgrades fail closed and require compatible peers or an explicit migration.

No auto-updater vendor, signing service, artifact store or package repository has been selected.

## 6. OpenTelemetry conventions

- W3C traceparent/tracestate context is preserved across HTTP, gRPC, Temporal Activity boundaries and NATS message headers.
- Stable attributes include service.name, service.version, deployment.environment, lia.account_id (pseudonymous opaque ID), device_id where permitted, operation_id, workflow/run/step IDs, adapter_id, contract_version and error.type.
- Do not create span names or metric labels from user text, window titles, file paths, prompt content, raw resource labels or provider payloads.
- High-cardinality IDs belong in traces/logs under access control, not metric labels.
- Secrets, access tokens, certificate/private-key bytes, prompts, memory text, screen/audio/camera data and provider request/response bodies are prohibited in telemetry.
- Metrics cover request/operation latency, queue depth, delivery attempts, retries, reconciliation count, adapter availability, capability failures, resource usage and redacted error classes.
- Logs are structured, bounded and redacted. Trace sampling must not omit security/audit state transitions from their canonical audit domain.
- Exact Rust/JS SDK versions and exporters are pinned per signal; Rust trace maturity remains an explicit review item.

## 7. OPS-01/02 and OTel-01 evidence

Record exact container/base-image digests, package metadata, config schema/revision, secret handles (never secret values), key-provider profile, health probes, update/rollback drill, restore test, trace propagation examples, exporter versions, resource overhead and redaction test results. Compose does not establish managed-hosted HA, RTO or RPO.
