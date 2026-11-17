# Synthetic accept-loop connection burst and reconnect stress

**Gate:** IPC-01 remains **Not Run**. This is a local synthetic stress measurement, not a conformance result or desktop performance claim.

**Workload:** The Rust accept loop receives a burst of 64 local connections that close before hello, followed by 128 valid hello/ready/reconnect/snapshot cycles. The event queue capacity is four. After each successful snapshot cycle the example samples `/proc/self/fd`; process RSS uses Linux `VmHWM`. Handshake timings include local connect, hello/ready exchange, and fresh snapshot receipt. No Godot process or compositor is involved.

**Environment:** Arch Linux 7.2.14 x86_64; Rust/Cargo 1.99.1. Exact commands and output are in `raw.txt`.

## Results

| Run | FD baseline → sampled max | RSS HWM start → end (KiB) | Handshake P50 / P95 / max (µs) |
|---|---:|---:|---:|
| 1 | 5 → 6 | 2,652 → 3,072 | 10,570 / 10,808 / 11,181 |
| 2 | 5 → 6 | 2,644 → 3,068 | 10,506 / 10,847 / 11,349 |
| 3 | 5 → 6 | 2,704 → 3,120 | 10,545 / 10,757 / 11,172 |

Workspace validation in the same run: 52 passed, 0 failed, 1 namespace-only test ignored; formatting and Clippy with warnings denied passed.

## Limits

FD samples are taken after completed session cycles and may miss transient peaks. RSS is whole-process high-water, not per-connection attribution. The peer burst is sequentially drained before the healthy reconnect sequence; no live healthy client is measured while abusive clients are concurrently active. No slow-reader test, long-duration load, target P2.2 budget comparison, coverage-guided fuzz, production Godot lifecycle, or full IPC-01 procedure was run. No conformance gate or acceptance condition is marked Passed.
