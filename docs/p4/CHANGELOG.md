# P4 contract package changelog

## 0.2.1 — 14 November 2026

**Baseline:** Blueprint v1.14.2. The branch incorporates Claude's supplied `lia-fix-p4.bundle` commit and `p4-contract-consistency.patch`, after independent review and additional corrections. The patch and bundle contain the same seven-file commit (`df9a3e6`).

Reconciles cross-contract fields and closes draft persistence gaps identified during independent review. This revision remains Draft; no conformance gate is marked Passed.

- Normalizes manifest revision as a monotonic unsigned Protobuf field, bounded to the JSON/JCS-safe integer range `1..9007199254740991` for JSON and SQL surfaces.
- Aligns `AcceptedCommand.statusUrl` with `GET /v1/requests/{requestId}` and an explicit rebuildable request-status projection. Defines HTTP 429 with a delay-seconds `Retry-After` header.
- Renames operation-evidence response identifiers to `*_attempt_ids`; adds event `source_event_id` and `sensitivity` fields.
- Adds PostgreSQL account-consistent references and an issuance ledger whose claims must match the operation's target, resource generation/revision/observation time/owner, arguments digest, purpose, grant, policy/revocation version and risk tier. Adds device-side SQLite consume-before-effect and nonce/session replay ledgers; operation claims cannot be changed after lease issuance, and consumption requires both a recorded nonce and a durable possible-effect marker.
- Adds PostgreSQL rebuildable ObjectiveSummary, RunSummary, GrantSummary and RequestStatus projections plus a bounded API idempotency-key record. They are explicitly non-authoritative and preserve Temporal and Permission Authority ownership.
- Adds PostgreSQL and SQLite migration 004 for interoperable JSON-safe integer bounds and summary projections.
- Adds rebuildable objective, run, grant-metadata and request-status projections plus bounded API idempotency records without moving canonical ownership from Temporal or Permission Authority.
- Documents cross-contract mappings and distinguishes P4 gates from MOBILE-01/HOME-01, which remain P6 work.

### Review verification

Independent checks executed in this review: SQLite migrations 001–004 on SQLite 3.46.1; exact lease-to-operation binding tests (target, resource revision, observation timestamp, owning device and argument digest), rejection of R2 leases, nonce replay rejection, consume-before-effect ordering and operation-claim immutability; JSON Schema Draft 2020-12 meta-validation and all four fixture validations; YAML parsing plus structural assertions for OpenAPI 3.1, the request-status route and HTTP 429 on all 11 operations; strict TypeScript checking of the provider interface file; relative-link checks for the P4 README and blueprint. These checks are not full conformance gates.

Claude's supplied validation note reports that its sandbox compiled the Protobuf file, validated OpenAPI 3.1, applied PostgreSQL migrations to a real PostgreSQL 16 server, applied SQLite migrations, validated schemas/fixtures and ran strict TypeScript checking. These are reported by Claude, not independently reproduced in this review. The current review environment did not have PostgreSQL or `protoc`; it therefore did not independently execute PostgreSQL migrations, Protobuf code generation/compilation, a full OpenAPI 3.1 validator, generated-client interoperability, security, Temporal, NATS, live adapters or end-to-end conformance. All conformance gates remain Not Run.
