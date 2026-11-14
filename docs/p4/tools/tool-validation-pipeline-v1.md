# Tool Registry and Proposal Validation Pipeline v1

**Status:** Draft — TOOL-01 conformance pending  
**Authority:** LIA tool broker, Permission Authority, Workflow Authority and resource-owning executor

## 1. Purpose and boundary

The tool registry is a LIA-owned, versioned catalog of model-visible affordances derived from P3.6 adapter capabilities. It is not a parallel permission system and does not itself authorize execution. The broker creates a task-specific projection from registered tools after filtering by target, live capability manifest, resource generation, account/project/task scope, current grant/policy, risk, confirmation path, data locality, runtime budget and provider capability.

Model-visible tool definitions are descriptive proposals. A tool call does not carry an authority token, grant, trusted resource ID, execution receipt or success claim.

## 2. Tool definition

Every registered tool has:
- stable tool ID and schema version;
- human-readable purpose, usage constraints and examples;
- exact P3.6 capability ID/version and adapter compatibility;
- resource types, permitted target classes and opaque resource-reference requirements;
- strict input/output JSON Schema references and size/complexity bounds;
- required LIA scope, minimum risk floor, confirmation behavior and purpose constraints;
- side-effect classification, idempotency class, timeout/retry/reconciliation semantics;
- verification method, evidence types and safe cancellation boundaries;
- data sensitivity, locality, retention and provider-egress restrictions;
- lifecycle, deprecation and contract-version compatibility.

The registry must reject duplicate IDs with incompatible major schema semantics. Removed fields are reserved; a breaking tool schema or behavior requires a new schema version and re-evaluation. The broker records which tool-registry revision was supplied to each model invocation.

## 3. Per-invocation projection

Before a tool list is sent to a model, the broker filters by:
1. authenticated principal and task purpose;
2. account/project/task/device scope of the context package;
3. live adapter trust/lifecycle and compatible manifest revision;
4. exact resource type and resource reference ownership/generation;
5. current grant, revocation epoch, policy version, risk floor and confirmation route;
6. current session, OS/provider permission and capability availability;
7. model/provider modality and structured-call support;
8. sensitivity, locality, retention, egress decision and allowed provider list;
9. deadline, token/tool-call budget, concurrency and quota.

A capability absent from the filtered list cannot be invoked by guessing an ID. A stale registry/manifest is refreshed before dispatch. An unavailable capability is not replaced by a generic shell, global key injection or arbitrary HTTP tool.

## 4. Proposal validation

A proposal is accepted only after the model invocation has completed a structured tool-call object. Partial stream tokens are never executable. The broker:
- resolves invocation ID, tool-call ID, tool ID and exact registry revision;
- validates input against the registered schema with unknown fields rejected;
- applies semantic bounds (length, ranges, cardinality, path/resource class, budget and deadline);
- resolves the target/resource through an authorized opaque reference rather than a model-generated display label;
- checks exact capability version, manifest freshness, resource generation/revision and owning adapter;
- rechecks current account scope, grant, purpose, policy version, revocation, risk and approval;
- computes a canonical operation digest from the exact target, operation, arguments and policy-bound claims;
- asks Permission Authority for a current decision and, where required, an approval bound to that digest;
- creates the P3.3 command envelope only after all checks pass;
- routes to the resource-owning executor, which repeats trust, support, resource and authorization checks immediately before the effect.

Any unknown tool, version mismatch, invalid field, missing reference, ambiguous resource, policy change, revoked grant or unsupported schema is rejected with a typed error and correlation ID. Replanning or clarification is a new proposal, not a relaxation of the validator.

## 5. Results and trust labels

Tool results are returned as untrusted data unless an independent verifier establishes a defined system fact. The broker labels provenance, source adapter/provider, operation ID, evidence references, data sensitivity, and verification status. A model cannot mark an operation as succeeded by writing a sentence or emitting a JSON boolean. The workflow authority transitions state only from accepted typed result/evidence.

Tool outputs are trimmed to the minimum required for the next model turn. They do not include credentials, raw secret values, unrestricted memory or unrelated provider content. Large permitted outputs are artifact references with digest and scope.

## 6. Security invariants

- A tool registry entry does not create a grant.
- Model output cannot override schema, risk, policy, user confirmation or target identity.
- No tool can grant permission, change device trust, mutate canonical memory, bypass workflow guards or use another adapter to escape a denial.
- A new provider/model does not receive more tools merely because it supports more function calls.
- Tool invocation is bounded by maximum model turns, tool cycles, plan steps, parallelism, wall time and cost/resource budget.
- A timeout after a possible effect requires reconciliation before retry.
- Revocation or material policy changes invalidate pending tool proposals and require revalidation.

## 7. TOOL-01 acceptance evidence

Tests must include partial/malformed tool-call streams, unknown tool IDs, old registry revisions, additional input fields, oversized arguments, wrong-account resources, stale manifest/resource generation, permission removal between proposal and dispatch, approval digest mismatch, capability downgrade, provider locality mismatch, tool result prompt injection, output redaction, duplicate dispatch and timeout-after-effect reconciliation. Retain fixture hashes, registry revision, model/provider metadata, broker decisions, command/operation IDs and resource-verification results.
