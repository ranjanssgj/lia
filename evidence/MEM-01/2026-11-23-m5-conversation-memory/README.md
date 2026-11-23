# MEM-01 — Low-Sensitivity Preference Memory, Provenance Tracking, and Tombstone Deletion

**Gate:** MEM-01 (partial implementation checks).  
**Gate Status:** **Not Run**. This run validates local SQLite preference persistence (AC-009), strict provenance binding to originating user turns, optimistic monotonic revisions, immediate tombstone deletion records, and end-to-end privacy guarantees (zero window title leaks into outbox / store). Full conformance gate remains **Not Run** until live coordinator environment execution.  
**Date:** 23 November 2026.  
**Procedure Revision:** P5 M5 task plan (`docs/p5/tasks/M5-conversation-memory.md`).  
**Environment:** Linux 6.18.2-arch2-1 x86_64; rustc 1.89.0 (pinned); rusqlite 0.41.1; SQLite 3.54.1.

## Scope and Invariants Validated
1. **Low-Sensitivity Preference Storage (AC-009):** The agent store permits saving only S0 and S1 preferences (e.g. `theme`, `window_default_position`). High-sensitivity preferences fail closed.
2. **Mandatory Provenance Tracking:** Saving a preference requires an explicit `source_reference_id` (turn ID or interaction event); unprovenanced preference writes fail closed with `MissingSourceProvenance`.
3. **Monotonic Optimistic Revision:** Updates advance the preference revision monotonically (`1 -> 2`), recording exact microsecond timestamps and updating the provenance link.
4. **Tombstone Deletion (AC-009):** Deleting a user preference marks the record as deleted and atomically records a deletion tombstone in `local_deletion_tombstone`. Subsequent inspections immediately return `None`.
5. **End-to-End Slice Verification (AC-001–AC-011):** Verifies the full integration path:
   - Presentation text input (`"Move active window to left"`);
   - Context packaging with TTL and canonical SHA-256 digest;
   - Untrusted tool proposal emitted by synthetic model;
   - Deterministic broker capability and permission evaluation (R3 tier check);
   - Compositor active window inspection and display work-area margin computation;
   - Consume-before-effect sequence verification and `EffectPermit` generation;
   - Compositor dispatch and independent post-operation read-back verification (`VerifiedSuccess`);
   - User preference persistence with provenance;
   - SQLite privacy invariant verification: zero raw window titles leak into outbox or persistent stores.

## Raw Output and Checksums
- Raw test run: [`raw.txt`](raw.txt)
- Checksum file: [`raw.sha256`](raw.sha256)
- Checksum: `d5905f11c71e7eb4e9ce5403d56fc1d7b3427a6b7256a7903aaceec865d26ca1`

## Results
- 8 conversation and memory conformance tests passed, 0 failed.
- Workspace test suite: 107 passed, 0 failed, 1 ignored.
- Conformance gate MEM-01 status: **Not Run**.
