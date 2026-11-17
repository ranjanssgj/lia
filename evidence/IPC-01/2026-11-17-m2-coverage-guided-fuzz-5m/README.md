# IPC parser fuzzing — five-minute bounded run (partial evidence)

- Gate: IPC-01; status remains **Not Run**.
- Base commit: `b6064894b898b4e568e523b5ee59ab725fd2dcd3`; harness and evidence were uncommitted during capture.
- Contract: P4 IPC 0.3.3 (Draft).
- Procedure: isolated nightly `2026-11-17`, `cargo-fuzz` 0.13.2, `libfuzzer-sys` 0.4.13, four synthetic seeds, five-minute cap, 262,148-byte max input, 5-second per-case timeout, 700 MiB harness RSS cap.
- Result: 19,261,140 executions, 63,990/s average, 8,157 newly admitted corpus units, 434 MiB peak RSS; normal exit and no crash artifact observed. This run had no fixed fuzzer seed; exact counts vary.
- Coverage replay: fixed generated corpus replayed successfully. `ipc.rs` line coverage was 22.41% (946 lines, 734 missed); this is descriptive only, with no threshold specified. Coverage totals and tool caveat are in [coverage-summary.txt](coverage-summary.txt). The tool stated that fuzzing was not performed during replay.
- Limitations: parser-only. It does not prove IPC service behavior, production policy, or target runtime resource budgets. Full IPC-01 remains Not Run.
- Redacted outputs and source/seed hashes: [raw.txt](raw.txt), checksum [raw.sha256](raw.sha256), [fuzz-output.txt](fuzz-output.txt), [coverage-output.txt](coverage-output.txt).
