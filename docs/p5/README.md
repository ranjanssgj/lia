# P5 — First vertical slice plan

**Status:** M0 readiness complete; M1 core merged with follow-up evidence open; M2 IPC spike in progress.
**Scope:** The approved first slice in Blueprint §6, on one Linux host, one supported display and one exact Hyprland profile.
**Authority:** Blueprint v1.14.24 and P4 contract package 0.3.2 (Draft) in this branch.

P5 proves one complete interaction path across presentation, conversation, fresh desktop context, current authorization, one same-display window move, truthful outcome reporting and one inspectable/deletable low-sensitivity memory. It does not establish general Linux, Windows, mobile, Home Assistant, cloud, or production-provider support.

## Readiness rule

The P4 artifacts are drafts. Coding may begin only against a named contract revision and after M0 has established a reproducible environment, fixed the OpenAPI parse defect, pinned the tools actually needed for that milestone, and retained baseline evidence. A passing structural check does not mark a conformance gate Passed. Do not start a milestone whose required owner decision is unresolved.

## Milestones and traceability

| Milestone | Outcome and likely paths | Acceptance conditions | Required gates / evidence | Depends on |
|---|---|---|---|---|
| **M0 — Agent-ready baseline** | `AGENTS.md`, root `README.md`, `.gitignore`, `CODEOWNERS`, `docs/p5/README.md`, `toolchain/`, `.github/workflows/`, `scripts/`, `testdata/`, `evidence/`; repair `docs/p4/openapi/lia-v1.yaml` | Enables agents to build from a clean checkout and know what is safe to change; no AC is claimed satisfied | CI applies Protobuf 36.2 compile, OpenAPI 3.1 bundle/parse, PostgreSQL 16.16 migration chain, SQLite 3.54.1 migration chain, JSON Schema/fixture validation and strict TypeScript check. Record tool versions, hashes and reports. Keep conformance gate statuses Not Run until their full implementation procedures are fulfilled | Selected Hyprland v0.57.2 stable for M3; migrate to Lua configuration and verify package/build provenance before the adapter proof |
| **M1 — Device-agent core** | `crates/` workspace, SQLite migrations 001–008, outcome/evidence/outbox model, lease/nonce/session-sequence ledgers, restore/reconciliation boundaries, security and digest vectors in `testdata/` | Foundation for AC-005, AC-006, AC-008, AC-009, AC-011; no end-user claim until later milestones integrate it | SQLITE-02 plus local SECURITY-01 vector subset; clean DB, upgrade, crash/restore, replay and consume-before-effect evidence | M0; Rust 1.99.1, rusqlite 0.41.1 with bundled SQLite 3.54.0, exact crypto pins and checked-in vector format |
| **M2 — Presentation IPC spike** | Threat map drafted; M2 spike implements framed JSON validation, private Unix socket endpoint, Linux peer credentials, hello/ready-gated connection state, session UUIDs, per-session user-event sequence checks and an absolute monotonic handshake deadline; socket service lifecycle/backpressure remain | Supports truthful lifecycle/capability state and user control in AC-001 and AC-010 | IPC-01 Linux subset: peer UID/PID, malformed/oversized/truncated frames, disconnect/restart, stale messages, backpressure and renderer restart evidence | M0; Ubuntu 24.04 CI is the development baseline only (not desktop support); see [M2 threat map](tasks/M2-threat-model-test-map.md) |
| **M3 — Desktop context and Hyprland adapter** | Active-window/geometry observation, capability manifest, same-display move, read-back verification and ADP harness | AC-003–AC-006, AC-008 and AC-011 | ADAPTER-01 shared cases applicable to this profile; HYPR-01, HYPR-02; Hyprland v0.57.2 build/package provenance, Lua config, fixture hashes, operation/attempt evidence, permission-denial and stale-resource results | M0, M1, M2; exact P5 reference selected as Hyprland v0.57.2 stable; migrate compositor profile to Lua and specify request/event use, work-area bounds and display checks |
| **M4 — Godot presentation** | Character lifecycle, state display, user placement/pin/lock controls, truthful animation tied to operation results; see [M4 task plan](tasks/M4-godot-presentation.md) | AC-001, AC-007, AC-010 | GODOT-01 and IPC-01 integration evidence; animation-state traces for success, denied, failed, unsupported and unknown outcomes | M2 and M3; Godot engine/export templates/plugin hashes pinned |
| **M5 — Conversation and memory loop** | Text and push-to-talk interaction; synthetic provider first; later owner-approved provider trials; persisted preference with provenance, inspect/delete; see [M5 task plan](tasks/M5-conversation-memory.md) | AC-002 and AC-009; end-to-end AC-001–AC-011 regression | MODEL-01, SPEECH-01 and MEM-01 as applicable; provider locality/retention/cancellation, interruption, restart, provenance, inspection and deletion evidence; full slice acceptance run | M1–M4; owner decisions on ASR/TTS/LLM candidates and locality rules; memory-local schema and retention policy |

M0 CI shall first report clearly scoped contract hygiene results. A passing CI job is not a P4 conformance pass; implementation gates remain Not Run until their full procedures and evidence are complete.

## Owner decisions and readiness record (15 November 2026)

PR #3 merged M0 on 15 November 2026. Its contract-hygiene workflow passed on the PR head; see [run 43625530632](https://github.com/ranjanssgj/lia/actions/runs/43625530632). GitGuardian also passed. These checks establish contract hygiene only. P4/P5 implementation gates remain Not Run, and no acceptance condition is claimed satisfied.

The owner approved this P5-local dependency profile: SQLITE-02, local SECURITY-01, Linux IPC-01, ADAPTER-01, HYPR-01/02 and GODOT-01. TEMP-01/02, NATS-01, DB-01, RPC-02 and OPS-01/02 are deferred to P6's coordinator profile, not considered passed or removed. P5 does not need Temporal, PostgreSQL coordinator services, NATS or hosted inference.

The owner deferred LIA's project license decision. No license is selected or implied, and this plan does not grant redistribution rights. The [dependency/model-weight license register](../../toolchain/LICENSE-REGISTER.md) records locked validation dependencies, including the M1 Rust lockfile, and open artifact reviews. The adopted [provider evaluation policy](provider-evaluation.md) sets egress boundaries without selecting any model/provider.

M1 core implementation is merged through PR #5; M1 follow-up tests and evidence are continuing alongside M2 on the separate IPC branch. Its planned scope and evidence boundaries are recorded in [the task plan](tasks/M1-device-agent-core.md). PR #5 CI passed the Rust and contract-hygiene jobs. M2 is tracked in draft PR #6. The local IPC primitive now tests a supervised child PID binding with bounded restart/backoff, a multi-connection accept loop, state snapshot delivery/coalescing, bounded user-event queue backpressure, and separate-UID peer rejection at the authenticated accept boundary. Godot process integration, policy authorization, probabilistic race/stress and service resource high-water bounds, coverage-guided fuzzing and full IPC-01 remain open. The [17 November M2 adversarial checks](../../evidence/IPC-01/2026-11-17-m2-adversarial-checks/README.md) record the local partial procedures. The [M3 Hyprland task plan](tasks/M3-hyprland-adapter.md) defines the v0.57.2/Lua profile before adapter implementation. No conformance gate or acceptance condition is marked Passed.

M2 follow-up on 17 November adds strict versioned hello, ready and state-replacement body schemas to P4 package 0.3.2 and Rust IPC validation; unknown fields, invalid state ranges and renderer-authored outcome claims fail closed. Workspace tests and Clippy pass in the retained [schema-check evidence](../../evidence/IPC-01/2026-11-17-m2-body-schemas/README.md). A separate-UID child now reaches the authenticated accept boundary over a disposable test socket and is rejected by `SO_PEERCRED`; see [peer credential evidence](../../evidence/IPC-01/2026-11-17-m2-peer-credentials/README.md). The milestone remains in progress: real Godot lifecycle binding, authorization boundary review, sustained resource/backpressure evidence, coverage-guided fuzzing and complete IPC-01 remain open. M3 has not started because its M2 dependencies are not satisfied.

### Open owner decisions

| Item | Status |
|---|---|
| P5-local gate profile | Approved 15 November 2026; recorded above |
| LIA project license | Deferred by owner; no LICENSE file selected |
| Exact Hyprland build/profile | Selected by user-authorized delegation on 15 November 2026: stable v0.57.2; Lua config migration, exact package provenance and M3 proof remain open |
| Hardware tier and GPU availability | Open; needed before local LLM evaluation |
| Languages | Open; “English first, Hindi stretch” remains a proposal |
| Hosted evaluation budget and time box | Open; $20/month and two weeks remain proposals |
| Coordinator Node baseline | P6 decision; Node 24 remains the blueprint reference |

## Acceptance-condition coverage

| AC | First milestone proving it | Contract / proof focus |
|---|---|---|
| AC-001 | M4; end-to-end in M5 | lifecycle truth, presentation restoration, IPC-01, GODOT-01 |
| AC-002 | M5 | push-to-talk/text, speech, interruption, pause/end, truthful capture state; MODEL-01/SPEECH-01 |
| AC-003 | M3 | active window and supported geometry without screen-content capture; HYPR-01 |
| AC-004 | M3 | ambiguity, stale targets and unsupported destination handling; ADAPTER-01 |
| AC-005 | M1 + M3 | current authorization and no standing grant inference; SECURITY-01/ADAPTER-01 |
| AC-006 | M3 | same-display move, dimensions/workspace preserved, verified read-back; HYPR-02 |
| AC-007 | M4 | animation follows actual result and stops/changes on failure or uncertainty; GODOT-01 |
| AC-008 | M1 + M3 | no force-close; explicit non-success on stale, unsupported or failed operation; SQLITE-02/ADAPTER-01 |
| AC-009 | M1 + M5 | one permitted preference persists with provenance and supports inspection/deletion; MEM-01 |
| AC-010 | M2 + M4 | hide/reposition/pin/lock/pause/stop and placement precedence; IPC-01/GODOT-01 |
| AC-011 | M3 + M5 | declared capability envelope and honest unsupported response; ADAPTER-01/HYPR-01 |

The matrix maps the first milestone expected to prove an AC; it does not claim that any AC has passed.

## Task completion and evidence

Every task names its ACs, exact P4 contract files/revisions, gate, fixture set and expected artifacts before implementation. A task is complete when its code/documentation is reviewed, the named gate procedure has run on the pinned environment, raw redacted output and environment details are retained under `evidence/<gate>/<run-id>/`, and the relevant status record is updated. Store command, commit, OS/runtime/tool versions, contract and fixture hashes, result, failures and reviewer. Never commit credentials, private content or raw desktop/audio capture.

For CI-hosted evidence, retain immutable run URLs and downloadable reports or checksums under the evidence index. A green linter, schema parse, compile, or provider statement is not a Passed conformance gate.

## Pins and decisions

The readiness research is dated 2026-11-14. Re-verify all version facts from primary release sources immediately before creating lockfiles or toolchain files. Pin a compatible set per milestone, including code generators and runtime libraries; record exact versions and artifact hashes. Apply a documented dependency cool-down for newly published versions. Do not copy candidate versions from the research note without rechecking.

Hyprland v0.57.2 is selected for the P5 M3 reference profile; exact package/build provenance and HYPR-01/02 evidence remain open. Owner decisions remain open: hardware/GPU tier; languages; hosted evaluation budget and time box; project license; and any change to the coordinator’s Node baseline. ASR/TTS/LLM candidates remain unselected. The P5-local gate profile was approved by the owner on 15 November 2026. The approved slice is local-first; PostgreSQL, Temporal, NATS and hosted inference are not required for baseline desktop operation. Do not silently add them to the slice.

## P4/P6 status boundary

P4 gate status remains as recorded in [`docs/p4/conformance/p4-conformance-matrix.md`](../p4/conformance/p4-conformance-matrix.md), all Not Run. The owner-approved P5-local dependency profile is SQLITE-02, local SECURITY-01, Linux IPC-01, ADAPTER-01, HYPR-01/02 and GODOT-01; coordinator gates TEMP-01/02, NATS-01, DB-01, RPC-02 and OPS-01/02 are deferred to P6. P5 Linux proof does not claim Windows parity. `MOBILE-01` and `HOME-01` remain P6. P5 may use the contracts as drafts while they are verified incrementally; update versioned contracts when implementation evidence exposes a defect, preserving prior revisions where required.
