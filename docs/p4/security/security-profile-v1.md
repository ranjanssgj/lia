# P4 Security and Authentication Profile v1

**Status:** Draft — SECURITY-01 proof and owner review pending  
**Scope:** User API, device/control-plane gRPC, signed commands/offline leases, and local IPC  
**Non-goal:** Selecting an identity-provider, certificate-authority, KMS/HSM, or hosting vendor

## 1. Principals and credential separation

LIA distinguishes:
- authenticated human/account principal;
- enrolled device installation principal;
- service/workload principal;
- external connector/account principal;
- local OS process principal.

A user session is not a device identity. A device certificate is not an operation grant. A Temporal worker lease is not an authorization grant. A connector OAuth consent is not a LIA permission grant. A model session is not any of these principals.

Credentials are never shared across these classes. Private keys and connector refresh credentials stay in an OS protected store or the separately governed credential authority. They do not enter Memory Authority, NATS payloads, Temporal history, normal logs, application backups, or model context.

## 2. Human sign-in and HTTP API

Interactive sign-in uses OpenID Connect Authorization Code flow with PKCE. The client validates the OIDC issuer, audience/client binding, state, nonce, redirect URI and token validity using the issuer's discovery and key-rotation metadata. The identity-provider vendor and issuer URL are deployment configuration.

Every HTTP request carries an access token in the Authorization bearer header over TLS. The API validates token authenticity or introspection, issuer, audience, expiry, not-before, client, and applicable scopes. It then independently checks the LIA account, object, resource, purpose, risk, grant, policy version and current revocation state.

Proposed coarse API scopes:
- lia.devices.read
- lia.devices.manage
- lia.objectives.read
- lia.objectives.write
- lia.runs.read
- lia.runs.control
- lia.grants.read
- lia.grants.revoke
- lia.approvals.decide

These scopes are necessary routing/consent constraints; they do not replace resource-level grants. API tokens, authorization headers, refresh tokens and raw identity assertions are redacted from logs and traces.

## 3. Device identity and control-plane transport

Device control uses TLS 1.3 with mutual X.509 client authentication. The certificate contains one device identity URI SAN in the form `urn:lia:device:<device_uuid>`. The identity registry binds the certificate public-key fingerprint, device ID, account ID, credential generation, status and revocation epoch. OpenSession's device_id and credential_generation must match that binding.

The server checks certificate validity/trust, registered fingerprint, credential generation, enrolled state, revocation epoch, protocol compatibility and session purpose. Reconnect is a new session-establishment event. Session expiry or revocation invalidates the logical session even if a transport socket remains open.

The gRPC metadata contract is:
- `authorization`: only where a separately issued service/user access token is part of the service profile; never a device secret copied into application logs;
- `x-lia-session-id`: session reference issued by OpenSession;
- `traceparent` and optional `tracestate`: W3C trace context;
- no custom header may widen authority or substitute for the device certificate.

The server must enforce message/header limits and finite RPC deadlines. Client-side deadlines and explicit cancellation are required for request/response RPCs. The Connect stream has heartbeat, idle and reconnect policy; a stream reconnect is never treated as continuity of authorization.

The exact certificate issuing hierarchy, trust-anchor rotation, certificate key algorithm, certificate lifetime, revocation protocol, protected-storage adapter and issuer/KMS vendor remain open proof/deployment choices. They must be pinned before a production credential can be issued.

## 4. Command and offline-lease signature profile

Two allowlisted Ed25519 signature profiles share a fixed 64-byte raw signature representation but use different signed message types and domain-separation prefixes.

**Online command profile:** lia-ed25519-jcs-v1.

- Envelope representation: Protocol Buffers JSON mapping for CommandEnvelope, serialized as UTF-8 JSON and canonicalized using RFC 8785 JSON Canonicalization Scheme.
- Signing input: ASCII bytes LIA-COMMAND-V1 followed by a newline byte, then the canonical JSON UTF-8 bytes. Signature fields are excluded.
- The envelope binds issuer, audience/target, account, target credential generation, operation/command IDs, resource generation/revision, capability/schema version, exact arguments digest, purpose/task references, grant/policy/revocation version, risk, approval, not-before, expiry, nonce/sequence and correlation/causation.

**Offline authorization lease profile:** lia-offline-lease-ed25519-jcs-v1.

- Signed message: Protocol Buffers JSON mapping for OfflineAuthorizationLease, serialized as UTF-8 JSON and canonicalized using RFC 8785 JSON Canonicalization Scheme.
- Signing input: ASCII bytes LIA-OFFLINE-LEASE-V1 followed by a newline byte, then the canonical JSON UTF-8 bytes. The command profile prefix is never accepted for a lease and vice versa.
- The lease binds issuer/audience, account, one target device and credential generation, one operation ID, exact capability/schema and resource generation, arguments digest, purpose/task, grant, policy and revocation epochs, risk, issued/not-before/expiry, nonce and signing-key generation.
- The coordinator issues an expiry no later than five minutes after issue time. The device accepts only R0/R1 local operations in the P3.3 allowlist; R2–R4, remote/cross-device, connector, publication and home safety/security operations are rejected.
- A lease is single-use and bound to one exact operation. The device verifies current local trust/revocation state, resource generation and arguments digest, atomically records lease consumption before effect, and reports the local journal for reconciliation after reconnect.

For both profiles, signing_key_id resolves to a currently trusted coordinator command-signing public key and key generation. The profile identifier fixes the algorithm and canonicalization; verifiers never accept an algorithm selected by untrusted input. The verifier compares the signed payload with the typed protobuf fields, checks digests/current target/capability/resource/permission state and signature, then atomically records command or lease consumption before any non-idempotent side effect.

Command-signing keys are separate from TLS client/server keys and provider credentials. Public-key rotation is versioned by key ID. The verifier supports an overlap window for rotation, but a revoked key, stale credential generation, policy/revocation epoch mismatch, expired command, consumed nonce or wrong target fails closed.

Ed25519 is proposed as the interoperable application-signature baseline using the EdDSA conventions in RFC 8037. If a FIPS or other mandatory compliance profile is introduced, the signing suite must be reevaluated and versioned before production rather than silently downgraded.

## 5. Offline authorization lease

The P3.3 maximum remains five minutes from coordinator authorization and applies only to explicitly allowlisted local R0/R1 operations. A lease is:
- bound to one enrolled installation, credential generation, policy/revocation epoch, operation ID, capability and exact resource/argument digest;
- signed under a distinct offline-lease domain-separation prefix;
- fixed-expiry, single-use and recorded in the local consumed-lease journal;
- user-visible and auditable;
- rejected if local time is uncertain, resource generation changed, prior execution is ambiguous, key state changed, or the lease has been consumed.

No R2/R3/R4, cross-device control, external publication, connector action, home safety/security actuation, or newly broadened scope may execute under an offline lease. Reconnect submits the local journal and reconciles against canonical workflow state; it does not replay old GUI commands.

## 6. Local IPC identity

Linux uses Unix domain sockets with restrictive per-user runtime-directory ownership/mode plus SO_PEERCRED UID/PID verification. Windows uses a named pipe with a restrictive DACL and client SID/process verification. The bridge is local-only. It does not rely on a secret embedded in the Godot binary and does not accept unauthenticated localhost TCP.

Local process identity proves which same-user process connected; it does not make the presentation process an authority. Every UI-originated intent is still validated by the Rust agent and the appropriate permission/workflow authorities.

## 7. Open choices and production blockers

The following remain open and must not be inferred:
- identity-provider vendor, issuer and account-recovery implementation;
- certificate authority, trust-anchor lifecycle, certificate profile and issuance automation;
- KMS/HSM/secret-store vendor and hardware-backed-key support per target OS;
- concrete certificate key algorithm/lifetime and revocation distribution;
- deployment region and any data-residency/compliance requirement;
- key-rotation overlap, emergency revocation and offline lease operational procedure;
- exact TLS library/version and gRPC interceptor implementation.

No production credentials or side-effect operations may be enabled solely because this document exists. SECURITY-01 must retain threat scenarios, fixtures, key/certificate versions, raw pass/fail results and reviewer sign-off.
