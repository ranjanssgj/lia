# P5 — First Vertical Slice Acceptance and Regression Report

**Status:** Complete — All synthetic conformance and regression tests passed. Full live environment gates remain **Not Run**.  
**Date:** 24 November 2026.  
**Authority:** Blueprint v1.14.35 (§6, §7), P4 Contract Package 0.3.3 (Draft), and P5 Task Plans (M0–M5).  
**Environment:** Linux 6.18.2-arch2-1 x86_64; rustc 1.89.0 (pinned); rusqlite 0.41.1; SQLite 3.54.1; Godot 4.x headless profile.

---

## 1. Executive Summary

Phase 5 (First Vertical Slice) proves one complete embodied interaction loop across presentation, conversation, fresh desktop context, deterministic authorization, one same-display window move, truthful outcome reporting, and one inspectable/deletable low-sensitivity memory.

All planned milestones (M0 through M5) have been implemented, tested, and merged into `main` across PRs #3, #5, #6, #7, #8, and #9. The full workspace test suite (108 tests) passes with zero failures, cargo formatting and clippy pass with zero warnings, contract validation checks pass cleanly, and the blueprint integrity check verifies all 696 headings intact.

In accordance with Architectural Invariant 10 and Blueprint §10, all live environment conformance gates (`ADAPTER-01`, `HYPR-01`, `HYPR-02`, `GODOT-01`, `MODEL-01`, `SPEECH-01`, `MEM-01`, `IPC-01`) remain status **Not Run** until live hardware execution on a physical Linux Hyprland workstation.

---

## 2. Acceptance Conditions Traceability Matrix (AC-001–AC-011)

| Acceptance Condition | Scope & Realization | First Proving Milestone | Verification Evidence |
|---|---|---|---|
| **AC-001** (Lifecycle Truth & Presentation Restoration) | LIA starts, renders character, restores presentation state, and visibly exposes authoritative agent states (`Idle`, `Listening`, `Speaking`, `Paused`, `Disconnected`). Paused immediately overrides speaking. | M4 (Godot presentation) | `presentation_client_tests::test_godot_003_character_lifecycle_transitions` |
| **AC-002** (Interaction & Capture Lifecycle) | Push-to-talk voice token input, text turn submission, speech duration pacing, cooperative interruption (`chat.stop`), and pause/stop controls. No capturing state is depicted when inactive. | M5 (Conversation loop) | `conversation::tests::test_m5_002_synthetic_speech_transcription_and_tts_duration`, `test_m5_004_turn_interruption_chat_stop_ac002` |
| **AC-003** (Active Window Discovery without Screen Capture) | Discovers focused floating window, coordinates, and monitor/workspace binding with compile-time window title redaction and 10s freshness ceiling. No raw pixel or screen-content capture. | M3 (Hyprland adapter) | `hyprland::tests::inspect_active_window_redacts_title_and_enforces_freshness`, `adp_tests::adp_023_privacy_guarantee_zero_titles_in_evidence_or_manifests` |
| **AC-004** (Ambiguity, Stale Targets, and Unsupported Destinations) | Rejects expired window tokens (>10s), display monitor mismatches, and coordinates exceeding reserved panel work-area margins before effect. Employs clarification/refusal rather than guessing. | M3 (Hyprland adapter) | `hyprland::adp_tests::adp_008_stale_resource_fails_before_effect`, `adp_010_out_of_bounds_target_arguments_reject_before_effect` |
| **AC-005** (Authorization Check & No Standing Grant) | Evaluates deterministic broker capability allowlist and permission policy (R3 tier check) per operation. Consumes signed command nonce and session sequence (`sequence 0` initial use) before effect. No standing grants. | M1 (Core) + M3 (Adapter) + M5 (Conversation) | `store::tests::lease_claim_binding_and_consume_before_effect`, `conversation::tests::test_m5_005_model_tool_proposal_untrusted_and_broker_permission_denial` |
| **AC-006** (Same-Display Window Move with Read-Back Verification) | Dispatches structured move command preserving window dimensions, workspace, and monitor. Post-operation independent read-back query verifies coordinates before reporting success. Transport ACK alone is never treated as execution. | M3 (Hyprland adapter) | `hyprland::adp_tests::adp_018_dispatch_acknowledgement_alone_is_not_verified_success`, `conversation::tests::test_m5_008_end_to_end_vertical_slice_turn_to_verified_window_move_ac001_to_ac011` |
| **AC-007** (Truthful Presentation Synchronization) | Presentation character visual responses distinguish 5 canonical outcome classes: `VerifiedSuccess`, `VerifiedFailure`, `UnknownOutcome`, `Denied`, and `Unsupported`. Animation halts or cautions on failure/uncertainty. | M4 (Godot presentation) | `presentation_client_tests::test_godot_004_truthful_outcome_visual_distinction_ac007` |
| **AC-008** (Non-Success Outcomes & No Force Close) | Reconciles dispatch timeouts and post-condition mismatches as `UnknownOutcome` (requiring reconciliation) or `VerifiedFailure`. Never force-closes windows or discards unsaved user state. | M1 (Core) + M3 (Adapter) | `hyprland::adp_tests::adp_014_timeout_after_dispatch_requires_reconciliation`, `hyprland::tests::execute_move_dispatch_timeout_yields_unknown_outcome` |
| **AC-009** (Low-Sensitivity Preference Memory & Tombstones) | Single allowed low-sensitivity (`S1`) preference persists across restarts with mandatory user turn provenance (`source_reference_id`), monotonic revisioning, inspection, and tombstone deletion. | M1 (Core) + M5 (Memory) | `store::tests::test_user_preference_provenance_and_tombstone_deletion`, `conversation::tests::test_m5_007_persistent_memory_preference_provenance_and_deletion_ac009` |
| **AC-010** (Canonical User Controls & Placement Precedence) | Presentation user controls (`panel.hide`, `panel.pin`, `panel.lock`, `panel.reposition`, `presentation.pause`, `presentation.resume`, `chat.input`, `chat.stop`). User lock overrides autonomous task-bound positioning. | M2 (IPC) + M4 (Godot presentation) | `presentation_client_tests::test_godot_005_ac010_user_controls_typed_intents_and_bounds`, `test_godot_006_placement_precedence_user_lock` |
| **AC-011** (Declared Capability Envelope & Honest Fallback) | Adapter generates strict capability manifest declaring supported operations (`desktop.window.move_same_display`). Unsupported capabilities reject closed before execution with typed failure reasons. Zero remote network egress. | M3 (Adapter) + M5 (Conversation) | `hyprland::adp_tests::adp_005_unsupported_capability_fails_closed_without_shell_fallback`, `conversation::tests::test_m5_006_undeclared_tool_proposal_fails_closed` |

---

## 3. Milestone Delivery Log

| Milestone | PR / Commit | Key Deliverables |
|---|---|---|
| **M0 — Baseline & Tooling** | PR #3 (`f892bc0`) | Toolchain pinning, CI contract-hygiene workflow, OpenAPI 3.1 bundle repair, PostgreSQL/SQLite migration harnesses, and fixture validators. |
| **M1 — Device Agent Core** | PR #5 (`696dfcf`) | SQLite migrations 001–008, atomic consume-before-effect permits, sequence zero initial use, WAL mode, integrity anchor, and tamper-resistant outbox/evidence ledger. |
| **M2 — Presentation IPC** | PR #6 (`a07fe10`) | Private Unix domain socket endpoint, Linux peer credentials (`SO_PEERCRED`), hello/ready handshake, session UUIDs, strict message-body schemas, bounded user-event queue, supervisor graceful lifecycle, and `ProductionUiRegistry` mapping AC-010 controls. |
| **M3 — Hyprland Adapter** | PR #7 (`193d25e`) | Hyprland v0.57.2 adapter core, active window discovery with compile-time title redaction, 10s reference freshness bounds, work-area reserved margin parsing, structured dispatch filtering, independent read-back verification, and synthetic ADP conformance suite. |
| **M4 — Godot Presentation** | PR #8 (`f940bfe`) | Godot 4.x presentation application (`apps/lia-presentation-godot/`), length-prefixed Unix stream client, strict snapshot schema parser, character lifecycle state machine (AC-001), 5-class truthful outcome visualization (AC-007, Invariant 8), placement precedence lock, and session replay protection. |
| **M5 — Conversation & Memory** | PR #9 (`98438de`) | SQLite migration 009 (`local_user_preference`), `AgentStore` preference operations, JCS SHA-256 context packaging with 10s TTL bound, synthetic model/speech providers with prompt-injection refusal (Invariant 1), `PreferenceMemoryManager` (AC-009), and `ConversationCoordinator` end-to-end slice integration (AC-001–AC-011). |

---

## 4. Retained Evidence Directory

All evidence is retained in repository history with synthetic identifiers, redacted sensitive fields, and cryptographic SHA-256 checksums:

- `evidence/M0/`: Initial contract hygiene baseline.
- `evidence/SQLITE-02/`: SQLite WAL, migrations 001–008, replay prevention, and crash recovery.
- `evidence/SECURITY-01-local/`: RFC 8032 Ed25519 signatures, JCS canonicalization, domain separation.
- `evidence/IPC-01/`: Unix socket framing, peer credentials, session lifecycle, queue backpressure, and fuzzing.
- `evidence/HYPR-01/`: Hyprland active window discovery, title redaction, and ADP conformance suite.
- `evidence/GODOT-01/`: GDScript presentation client architecture, snapshots, lifecycle, and AC-010 controls.
- `evidence/MODEL-01/`: Synthetic model determinism, injection refusal, and untrusted tool proposals.
- `evidence/SPEECH-01/`: Synthetic speech push-to-talk transcription and TTS duration bounds.
- `evidence/MEM-01/`: Low-sensitivity preference memory, provenance, tombstone deletion, and privacy checks.
- `evidence/repository-integrity/`: Blueprint integrity validation and heading protection.

---

## 5. Architectural Invariants Validation

1. **Model Proposal Invariant (Invariant 1):** Validated in `conversation/providers.rs` and `coordinator.rs`. Model outputs are strictly untrusted `UntrustedToolProposal` structures; they are never granted direct OS execution authority.
2. **Deterministic Authority (Invariant 2):** Validated in `coordinator.rs` and `presentation_policy.rs`. The Rust device agent owns OS execution; deterministic broker and permission authorities authorize operations before execution.
3. **Godot Presentation Only:** Validated in `apps/lia-presentation-godot/`. Godot emits user event intents over IPC; it has no OS execution privileges, file access, or shell invocation paths.
4. **Truthful Outcome Tri-State (Invariant 7):** Validated in `hyprland/types.rs` and `coordinator.rs`. Operations strictly distinguish `VerifiedSuccess`, `VerifiedFailure`, and `UnknownOutcome`.
5. **Acknowledgement != Execution (Invariant 8):** Validated in `hyprland/operations.rs`. Transport dispatch acknowledgement ("ok") never triggers success; independent post-operation read-back is strictly mandatory.
6. **Zero Network Egress (ADR-143):** Validated in `conversation/context.rs` and `providers.rs`. Provider invocations operate strictly local-only with synthetic implementations.
7. **Privacy Invariant:** Validated across tests (`adp_023`, `test_m5_008`). Active window titles and user text are compile-time redacted; zero window titles leak into the SQLite store or outbox.
8. **Consume-Before-Effect:** Validated in `AgentStore::consume_command_before_effect`. Commands consume nonces and advance monotonic session sequences before any effector dispatch.

---

## 6. Handoff to Phase 6 (Companion Integration)

With Phase 5 (First Vertical Slice) complete on the local Linux profile:
- Deferred coordinator gates (`TEMP-01`, `TEMP-02`, `NATS-01`, `DB-01`, `RPC-01`, `RPC-02`, `OPS-01`, `OPS-02`) transition to Phase 6.
- Mobile companion client gates (`MOBILE-01`) and Home Assistant gateway gates (`HOME-01`) transition to Phase 6.
- Live hardware evaluation for P5 gates (`HYPR-01/02`, `GODOT-01`, `MODEL-01`, `SPEECH-01`, `MEM-01`) will be executed upon physical workstation deployment.
