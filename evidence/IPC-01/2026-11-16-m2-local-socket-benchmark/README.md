# M2 local UnixStream framing benchmark

**Scope:** Local round-trip baseline for length-prefixed JSON IPC framing over a Rust `UnixStream::pair()`. It includes frame serialization, parsing, OS scheduling and a local responder thread. It does not exercise a bound filesystem socket, renderer supervisor, multi-connection accept loop, real event/state workload, compositor, sustained queue pressure, or IPC-01 as a whole.

**Procedure:** `cargo run --locked --release -p lia-device-agent --example ipc_local_benchmark` (20,000 sequential request/response exchanges). The exact command, base commit, source hash, kernel and Rust/Cargo versions are in `raw.txt`; its checksum is in `raw.sha256`.

**Observed on this run:** p50 10 µs, p95 23 µs, p99 31 µs, maximum 669 µs; process RSS 3,216 kB and 5 open file descriptors at completion. These are one local sample and are not service budgets or performance guarantees.

**Limitations:** This provides no renderer service queue, concurrent clients, backpressure recovery, foreign-UID/PID checks, desktop compositor, or adversarial load measurements. Resource counts are point-in-time process values, not high-water marks. IPC-01 remains Not Run.
