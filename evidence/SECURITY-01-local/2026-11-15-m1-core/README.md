# M1 cryptographic primitive checks (partial evidence)

**Gate:** SECURITY-01 local vector subset; full gate status remains **Not Run**.
**Environment:** Linux x86_64; Rust 1.99.1; `ed25519-dalek` 3.0.0, `serde_jcs` 0.2.0, `sha2` 0.11.1.

The tests verify the RFC 8032 Section 7.1 test vector 1, ensure the command profile verifies only its own `LIA-COMMAND-V1\n` signing input and rejects use under the lease verifier, and check a simple sorted-key JCS/SHA-256 fixture.

This is primitive-level evidence only. It does not establish ProtoJSON/Protobuf wire interoperability, key lifecycle, OIDC, mTLS, local IPC identity, signature trust/revocation or a complete SECURITY-01 pass. No conformance gate or acceptance condition is marked Passed.
