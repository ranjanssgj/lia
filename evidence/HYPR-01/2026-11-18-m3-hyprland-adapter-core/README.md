# HYPR-01 / HYPR-02 — Hyprland v0.57.2 adapter core implementation evidence

**Gate:** HYPR-01 / HYPR-02 / ADAPTER-01 (partial implementation checks).  
**Gate Status:** **Not Run**. This run validates local unit implementations, command filtering, title redaction, bounds enforcement, consume-before-effect store integration, and read-back verification against synthetic mock compositor responses; full conformance gates remain **Not Run** until live compositor execution on the pinned Linux profile.  
**Date:** 18 November 2026.  
**Procedure Revision:** P5 M3 task plan (commit `a07fe10d788e71b8f2147980ef3d4d5550f3d16b`).  
**Environment:** Linux 6.18.2-arch2-1 x86_64; rustc 1.89.0 (pinned); rusqlite 0.41.1; SQLite 3.54.1.

## Scope and Invariants Validated
1. **Title Redaction & Privacy Invariant:** `inspect_active_window` and `parse_event_line` explicitly drop `title` and `initialTitle` from compositor queries and event streams, serializing only opaque hex addresses and application class tokens (`title_redacted: true`). ADP-023 synthetic test confirms zero window titles written to SQLite store (`local_operation_evidence` and `local_outbox`).
2. **Ephemeral Reference Freshness:** Window identity references are bound to a strict 10-second freshness window (`fresh_until_us = observed_at_us + 10_000_000`); expired references fail closed before dispatch (ADP-008, ADP-009).
3. **Structured Command Allowlist:** Disallowed queries (`dispatch killactive`, `sh -c echo`) and malformed hex addresses (`not_hex`, `0x123;rm -rf /`) are rejected before transport execution; no arbitrary shell execution permitted (ADP-005).
4. **Usable Work Area Margins:** Reserved panel margins from `j/monitors` (`[top, bottom, left, right]`) are correctly subtracted from display dimensions; out-of-bounds destinations fail closed before dispatch (ADP-010).
5. **Consume-Before-Effect Invariant:** Operations require an authoritative [`EffectPermit`] from [`crate::AgentStore`] issued via `consume_command_before_effect` in an immediate SQLite transaction before any dispatch request.
6. **Independent Read-Back Verification (Invariant 8):** Dispatch acknowledgement ("ok") is never accepted as proof of execution (ADP-018); an independent query to `j/activewindow` verifies that coordinates match target, dimensions are preserved, and workspace/monitor remain identical. Contradictory read-back yields [`MoveOperationResult::VerifiedFailure`].
7. **Uncertainty & Reconciliation (Invariant 7):** Dispatch I/O errors or timeouts after potential send atomically transition the attempt to [`MoveOperationResult::UnknownOutcome`] with `reconciliation_required: true` (ADP-014).
8. **Capability Manifest Generation:** `generate_hyprland_manifest` produces contract-v1 compliant manifests declaring `desktop.window.observe_geometry` and `desktop.window.move_same_display` with `local_only` data governance and `raw_capture: false` (ADP-001).
9. **Event Socket Invalidation Hints:** `HyprlandCacheInvalidator` increments epoch upon receiving focus, close, move, monitor, or workspace change events from `.socket2.sock`, ensuring stale observations trigger rediscovery.
10. **Presentation Authorization Boundary:** The presentation client (Godot) only submits typed proposals and has zero OS execution authority (ADP-024).

## Raw Output and Checksums
- Raw test run: [`raw.txt`](raw.txt)
- Checksum file: [`raw.sha256`](raw.sha256)
- Checksum: `2314f384b25c32bdad691f6a27ec6d16a293c447aa2e946866602d856b3055ac`

## Results
- 22 tests passed, 0 failed.
- Workspace test suite: 91 passed, 0 failed, 1 ignored.
