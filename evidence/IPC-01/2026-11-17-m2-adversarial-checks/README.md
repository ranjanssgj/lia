# M2 adversarial parser and path checks

**Gate:** IPC-01 remains **Not Run**. These are local partial checks for the Linux Unix-socket implementation.

**Base commit:** `82b90b4415dd0eb905865a0472e86edbb70e7bb0`; exact working-source SHA-256 hashes, environment, commands and raw output are in `raw.txt` and `raw.sha256`.

**Hosted validation:** The corrected branch revision `c9e7a5c5b84c4c30cbe84ce6d4d4a93e94e023e5` passed both the contract-hygiene and device-agent Rust jobs in [GitHub Actions run 44018541606](https://github.com/ranjanssgj/lia/actions/runs/44018541606).

## Results

- The regular workspace suite passed 49 tests; one user-namespace test is deliberately ignored in the ordinary run. Clippy passed with warnings denied.
- Deterministic parent-swap and endpoint-symlink race tests verify that binding fails closed, cleanup stays anchored to the pinned directory, and a raced symlink target's mode and contents are preserved.
- A user-namespace procedure passed for both a foreign-owned runtime directory and a client running as a distinct mapped UID. The separate-UID client cannot traverse the required owner-only mode-0700 directory, so it never reaches `SO_PEERCRED`; distinct-UID rejection on a reachable socket remains untested.
- The deterministic mutation smoke exercised 50,000 length-prefixed JSON frames from valid and malformed seeds: 73 were accepted as valid envelopes, 49,927 were rejected, largest generated payload 354 bytes, and no panic occurred. This is not coverage-guided fuzzing.

## Limits

The procedure does not run a real renderer, verify a compositor, perform probabilistic race stress, measure service-level memory/FD high-water marks under contention, or complete the IPC-01 matrix. No conformance gate is marked Passed. State-changing event dispatch remains disabled.
