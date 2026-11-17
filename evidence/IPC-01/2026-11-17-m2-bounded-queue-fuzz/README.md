# Bounded ingress flood and parser mutation smoke

**Gate:** IPC-01 remains **Not Run**. These are focused adversarial checks, not conformance results.

## Results

- A 64-frame user-event sender targets a synchronous queue with capacity two. The Rust session closes on the first enqueue beyond capacity; exactly two untrusted intents remain queued. The stress is local to one socket pair and does not measure concurrent service load.
- The deterministic parser mutation example now seeds the P4 0.3.2 hello, ready, user-event and state-replacement schemas. With seed `0x8f315a27d4c9b601`, 50,000 mutations completed: 41 accepted, 49,959 rejected; largest mutated payload was 444 bytes. This is not coverage-guided fuzzing.
- Separate-UID `SO_PEERCRED` rejection at the accept boundary passed under a mapped user namespace.
- `cargo fmt --all -- --check`, `cargo test --locked --workspace` (52 passed, 0 failed, 1 ignored), and Clippy with warnings denied passed.

`raw.txt` contains the exact commands, environment, results and source hashes.

## Limits

No concurrent slow-reader or connection flood, healthy-client latency under contention, service RSS/FD high-water measurement, coverage-guided fuzzing, production Godot lifecycle, or full IPC-01 procedure was run. The queue bound and deterministic mutation run are partial evidence only; no conformance gate or acceptance condition is marked Passed.
