# Presentation authorization boundary (partial evidence)

- Gate: IPC-01; status remains **Not Run**.
- Base commit: `998d62f7178992d8c65914accabdf634f2ee3a01`; implementation/evidence changes were uncommitted during capture.
- Contract: P4 IPC 0.3.3 (Draft), §4.1 user-event schema and §8 IPC-01.
- Implemented: a separate Rust policy adapter requires trusted UI registry resolution and a deterministic policy decision before yielding a typed `AuthorizedPresentationIntent`. IPC still yields only `UntrustedUserIntent`; the policy adapter does not dispatch or authorize OS operations.
- Tests: successful synthetic registry+policy, unknown control rejected before policy, and registered control denied by policy. **3 passed.**
- Limitations: test doubles only; no production UI registry, local permission authority, action handler, Godot binding, capability adapter, or revocation/race test. No product control IDs were selected. Full policy integration and IPC-01 remain open.
- Output and SHA-256 source/report hashes: [raw.txt](raw.txt), checksum [raw.sha256](raw.sha256).
