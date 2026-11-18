# Typed untrusted-intent queue (partial evidence)

- **Procedure:** Focused local implementation check for queue boundary and flood backpressure; not the full IPC-01 procedure.
- **Revision:** Branch `feat/p5-m2-ipc-spike`, based on commit `7a32804`, with the source change in the working tree during capture. P4 IPC package 0.3.2 is Draft. Exact source hashes and local tool versions are in `raw.txt`.
- **Check:** The ingress queue holds only `UntrustedUserIntent { session_id, sequence, body }`. Parsing still rejects malformed user-event bodies. The type represents a renderer request only; downstream policy authorization and action dispatch are not implemented.
- **Evidence:** `raw.txt` records formatting, workspace tests, Clippy, blueprint integrity, and the namespace-only separate-UID test. `raw.sha256` verifies the raw record.

This evidence does not demonstrate authorization policy, production renderer binding, Godot lifecycle, slow-reader/concurrent resource limits, coverage-guided fuzzing, or full IPC-01. No conformance gate or acceptance condition is marked Passed.
