# MODEL-01 — Synthetic Model Provider, Prompt Injection Refusal, and Untrusted Tool Proposals

**Gate:** MODEL-01 (partial implementation checks).  
**Gate Status:** **Not Run**. This run validates local synthetic model provider deterministic reply generation, prompt injection detection and refusal (Zero-trust / Invariant 1), untrusted tool proposal generation, context package integrity verification, and provider locality enforcement (zero network egress, ADR-143). Conformance gate remains **Not Run** until live model provider evaluation under owner-approved runtime conditions.  
**Date:** 23 November 2026.  
**Procedure Revision:** P5 M5 task plan (`docs/p5/tasks/M5-conversation-memory.md`).  
**Environment:** Linux 6.18.2-arch2-1 x86_64; rustc 1.89.0 (pinned); rusqlite 0.41.1; SQLite 3.54.1.

## Scope and Invariants Validated
1. **Model Proposal Invariant (Invariant 1):** Model responses are strictly untrusted proposals. When an action or tool is requested, the model emits an `UntrustedToolProposal` containing capability name and serialized JSON arguments; it never exercises direct execution authority.
2. **Deterministic Prompt Injection Refusal:** The model provider detects adversarial override attempts (e.g., prompt injections attempting to bypass system prompts or drop database tables) and deterministically refuses execution with an explicit refusal reason.
3. **Context Package Verification and Digest Check:** Context packages compute a canonical SHA-256 digest over JCS-canonicalized metadata and items. Missing digest, corrupted digest, or expired TTL (`expires_at_us < now_us`) causes validation to fail closed before any provider invocation.
4. **Zero Remote Egress (ADR-143):** Locality policies strictly reject unauthorized egress destinations (e.g., external endpoints) when evaluating context items bound to `DataLocality::LocalOnly`.
5. **Turn Interruption (AC-002):** Atomic interruption flags (triggered by `chat.stop` / `ui.chat.interrupt`) are evaluated cooperatively, immediately halting ongoing generation and returning `TurnResult::Interrupted`.

## Raw Output and Checksums
- Raw test run: [`raw.txt`](raw.txt)
- Checksum file: [`raw.sha256`](raw.sha256)
- Checksum: `d5905f11c71e7eb4e9ce5403d56fc1d7b3427a6b7256a7903aaceec865d26ca1`

## Results
- Synthetic model provider and context packaging tests passed.
- Conformance gate MODEL-01 status: **Not Run**.
