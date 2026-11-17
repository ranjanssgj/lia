# M5 task plan — conversation and memory loop

**Status:** Planned; implementation not started. **Gate status:** MODEL-01, SPEECH-01 and MEM-01 remain Not Run.

**Acceptance conditions:** AC-002 and AC-009, followed by the end-to-end AC-001–AC-011 regression described in the P5 plan. The first implementation slice uses deterministic synthetic providers and synthetic content; no production provider is selected or contacted.

## Contract inputs and dependencies

- P4 package revision 0.3.1 Draft: [provider interfaces](../../p4/providers/provider-interfaces-v1.ts), [synthetic evaluation harness](../../p4/providers/synthetic-evaluation-harness-v1.md), [context package](../../p4/context/context-package-v1.md) and schema, [tool validation pipeline](../../p4/tools/tool-validation-pipeline-v1.md), and the P4 conformance matrix.
- M1 operation/outcome/evidence persistence, local preference provenance and retention/deletion boundaries must be reviewed before persisting user data.
- M2 must provide the versioned presentation snapshot and typed user-control schemas, renderer-session binding and safe cancellation/stop path before wiring conversation state or controls into Godot. The renderer is never a tool executor.
- M4 presentation work depends on the reviewed M2 IPC contract and M3 capability profile. Use synthetic context and mock interfaces until those are ready.

## In-scope behavior

1. Implement typed text-turn lifecycle and a synthetic speech path for push-to-talk, capture-state indication, interruption, pause and stop. Listening must be user-controlled and visibly distinguish capturing, paused, failed and stopped states.
2. Use deterministic synthetic model, speech and embedding implementations first. Exercise malformed output, refusal, timeout, cancellation, rate limits, locality mismatch and prompt-injection fixtures without network egress.
3. Keep model output untrusted. Tool-shaped content is only a proposal and must pass the versioned tool registry, context checks, current permission policy and deterministic broker before any later executor. This task's conversation loop must not directly invoke an OS adapter.
4. Persist at most the first approved low-sensitivity preference through the M1 store, with provenance, scope, source reference and retention metadata. Provide user inspection and deletion; deletion must follow the canonical owner/tombstone path and resist stale restore.
5. Keep context packages purpose-bound, expiring, digest-checked and locality/egress-filtered at invocation time. No raw microphone audio, full transcripts, private prompts or credentials enter ordinary logs/evidence.

## Test and evidence plan

| Area | Required cases | Evidence |
|---|---|---|
| Conversation lifecycle | Text and synthetic push-to-talk, interruption, pause/stop, restart, partial/malformed transcript and provider timeout | Synthetic trace IDs, state transitions, cancellation behavior and fixture hashes |
| Synthetic model | Refusal, prompt injection, malformed/oversized output, undeclared tool proposals, timeout, cancellation and locality mismatch | Harness/build digest, policy decisions, schema results, zero-egress proof and redacted traces |
| Memory preference | Allowed persistence with provenance; wrong scope, missing source, stale revision and denied write; inspect and delete | Synthetic record IDs, owner/revision/tombstone trace and no-resurrection check |
| Privacy/locality | Context expiry/digest mismatch, access revocation, provider egress block and log/prompt redaction | Context/fixture hashes, routing decision, egress trace and privacy review |
| End-to-end slice | AC-001–AC-011 with synthetic providers, mock IPC/adapter until approved dependencies are available | Reproducible acceptance report, exact tool versions, failures and reviewer; no gate promotion from partial runs |

Retain redacted procedure output under `evidence/MODEL-01/<run-id>/`, `evidence/SPEECH-01/<run-id>/` and `evidence/MEM-01/<run-id>/`. Record commit, contract and fixture revisions/hashes, runtime/tool versions, command, result, limitations and reviewer. Never retain credentials, private user conversations, raw audio, real provider payloads or unredacted prompts.

## Exit criteria

- The synthetic harness covers all defined provider failure and adversarial cases with network egress disabled.
- Conversation and capture controls have visible, truthful lifecycle states and reliable interruption/stop behavior.
- The single allowed preference has provenance and supports inspection/deletion through the canonical store/owner path.
- Context access, expiry, digest, locality and egress checks are demonstrated at invocation time.
- End-to-end acceptance is reviewed against every AC; update gate/AC records only after the full corresponding procedures and review. No gate or AC is marked Passed by synthetic tests alone.

## Owner decisions — unresolved

Do not select ASR, TTS, LLM, embedding, identity, hosting or storage vendors. The owner must decide whether and when to evaluate live candidates under ADR-143 and the provider-evaluation policy. Hardware/GPU tier, supported languages, hosted evaluation budget/timebox, and project license remain open. This task starts with no live provider, no egress and synthetic fixtures only.
