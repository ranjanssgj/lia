# M4 task plan — Godot presentation

**Status:** Planned; implementation not started. **Gate status:** GODOT-01 and IPC-01 integration remain Not Run.

**Acceptance conditions:** AC-001, AC-007 and AC-010. The task implements a presentation surface only; it does not execute desktop operations or establish an end-to-end acceptance claim.

## Contract inputs and dependencies

- P4 package revision 0.3.1 Draft: [presentation IPC](../../p4/ipc/presentation-agent-v1.md), [P4 conformance matrix](../../p4/conformance/p4-conformance-matrix.md), and the relevant Blueprint lifecycle and presentation requirements.
- M2 must define versioned snapshot/state bodies and control mappings, bind the expected renderer process, and enforce authorized event handling before any state-changing control is connected. Until then, develop only against a local mock Unix socket and synthetic fixtures. Do not enable action dispatch.
- M3 adapter behavior is represented through the mock boundary until the owner supplies the exact Hyprland v0.57.2 build and verifiable provenance. Do not use a live compositor as an integration dependency for this task.
- Godot engine, export-template, plugin and asset versions/hashes must be selected or confirmed by the owner and pinned before implementation. The P4 Godot 4.7.2 reference is a planning input, not evidence that it is installed or approved for the build.

## In-scope behavior

1. Render character lifecycle and approved presentation state from versioned agent snapshots. The renderer treats malformed, stale or unknown state as unavailable/neutral and never invents operation outcomes.
2. Bind animation and visual transitions to explicit agent-reported states: success, denied, failed, unsupported and unknown. A request acknowledgement or optimistic animation must not imply a completed effect.
3. Provide only the approved user controls: hide/reposition, pin/lock, pause/resume and stop/interruption as defined by the reviewed P4 and Blueprint precedence rules. UI actions emit typed intents with bounded fields.
4. Recover cleanly from socket loss, agent restart and renderer restart. Reconnect creates a new session and waits for the fresh snapshot; prior user actions are never replayed.
5. Keep presentation state and interaction local. Do not read arbitrary files, run commands, call providers, capture screen/audio, or hold credentials.

## Test and evidence plan

| Area | Required cases | Evidence |
|---|---|---|
| IPC contract | Mock hello/ready, strict snapshot decoding, unknown fields, stale revision/session and malformed frames | Synthetic fixture hashes, protocol traces and schema/parser results |
| Lifecycle | Startup, hide/show, pin/lock, pause/stop, socket loss, agent restart and renderer restart | Redacted state-transition traces; no screenshot or raw user content |
| Truthful animation | Verified success, denied, failed, unsupported and unknown outcomes | State-to-animation matrix and traces proving no optimistic success |
| Input controls | Valid typed intents, unauthorized/unknown controls, duplicate/replayed events, queue backpressure | Mock event fixtures, rejected-event results and no-dispatch assertions |
| Resources and privacy | Idle/interactive frame budget, memory/CPU, DPI/scaling, focus/input behavior and log redaction | Exact runtime/build hashes, redacted resource report, privacy inspection |

Use synthetic data and the mock socket until M2 schemas and policy binding are reviewed. Keep screenshots, audio, raw window titles, credentials and private user content out of evidence. Retain command, commit, OS/runtime versions, contract revision, fixture/build hashes, raw redacted output, failures, limitations and reviewer under `evidence/GODOT-01/<run-id>/` and the IPC-01 integration evidence directory.

## Exit criteria

- The pinned Godot runtime and export templates are reproducible and their hashes are recorded.
- The presentation renders agent-owned state and reconnects without stale or replayed intents.
- All five outcome classes produce truthful, distinguishable states; unknown remains visibly non-success.
- User controls match the approved precedence and send only typed, bounded intents.
- The focused frame/resource/privacy procedures have reproducible evidence and review.
- Update GODOT-01 and AC status only after the full procedure and review. Until then, all gates and acceptance conditions remain Not Run/not claimed Passed.

## Open decisions

The exact Godot runtime/export-template/plugin set and asset rights review remain open. Do not select them by inference. No character asset, plugin, voice, font or other licensed content is approved by this plan.
