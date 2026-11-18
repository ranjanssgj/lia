# Bounded IPC I/O timeout (partial evidence)

- **Procedure:** Request a 3,600-second hello timeout while the peer sends no frame; verify the transport disconnects within the one-second hard maximum. Also run the existing 300 ms slow-drip test to confirm byte-by-byte progress cannot extend an absolute deadline.
- **Contract:** P4 package 0.3.3 (Draft), based on `75656fedc95dbb6948fd722f6d4ccedc7aa73880`, with the implementation change in the working tree at capture. Tool versions and source hashes are retained in `raw.txt`.
- **Control:** Handshake and each frame read/write use an absolute monotonic deadline capped at one second; caller-requested shorter durations are honored. The malformed-frame policy closes after the first invalid/truncated frame.
- **Validation:** Workspace tests (55 passed, 1 namespace-only ignored), formatting, Clippy, Blueprint integrity (6 integrity tests), and the separate-UID namespace test passed. Full command output is in `raw.txt`; `raw.sha256` verifies the record.

This does not measure concurrent contention, production resource budgets, real Godot lifecycle, or complete IPC-01. The contract remains Draft and no gate is marked Passed.
