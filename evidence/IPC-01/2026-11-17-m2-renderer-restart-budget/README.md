# Renderer supervisor restart budget (partial evidence)

- Gate: IPC-01; status remains **Not Run**.
- Base commit: `fdcc44270f86817f4773a7a82456f018cce60c62`; the test and documentation updates were uncommitted during capture.
- Contract: P4 IPC 0.3.3 (Draft).
- Test: `renderer_supervisor_enforces_bounded_restart_budget` starts synthetic child processes that exit immediately, verifies three restarts using the configured 50/100/200 ms backoff, then verifies fail-closed behavior after the restart limit.
- Result: passed in approximately 0.40 seconds.
- Limitations: exercises the Rust supervisor primitive only; no Godot renderer, production service wiring, real desktop launch, or availability target. IPC-01 remains Not Run.
- Command output and source hashes: [raw.txt](raw.txt), checksum [raw.sha256](raw.sha256).
