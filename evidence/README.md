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
