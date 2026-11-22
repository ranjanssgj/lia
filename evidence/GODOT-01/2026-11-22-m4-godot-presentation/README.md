# GODOT-01 — Godot presentation architecture and synthetic client conformance evidence

**Gate:** GODOT-01 / IPC-01 integration (partial implementation checks).  
**Gate Status:** **Not Run**. This run validates local GDScript client architecture, strict snapshot decoding, character lifecycle states, truthful outcome visualization (Invariant 8 & AC-007), placement precedence, and canonical AC-010 typed user control intent generation against synthetic agent fixtures; full conformance gates remain **Not Run** until live Godot 4.x runtime execution on the pinned Linux profile.  
**Date:** 22 November 2026.  
**Procedure Revision:** P5 M4 task plan (`docs/p5/tasks/M4-godot-presentation.md`).  
**Environment:** Linux 6.18.2-arch2-1 x86_64; rustc 1.89.0 (pinned); rusqlite 0.41.1; SQLite 3.54.1.

## Scope and Invariants Validated
1. **Zero OS Execution Authority & Proposal Invariant:** The presentation client acts solely as an untrusted presentation surface. It has no authority to execute OS commands, mint grants, access raw screen/audio capture, or claim operation outcomes. User interactions emit typed `presentation.user_event` intents with bounded fields.
2. **Strict Snapshot Schema & Validation:** `PresentationSnapshot` enforces contract schemas for `presentation.state.replace`, requiring strictly allowlisted fields (`state_revision`, `character_id`, `emotion`, `pose_id`, `speaking`, `listening`, `paused`), finite intensity in `0.0..=1.0`, safe integer bounds, and valid ASCII identifiers. Unknown properties, missing required fields, and out-of-bounds intensity fail closed.
3. **Monotonic State Revision Invariant:** The client retains the last valid revision only within the current live session. Incoming snapshots with `state_revision <= last_revision` are rejected as stale.
4. **Character Lifecycle State Machine (AC-001):** Derived presentation states (`Idle`, `Listening`, `Speaking`, `Paused`, `Disconnected`) transition strictly according to authoritative agent snapshot flags. `paused == true` takes immediate precedence over active speech.
5. **Truthful Outcome Visualization (Invariant 8 & AC-007):** The client distinguishes all five canonical outcome classes:
   - `VerifiedSuccess`: affirmative visual indicator;
   - `VerifiedFailure`: explicit non-success indicator;
   - `UnknownOutcome`: visible caution indicator (strictly non-success);
   - `Denied`: polite refusal cue;
   - `Unsupported`: honest incapability indicator.
   Transport acknowledgement ("ok") or socket delivery alone never triggers a success animation.
6. **Canonical AC-010 User Controls:** All 8 approved controls (`panel.hide`, `panel.pin`, `panel.lock`, `panel.reposition`, `presentation.pause`, `presentation.resume`, `chat.input`, `chat.stop`) are mapped to typed intents. Text submission is bounded to 8,192 UTF-8 bytes; empty text submissions are rejected.
7. **Placement Precedence (AC-010):** When `user_locked` is enabled, user placement takes strict precedence over autonomous task-bound window movements.
8. **Session Replay Protection:** Reconnection establishes a fresh session UUID and resets sequence counters. Captured intents from previous sessions are rejected with `PolicyDenied` or `InvalidSession`.

## Raw Output and Checksums
- Raw test run: [`raw.txt`](raw.txt)
- Checksum file: [`raw.sha256`](raw.sha256)
- Checksum: `c206f67852ebfadaddbe11130827f939026a5d4a328cb6feaa0d7c6efd2207d4`

## Results
- 7 synthetic presentation client conformance tests passed, 0 failed.
- Workspace test suite: 98 passed, 0 failed, 1 ignored.
