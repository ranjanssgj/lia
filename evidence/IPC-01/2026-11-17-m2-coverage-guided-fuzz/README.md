# Coverage-guided IPC frame parser fuzzing (partial evidence)

- Gate: IPC-01; status remains **Not Run**.
- Base commit: `91daf46b723401ed995befea501f2dd3575bd625`; harness/evidence changes were uncommitted during capture.
- Contract: P4 IPC 0.3.3 (Draft).
- Procedure: isolated nightly `2026-11-17`, `cargo-fuzz` 0.13.2, `libfuzzer-sys` 0.4.13; four synthetic protocol seeds; 60 seconds; max input 262,148 bytes; per-input timeout 5 seconds.
- Result: 4,010,078 executions, average 65,738/s, 5,457 new units, 504 MiB peak RSS; normal completion and no crash artifact observed.
- Limitations: bounded parser-only run. Coverage review and crash triage, sustained resource limits, slow-reader/concurrent-flood tests, production renderer/service lifecycle, authorization boundary, and full IPC-01 evidence/review remain open. This is not a gate pass.
- Raw output and source/fixture hashes: [raw.txt](raw.txt), checksum [raw.sha256](raw.sha256). Paths are redacted; fixtures synthetic.
