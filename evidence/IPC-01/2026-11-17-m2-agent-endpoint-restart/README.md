# Agent endpoint restart and stale-session rejection (partial evidence)

- **Procedure:** Recreate the local Rust agent socket endpoint after shutdown, reconnect with a fresh handshake, then submit a user event carrying the pre-restart session ID.
- **Contract:** P4 IPC package 0.3.2 (Draft). Branch `feat/p5-m2-ipc-spike`, based on `627edea` with the test change in the working tree at capture. Runtime versions and source hashes are in `raw.txt`.
- **Expected/observed:** The old endpoint path is removed at shutdown and can be rebound. The new handshake issues a different session UUID. The old-session user event is rejected with `PermissionDenied`; endpoint cleanup succeeds after the second server exits.
- **Validation:** Workspace tests (54 passed, 1 namespace-only ignored), formatting, Clippy, Blueprint integrity (6 integrity unit tests), and the separate-UID namespace test passed. Complete output is in `raw.txt`; `raw.sha256` checks the capture.

This simulates endpoint ownership recreation in one process. It is not evidence of agent process crash recovery, production supervisor wiring, Godot lifecycle, concurrent contention/resource budgets, coverage-guided fuzzing, or the complete IPC-01 gate. No conformance gate or acceptance condition is marked Passed.
