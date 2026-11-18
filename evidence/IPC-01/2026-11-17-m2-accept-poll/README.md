# IPC accept-loop readiness stress (partial evidence)

- **Purpose:** Compare local accept-loop behavior after replacing the 10 ms idle sleep with Linux listener readiness polling.
- **Capture:** 17 November 2026, on the Arch Linux host identified in `raw.txt`; Rust/Cargo 1.99.1.
- **Source revision:** Working tree based on `e4afbc71228ff5b86acd4056e7f7ed55961eeae4`; exact source and documentation SHA-256 values are recorded in `raw.txt`. Protocol package 0.3.2 is Draft.
- **Workload:** Three sequential optimized example runs. Each creates a private local Unix socket, offers a burst of 64 peers that disconnect before hello, then performs 128 healthy reconnect/hello/ready/snapshot cycles. Bounded queue capacity is 4. The same test-only synthetic service is used across runs.
- **Command:** `cargo run --locked --release -p lia-device-agent --example ipc_accept_loop_stress` (three runs); namespace peer-credential test and workspace validation commands are also retained in `raw.txt`.

| Run | FD baseline → sampled high | RSS HWM start → end (KiB) | Handshake P50 / P95 / max (µs) |
|---|---:|---:|---:|
| 1 | 5 → 6 | 2672 → 3084 | 89 / 155 / 1684 |
| 2 | 5 → 6 | 2652 → 3072 | 63 / 155 / 683 |
| 3 | 5 → 6 | 2656 → 3076 | 79 / 116 / 796 |

The P95 range is 0.116–0.155 ms, compared with 10.76–10.85 ms in the earlier sleep-based run. This is local synthetic sequential evidence only. FD counts are sampled after completed cycles and may miss transient peaks; RSS is whole-process high-water, not attributed to IPC. The workload does not measure concurrent healthy and abusive peers, slow readers, sustained load, CPU budget or a target P2.2 resource budget. It does not include Godot, a compositor, production process supervision, dispatch policy integration or user content. No conformance gate or acceptance condition is marked Passed.

`raw.txt` contains commands, results, runtime details and input hashes. `raw.sha256` verifies the retained raw log.
