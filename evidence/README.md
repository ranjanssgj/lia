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

## P5 M1 partial implementation checks (15 November 2026)

- [SQLITE-02 local implementation checks](SQLITE-02/2026-11-15-m1-core/README.md) — migrations 001–005, selected replay/claim tests, WAL and consume-before-effect behavior. The full gate remains Not Run.
- [SECURITY-01 local primitive checks](SECURITY-01-local/2026-11-15-m1-core/README.md) — RFC 8032, command/lease domain separation, and a simple JCS/SHA-256 vector. The full gate remains Not Run.

## P5 M2 partial implementation checks (15 November 2026)

- [IPC-01 local IPC primitive checks](IPC-01/2026-11-15-m2-spike/README.md) — bounded framing, envelope/session validation, private Unix socket setup and Linux peer credentials. The full gate remains Not Run.

## P5 M2 parser and handshake checks (16 November 2026)

- [IPC-01 local parser, handshake and reconnect checks](IPC-01/2026-11-16-m2-parser-edge-cases/README.md) — malformed-frame rejection, UID-authenticated hello/ready exchange, fresh session on reconnect and stale-ready rejection. The full gate remains Not Run.

## P5 M1 replay follow-up (15 November 2026)

- [M1 expiry and sequence-order checks](SQLITE-02/2026-11-15-m1-replay-followup/README.md) — additional local test coverage on the M2 branch; SQLITE-02 remains Not Run.
