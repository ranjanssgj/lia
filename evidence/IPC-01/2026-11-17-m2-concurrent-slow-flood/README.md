# Concurrent slow-handshake and event-flood check (partial evidence)

- Gate: IPC-01; status remains **Not Run**.
- Base commit: `e99737c46c88c2d1aa7c22e1ee6732fc18efbc38`; implementation/evidence changes were uncommitted during capture.
- Contract: P4 IPC 0.3.3 (Draft).
- Procedure: eight synthetic same-process peers send incomplete frame headers and remain connected while one healthy peer negotiates a fresh session and snapshot. The healthy peer then submits three ordered user events to a capacity-two queue. The test requires the queue to retain exactly two events and the overflowing session to close. The server requests a 50 ms handshake timeout.
- Result: five consecutive runs passed; each took 0.51 seconds.
- Limitations: bounded local procedure only. It does not measure sustained concurrent load, process RSS/FD high-water, target resource budgets, or production renderer behavior. IPC-01 remains Not Run.
- Redacted output and source hashes: [raw.txt](raw.txt), checksum [raw.sha256](raw.sha256).
