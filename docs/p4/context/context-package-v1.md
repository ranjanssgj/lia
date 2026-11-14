# Governed Context Package Contract v1

**Status:** Draft — CONTEXT-01 proof pending  
**Schema:** [context-package-v1.schema.json](context-package-v1.schema.json)  
**Example:** [context-package-v1.example.json](fixtures/context-package-v1.example.json)

## 1. Purpose and authority

The context package is the only model-context ingress contract. Context Broker assembles an immutable, revisioned, purpose-bound projection from authorized source references. It does not become the owner of memory, identity, workflow state, device observations or provider records. Each source remains governed by its canonical owner and its own retention/deletion policy.

A context package is not a permission grant, durable memory record, workflow state snapshot or authorization token. A successful read at assembly time does not imply indefinite cache permission, permission to write, permission to share, or permission to send data to an external provider.

## 2. Required envelope semantics

The schema binds:
- context ID, monotonic package revision, digest and finite validity interval;
- authenticated principal ID/type, explicit task purpose and exact account/project/task/device/session scope;
- immutable instruction-bundle ID/revision/digest, kept separate from user content and retrieved material;
- sensitivity, processing locality, retention class, raw-capture flag, exact egress decision and allowed provider IDs;
- input/output token limits, tool-cycle budget and deadline;
- source references with canonical owner domain, scope, exact revision, digest, sensitivity, provenance and access-decision ID;
- context items with kind, source-reference closure, content reference/digest, trust class, sensitivity, purpose, validity and derivation lineage.

## 3. Canonical digest

digest_sha256 is the lowercase hexadecimal SHA-256 digest of the UTF-8 RFC 8785 JSON Canonicalization Scheme representation of the complete package object with the top-level digest_sha256 property omitted. JSON object keys are canonicalized; array order is preserved. A consumer must reject digest mismatch and must not attempt a looser reserialization.

Each referenced source/item also carries its own SHA-256 digest. A package digest proves integrity of the package envelope, not truth or authorization of its contents. The source owner and access decision must still be checked. The checked-in example uses synthetic digest sentinel values to test schema shape only; it is not a digest-valid vector. CONTEXT-01 must use separately generated, fixed canonicalization/hash vectors.

## 4. Source resolution and scope

Every items.source_reference_ids entry must resolve to a source_references[].reference_id in the same package. Every source reference must be authorized for the exact principal, purpose and scope. A project/task/device-scoped item cannot be broadened by embedding it in an account-scoped package. Scope is not transitive.

The Context Broker verifies:
1. caller/session identity and purpose;
2. owner domain and stable source reference;
3. exact source revision and digest, or a source-owner refresh if the revision changed;
4. source scope and current access-decision ID;
5. sensitivity/locality/retention rules and deletion tombstone/revocation state;
6. whether the item is needed for the current task and whether its content may be copied or only referenced;
7. provider/region egress eligibility immediately before model dispatch.

Missing owner, ambiguous scope, expired freshness, deleted/tombstoned source, denied access or unresolved revision conflict causes the item/package to be excluded or the invocation to be rejected. It is never silently replaced with a broader memory query.

## 5. Trust and instruction separation

The trust_class is semantic metadata, not a prompt-instruction marker that a model can self-assign. Trusted system instructions come only from the versioned instruction bundle. User content remains user content. Retrieved pages, documents, tool results, provider events and other external material are untrusted data even when their text contains imperative language.

Only an explicitly defined owner-authenticated fact may use verified_system_fact; model inference remains inferred and cannot overwrite a user correction or canonical record. Summaries preserve source references and derivation lineage. No retrieved content can add tools, widen grants, change policy, or modify the instruction bundle.

## 6. Sensitivity, locality, retention and egress

- S0/S1/S2/S3 retain the P2.5 definitions. S2 requires explicit retention/processing authorization. S3 credentials, keys and equivalent secrets are not context items.
- local_only means no provider network egress; a local model/runtime candidate must be explicitly selected and verified for the target.
- approved_region and any_approved_region are still constrained by exact provider capability/contract, user policy and egress-decision ID.
- Fallback is allowed only to a provider/region with the same or stricter locality, retention, training-use and sensitivity terms.
- Raw screen/audio/camera content is excluded by default; a task may include a minimized, explicitly authorized derived observation or artifact reference.
- Context package retention is transient by default and does not authorize provider-side retention. Provider-side retention is a separate vendor-evaluation and policy decision.
- Context caches/indexes are disposable projections; deletion/tombstones propagate through derived summaries and indexes under P3.5/P4 retention workflows.

## 7. Invocation lifecycle and cancellation

A package is assembled for one model invocation/task scope. Before provider dispatch the gateway checks package expiry, digest, instruction-bundle revision, egress decision, provider/region eligibility, token/tool-cycle budget and all referenced access/deletion decisions. If access changes during assembly, the package is rebuilt or rejected.

Cancellation stops new model/tool cycles. It does not undo an already-started external action; a related operation follows its own P3.4 reconciliation path. A provider stream is not allowed to execute a tool as tokens arrive; completed tool proposals use the TOOL-01 pipeline.

## 8. CONTEXT-01 acceptance

- Validate schema and canonical digest using fixed test vectors.
- Reject an item with an unresolved source-reference ID or mismatched source digest.
- Reject cross-account/project/task/device scope expansion.
- Reject expired package, source freshness or access decision.
- Revoke/delete a source between assembly and dispatch; prove the gateway blocks egress.
- Attempt S3/secret injection and raw-capture inclusion without explicit authorization; prove rejection.
- Attempt a provider fallback with weaker locality/retention terms; prove it is ineligible.
- Confirm logs contain references/digests and metadata only, not context text or source content.
- Confirm cancellation prevents subsequent tool cycles and preserves reconciliation for any started operation.
