# Production presentation authorization and supervisor lifecycle checks (partial evidence)

- Gate: IPC-01; status remains **Not Run**.
- Base commit: `67596034ecc09e02ab5d3505219a91e546ea6ead`.
- Contract: P4 IPC 0.3.3 (Draft), §1 responsibility boundary, §4.1 user-event schema, §6 lifecycle, §8 IPC-01.
- Implemented:
  1. Production UI Action Registry (`ProductionUiRegistry`): Allowlisted registry of canonical AC-010 controls (`panel.hide`, `panel.pin`, `panel.lock`, `panel.reposition`, `presentation.pause`, `presentation.resume`, `chat.input`, `chat.stop`) declaring risk tiers (R0, R1), allowed event kinds, session requirements, and text payload limits.
  2. Presentation Policy & Permission Authority integration: `DeterministicPresentationPolicy` evaluating deterministic permission grants against declared risk tiers and active session binding.
  3. Presentation intent invariants: `AuthorizedPresentationIntent` represents validated presentation intents without granting OS execution authority or carrying forged operation outcomes.
  4. Renderer Supervisor lifecycle: Added `terminate(grace_period)` sending `SIGTERM` before falling back to `SIGKILL` on timeout; inspectability via `is_alive()` and `restart_count()`; clean process termination on Drop without leaving orphan or zombie processes.
  5. Resource bounds & concurrency: Isolated churn test verifying zero file descriptor leaks across 32 rapid connection/handshake/disconnect cycles; slow reader timeout bounding under 1.0s I/O limit.
  6. Adversarial parser edge cases: Deep nesting rejection, control ID validation (length and character boundaries), and snapshot intensity range enforcement.
- Results: Full workspace test suite passed (69 passed, 1 ignored for user-namespace mapping requiring unshare, 0 failed).
- Limitations: Conformance gate IPC-01 remains Not Run pending full multi-process Godot desktop integration and end-to-end P5 slice verification.
- Output and SHA-256 checksum: [raw.txt](raw.txt), checksum [raw.sha256](raw.sha256).
