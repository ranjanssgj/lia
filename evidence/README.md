# Evidence index

This directory indexes retained, reviewed evidence. It starts empty; no implementation or conformance gate is marked Passed by the existence of this index or by the contract-hygiene workflow.

For each gate run, add `evidence/<gate>/<run-id>/README.md` with:

- gate and procedure revision;
- repository commit, contract revisions and SHA-256 hashes of fixtures;
- OS/architecture and exact runtime/tool versions;
- command and immutable CI run URL;
- raw report/log paths or CI artifact names and checksums;
- result, failures, limitations, reviewer and review date.

Evidence must use synthetic identities and redacted output. Do not commit credentials, private key material, real user content, window titles, screen/audio captures, or unredacted production logs. Keep public cryptographic test keys and synthetic database fixtures under `testdata/`; these must remain trackable.

The contract-hygiene workflow is structural validation only. It does not complete API authorization, adapter, IPC, security, replay, restore, provider, workflow, live compositor or end-to-end conformance procedures.

Partial M2 implementation evidence (not a conformance run): [agent endpoint restart and stale-session rejection](IPC-01/2026-11-17-m2-agent-endpoint-restart/README.md), [typed untrusted-intent queue](IPC-01/2026-11-17-m2-typed-intents/README.md), [accept-loop readiness stress](IPC-01/2026-11-17-m2-accept-poll/README.md), and [pre-optimization accept-loop stress](IPC-01/2026-11-17-m2-accept-loop-stress/README.md).

## P5 M1 partial implementation checks (15 November 2026)

- [SQLITE-02 local implementation checks](SQLITE-02/2026-11-15-m1-core/README.md) — migrations 001–005, selected replay/claim tests, WAL and consume-before-effect behavior. The full gate remains Not Run.
- [SECURITY-01 local primitive checks](SECURITY-01-local/2026-11-15-m1-core/README.md) — RFC 8032, command/lease domain separation, and a simple JCS/SHA-256 vector. The full gate remains Not Run.

## P5 M2 partial implementation checks (15 November 2026)

- [IPC-01 local IPC primitive checks](IPC-01/2026-11-15-m2-spike/README.md) — bounded framing, envelope/session validation, private Unix socket setup and Linux peer credentials. The full gate remains Not Run.

## P5 M2 parser and handshake checks (16 November 2026)

- [IPC-01 local parser, handshake and reconnect checks](IPC-01/2026-11-16-m2-parser-edge-cases/README.md) — malformed-frame rejection, UID-authenticated hello/ready exchange, fresh session on reconnect, stale-ready rejection, symlink-path rejection and inode-safe cleanup. The full gate remains Not Run.

## P5 M1 replay follow-up (15 November 2026)

- [M1 expiry and sequence-order checks](SQLITE-02/2026-11-15-m1-replay-followup/README.md) — additional local test coverage on the M2 branch; SQLITE-02 remains Not Run.

## Repository integrity

- [Blueprint truncation incident and recovery (16 November 2026)](repository-integrity/2026-11-16-blueprint-truncation/README.md) — full content restored from `010202c`; integrity thresholds run in contract-hygiene CI.

- [M2 local UnixStream framing benchmark](IPC-01/2026-11-16-m2-local-socket-benchmark/README.md) — 20,000 local request/response exchanges with latency and point-in-time RSS/FD observations; not end-to-end IPC-01 evidence.

## P5 M2 adversarial checks (17 November 2026)

- [Namespace ownership, parent-swap and parser mutation checks](IPC-01/2026-11-17-m2-adversarial-checks/README.md) — separate-UID path denial, foreign-owner rejection, deterministic pinned-directory race cleanup and 50,000 deterministic parser mutations. IPC-01 remains Not Run.
- [Strict presentation body-schema checks (17 November 2026)](IPC-01/2026-11-17-m2-body-schemas/README.md) — strict hello, ready and state-replacement body validation; workspace test and lint output. IPC-01 remains Not Run.
- [Reachable separate-UID peer-auth check (17 November 2026)](IPC-01/2026-11-17-m2-peer-credentials/README.md) — mapped-user-namespace client reaches the test accept boundary and is rejected by kernel credentials before parsing; production permissions remain private. IPC-01 remains Not Run.
- [Bounded queue flood and current-schema mutation smoke (17 November 2026)](IPC-01/2026-11-17-m2-bounded-queue-fuzz/README.md) — 64-frame intent flood stops at queue capacity two; 50,000 deterministic parser mutations use P4 0.3.2 message bodies. IPC-01 remains Not Run.
- [Accept-loop connection burst and reconnect stress (17 November 2026)](IPC-01/2026-11-17-m2-accept-loop-stress/README.md) — three runs of 64 short-lived connections followed by 128 valid reconnect/snapshot cycles, with sampled FD/RSS and handshake latency. IPC-01 remains Not Run.

## Repository integrity review (17 November 2026)

- [Blueprint target-heading superset check and PR #6 self-review notes](repository-integrity/2026-11-17-heading-superset/README.md) — preserves every `origin/main` heading occurrence and records the local checks plus CI run 44020201485. This is not a conformance-gate result.
