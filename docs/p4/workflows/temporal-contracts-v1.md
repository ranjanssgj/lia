# Temporal Workflow and Activity Contracts v1

**Status:** Draft — TEMP-01/TEMP-02 proof pending  
**Runtime baseline:** Temporal TypeScript SDK on Node.js 24  
**Authority:** Coordinator workflow authority

## 1. Workflow authority and database boundary

Temporal is the durable execution runtime for Run/Step lifecycles, timers, signals, retries, and worker recovery. The workflow authority remains the only writer of canonical campaign/run/step transitions and aggregate sequence.

Temporal workflow history is the durable authority for RunWorkflow/CampaignWorkflow state and transitions. The application PostgreSQL database stores domain-owned metadata, operation journals, outbox/inbox records, and rebuildable read projections. A projection/outbox activity follows a committed Temporal transition using a stable transition ID; it does not become a second workflow-state writer. PostgreSQL does not mirror Temporal's internal persistence schema. Temporal persistence uses its own database/schema and database role. A read projection may support HTTP queries but is not allowed to initiate an effect or override workflow history.

Memory Authority and Permission Authority remain separate. Workflows keep references to context and grants; they do not become owners of memory contents or grants.

## 2. Workflow types

| Workflow type | Workflow ID convention | Responsibility |
|---|---|---|
| RunWorkflow | `lia/run/{run_id}` | One finite objective execution, accepted plan snapshot, dependencies, attempts, deadlines and result |
| CampaignWorkflow | `lia/campaign/{campaign_id}/{revision}` | Campaign lifecycle, bounded schedules/triggers and independent run creation |
| ReconciliationWorkflow | `lia/reconcile/{operation_id}` | Resolve an uncertain side effect when a RunWorkflow cannot safely continue inline |
| NotificationDeliveryWorkflow | `lia/notification/{notification_id}` | Independent bounded channel delivery; never changes workflow truth |
| RetentionSweepWorkflow | `lia/retention/sweep/{schedule_revision}/{window_id}` | Select due retention policy work in bounded batches and create stable work items |
| RetentionPropagationWorkflow | `lia/retention/propagate/{work_item_id}` | Coordinate owner-domain tombstone, projection/cache/index propagation and verification |

A recurring campaign creates distinct RunWorkflow executions with immutable campaign/template revision references. A workflow ID is stable for one logical execution and must not be derived from a transient worker identity.

## 3. RunWorkflow input and output

~~~ts
export interface RunWorkflowInput {
  runId: string;
  accountId: string;
  objectiveId: string;
  campaignId?: string;
  campaignRevision?: number;
  templateId: string;
  templateRevision: number;
  acceptedPlanSnapshotId?: string;
  principalId: string;
  purpose: string;
  correlationId: string;
  deadlineAt?: string;
  executionBudget: {
    maxSteps: number;
    maxParallelSteps: number;
    maxRetriesPerStep: number;
    maxWallTimeMs: number;
    maxModelTurns: number;
    maxToolCycles: number;
  };
}

export interface RunWorkflowResult {
  runId: string;
  finalState:
    | "completed"
    | "completed_with_warnings"
    | "failed"
    | "cancelled"
    | "expired";
  reasonCode?: string;
  verifiedEvidenceReferences: string[];
  unresolvedOperationIds: string[];
}
~~~

Workflow inputs contain IDs, versions, bounded policy values and minimum required metadata. They must not contain unrestricted conversation archives, raw screen/audio/camera payloads, long-lived provider credentials, private keys, or complete memory records. Any context used by a step is fetched through the governed context boundary under current authorization.

## 4. Signals, queries and updates

| Workflow | Signal | Meaning |
|---|---|---|
| RunWorkflow | pause | Stop scheduling new steps and reach the defined safe checkpoint |
| RunWorkflow | resume | Re-evaluate current grants, devices, resources and policy before continuing |
| RunWorkflow | cancel | Prevent new side effects; reconcile any in-flight possible effect |
| RunWorkflow | approvalRecorded | Wake a step waiting for the exact operation approval; recheck digest and current grant |
| RunWorkflow | deviceAvailabilityChanged | Re-evaluate a blocked step; do not replay a stale command |
| RunWorkflow | externalEventAccepted | Re-evaluate an event condition using the authenticated/deduplicated event reference |
| CampaignWorkflow | pause/resume/revoke | Update campaign lifecycle under the current authority/version guard |

Queries expose current state, reason, blockers, and history references without mutating workflow state. An approval is a durable domain fact from Permission Authority, not a signal payload that grants authority. Signal handlers validate the current aggregate/revision and deduplicate by source event/decision ID.

## 5. Activity inventory

| Activity | Input contract | Responsibility | Idempotency/reconciliation |
|---|---|---|---|
| LoadAuthorizedContext | context references, principal, purpose, scope | Fetch minimum permitted context from Context Broker | Read-only; bind to context revision and digest |
| RequestPlanProposal | objective and context-package reference, bounded plan schema | Invoke model gateway and return a typed proposal | Invocation ID; no workflow/permission mutation |
| ValidatePlanProposal | proposal ID, plan digest, run revision | Validate DAG, budgets, capabilities, locality, permissions and success conditions | Stable validation request ID; approval remains separate |
| ResolveCapabilitySnapshot | target device, capability ID, resource reference | Read current manifest and resource freshness | Manifest revision and observation expiry returned |
| RequestAuthorizationDecision | operation digest, grant reference, policy version, risk and target | Ask Permission Authority for current decision/approval requirement | Decision ID; never treats a previous decision as permanent |
| DispatchOperation | signed command envelope, operation/attempt IDs, target | Deliver to resource-owning executor | Stable operation ID; transport retry is safe only with executor dedupe |
| ReconcileOperation | operation/attempt IDs and last known evidence | Inspect actual target/provider state after uncertain outcome | Query by idempotency/reference; no blind repeat |
| VerifyOperationEvidence | evidence refs and acceptance criteria | Independently verify the resulting state | Evidence IDs and verifier version are immutable |
| WriteTransitionProjectionAndOutbox | committed transition ID, aggregate revision and minimum event data | Persist a rebuildable read projection and outbox row after Temporal commits the transition | Stable transition/event ID; transaction is idempotent; projection never drives side effects |
| PublishDomainEvent | committed outbox event ID | Publish event to JetStream | Same event_id on every retry; publish ack is not side-effect completion |
| DeliverNotification | notification intent ID and channel preference ref | Attempt user-facing delivery | Independent bounded delivery state |
| RecordArtifactReference | artifact digest, owner/scope/classification | Promote a validated output into the authorized artifact domain | Content digest and owner idempotency key |
| SelectDueRetentionWork | policy revision, current window and batch limit | Select only due, authorized retention work without reading record content | Stable schedule occurrence and bounded batch cursor |
| CreateRetentionWorkItem | owner domain, opaque record reference, policy/revision and idempotency key | Schedule a retention/deletion request without copying record content | Stable work_item_id and owner-domain dedupe |
| RequestOwnerDeletion | work_item_id, owner domain and current authority reference | Ask canonical owner to apply/confirm its tombstone and deletion policy | Owner idempotency key; no cross-domain direct delete |
| VerifyTombstonePropagation | work_item_id and owner tombstone revision | Verify projections, search indexes, caches, replicas and artifact references observe the current tombstone | Owner proof and per-target verification references |
| PurgeExpiredProjection | owner domain, projection reference and tombstone revision | Remove expired non-authoritative projections after owner confirmation | Revision-guarded, idempotent; cannot remove canonical source data |

Activities are the only place for nondeterministic I/O, network requests, database access, model invocation, OS/provider calls, and external side effects. Activity handlers must be independently idempotent or expose a reconciliation query.

## 6. Determinism and replay rules

Workflow code:
- uses Temporal workflow APIs for time, timers, random values, and workflow-safe concurrency;
- does not call network APIs, databases, filesystem APIs, native OS functions, wall-clock time, or ordinary random generators;
- does not import provider SDKs or perform an external side effect;
- uses explicit, versioned workflow inputs and deterministic state transition functions;
- records a version marker before any incompatible workflow-code path change;
- preserves replay compatibility for histories created by supported deployed versions.

Worker versioning and workflow patch/version markers must be tested against stored histories before rollout. A workflow code change that alters command ordering, retry behavior, state interpretation, or activity input semantics is not a routine refactor.

## 7. Activity retry and timeout policy

- Each Activity declares schedule-to-start, start-to-close, schedule-to-close and heartbeat policies by activity class. Numeric values are tuned against P2.2 targets and are not frozen in this draft.
- Retry policy is finite and bounded by the Run deadline, step budget, risk class and current authority.
- Transient pre-effect failures may retry with bounded backoff.
- A timeout, worker crash, or lost response after a possible side effect moves the operation to reconciliation. It does not authorize a second effect.
- Activity retries reuse the same operation_id for the same logical effect and a new attempt_id for a new attempt. Command IDs identify individual dispatch commands; worker lease generation fences stale workers.
- An expired worker lease cannot advance canonical state. Evidence from a stale lease is retained for reconciliation but cannot complete the step.
- Cancellation requests are cooperative. The workflow stops new work and waits for Activities to reach a safe cancellation/reconciliation boundary. Cancellation is not rollback.

## 8. Durable schedule semantics

Campaign schedules preserve the P3.4 rules: timezone identifiers, schedule revision, occurrence identity, explicit misfire policy, daylight-saving behavior, overlap policy, maximum frequency and bounded catch-up. The default misfire policy remains skip and report. Temporal Schedules/Timers are runtime mechanisms; LIA owns occurrence identity and policy meaning.

## 9. History, payload and retention rules

Temporal histories contain identifiers, typed bounded metadata, decisions, digests and governed references. They do not contain secrets, full model prompts, full audio/screen/camera payloads, unrestricted memory, or full provider responses. Large permitted artifacts live in the artifact domain and are referenced by digest/owner/scope.

History retention and archival must follow the workflow data classification and deletion requirements. A deletion request must restrict future reads immediately and propagate through projections/retention jobs under P2.5/P3.5; it must not be implemented by silently rewriting an active workflow history.

## 9.1 Temporal Search Attributes and retention/deletion workflows

The initial custom Search Attribute catalog is:
- LiaAccountId (Keyword)
- LiaObjectiveId (Keyword)
- LiaCampaignId (Keyword)
- LiaDeviceId (Keyword)
- LiaRunState (Keyword)
- LiaRetentionClass (Keyword)
- LiaWorkflowRevision (Int)
- LiaCreatedAt (Datetime)

These attributes are bounded routing/operational metadata only. Do not index user names, email, prompt/context text, window titles, file paths, raw observations, secrets or provider payloads. Visibility queries are always account-scoped by the authenticated API; Search Attributes do not create authorization. Attribute registration and worker usage are versioned with the workflow deployment. Do not write Temporal internal tables directly.

RetentionSweepWorkflow selects due retention work in bounded batches and creates stable work items. RetentionPropagationWorkflow asks each canonical owner domain to apply its own tombstone/deletion/expiry, then verifies owner acknowledgement plus projection, cache, search-index, artifact and replica propagation. The PostgreSQL retention_work_item table is an orchestration journal, not the canonical tombstone store.

The owner domain decides and executes deletion under its own policy. Memory Authority deletes memory and derived context, Permission Authority handles grants, Artifact Authority handles stored artifacts, Temporal manages workflow execution history/visibility, and device agents apply local cache tombstones. A coordinator work-item state of verified means required owner/replica evidence has been collected; it does not prove removal from immutable backups outside the approved lifecycle.

Deletion of Temporal workflow histories is allowed only under the Temporal namespace/owner retention policy and supported Temporal APIs after the workflow reaches an eligible terminal state. Active workflow histories are not silently rewritten. Search Attributes and visibility projections are removed or expire under the Temporal retention process; application projections and NATS events are handled by their own owners. Restoring a backup must reapply current tombstones and revocation state before serving reads or accepting writes.

Every delete/expiry job is idempotent, account-scoped, revision-guarded, auditable and retry-bounded. A stale replica/outbox/context cannot recreate data older than the canonical tombstone. Unknown owner response remains awaiting_owner/reconciliation_required; it is not marked deleted.

## 10. TEMP-01 / TEMP-02 acceptance

- Replay representative RunWorkflow and CampaignWorkflow histories across the supported worker versions with no nondeterminism.
- Kill a worker before dispatch, after dispatch, after target effect, and before Activity completion; verify correct state and reconciliation.
- Deliver duplicate/out-of-order signals and NATS events; verify one accepted transition.
- Exercise pause, resume, cancellation, deadline, approval expiry, schedule misfire and worker lease expiry.
- Confirm a stale worker lease cannot write a transition or repeat a side effect.
- Simulate an Activity timeout after an external effect may have happened; verify reconciliation by stable operation identity and evidence.
- Confirm Temporal history contains no disallowed raw content or credential material.
- Verify Search Attribute registration/types and account-scoped visibility without indexing sensitive content.
- Crash/retry retention workflows around owner tombstone commit, projection/cache purge and verification; assert no stale replica can resurrect deleted data.
- Verify terminal workflow history retention/deletion only through the supported Temporal boundary and that active histories are not rewritten.
