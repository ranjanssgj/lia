# Synthetic Model-Provider Evaluation Harness v1

**Status:** Draft — MODEL-01 proof pending  
**Provider policy:** No production LLM, embedding, ASR or TTS vendor selected

## 1. Purpose

P4 defines a deterministic harness using synthetic provider adapters and synthetic content. The harness verifies that LIA's provider abstraction, context broker, tool proposal validator, data policy and workflow integration behave correctly without selecting or contacting a production model provider.

The harness is not a general model-quality benchmark and does not authorize production provider use. ADR-143 remains the provider selection gate.

## 2. Synthetic provider set

The harness should include deterministic providers that can:
- return a schema-valid response;
- return malformed or extra-field structured output;
- emit partial tool-call arguments and then timeout/cancel;
- propose an unknown or unauthorized tool;
- return a refusal or policy-blocked response;
- simulate rate limit, quota exhaustion, authentication failure and service outage;
- return prompt-injection-like content in retrieved documents and tool results;
- report different modalities, region, retention, training-use and capability profiles;
- delay or fail mid-stream and expose cancellation behavior;
- produce controlled speech transcripts/synthesis failures and embedding vectors with known nearest neighbors.

No test fixture contains real user prompts, secrets, private records or real audio unless a separately consented opt-in test is explicitly documented.

## 3. Evaluation dimensions

| Dimension | Required assertion |
|---|---|
| Interface conformance | Normalized request/response/error types validate; provider-specific fields remain namespaced |
| Structured output | Invalid output and unknown fields are rejected before tool/workflow mutation |
| Tool safety | Partial streams never execute; tool proposals pass the broker, permission and resource-owner checks |
| Instruction trust | Retrieved documents, tool output and provider event content remain untrusted data, not system instructions |
| Context governance | Context package revision/digest, source scopes, sensitivity, locality and expiry are preserved |
| Data egress | Disallowed provider/region/retention/training profile is ineligible regardless of quality/latency |
| Cancellation | User/task cancellation stops streaming and new tool cycles; already-started effects are reconciled |
| Fallback | Fallback only to a provider satisfying the same or stricter task, locality, sensitivity, retention and safety constraints |
| Boundedness | Model turns, tokens, tool cycles, steps, concurrency, wall time and estimated cost remain within declared budget |
| Refusal/failure | Refusal, incomplete response, quota and transient failure map to typed outcomes; no silent success |
| Provenance | Provider/model/version, context digest, instruction bundle revision, timing and diagnostics are retained without sensitive payload logging |
| Retention | Raw input/audio and provider-side retention claims are checked against the selected data policy; no provider is assumed compliant |

## 4. Security and adversarial fixtures

- Retrieved text instructs the model to reveal secrets, alter policy or execute undeclared shell commands.
- A model emits a tool call whose target identifier names another account's resource.
- A model returns a valid tool name but malformed, unbounded or extra-field arguments.
- A stream contains a partial tool call followed by cancellation.
- Provider metadata claims local processing while its capability profile declares remote-region processing.
- A fallback provider has better latency but weaker retention/locality terms.
- Tool output claims that an action succeeded while independent verification reports the old state.
- A malicious provider diagnostic/error string includes tokens, credentials or executable content.
- Context is revoked or deleted between retrieval and invocation.
- A stale model session attempts to reuse a prior tool call or approval.

Every negative security fixture must assert that no unauthorized side effect or state transition occurred. The accepted count for unauthorized tool dispatch is zero.

## 5. Quality/latency/cost report

For each candidate evaluated after ADR-143 opens, retain task suite version, synthetic prompt/data fixture IDs, model/provider/version, modality, routing policy, region, input/output usage, wall/first-token latency, completion/refusal/schema pass rate, tool proposal error rate, task quality rubric and estimated cost. Compare only providers evaluated with the same dataset, rubric and policy. Do not report synthetic results as production user quality.

Thresholds for task quality, P95 latency and cost per task are set during the vendor evaluation record after P2.2 budgets and workload classes are measured. They are not invented by this contract draft.

## 6. MODEL-01 acceptance evidence

A passing report includes synthetic provider source/build digest, fixture hashes, context and registry revisions, policy/routing decision records, exact provider capability descriptors, trace IDs, result metrics and reviewer sign-off. Live model providers remain disabled until ADR-143 is approved and privacy/retention/egress review is complete.
