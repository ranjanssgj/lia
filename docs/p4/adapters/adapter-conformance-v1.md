# Adapter Capability and Conformance Contract v1

**Status:** Draft — no production adapter has passed conformance  
**Schema:** [capability-manifest-v1.schema.json](capability-manifest-v1.schema.json)  
**Example:** [hyprland-wayland-manifest.example.json](fixtures/hyprland-wayland-manifest.example.json)

## 1. Manifest purpose

Every adapter publishes a versioned, current capability manifest for its owning node/resource domain. The manifest supports planning, routing, permission review and compatibility checks. It describes technical support and runtime conditions; it is not an authorization grant.

The manifest carries stable adapter identity/version, contract family, host/runtime compatibility, execution locality, trust/lifecycle state, resource identities/generations/revisions, per-capability schemas, supported scopes, minimum risk floor, confirmation requirements, preconditions, locality/sensitivity/retention, idempotency/retry/reconciliation/verification semantics, events, quotas and health constraints.

Capability availability uses exactly these states:
- supported: technically implemented and current prerequisites hold;
- permission_required: supported but a required user/OS/provider grant is absent;
- unsupported: platform/resource/API does not implement it;
- unavailable: owner/resource/session is offline or not addressable;
- degraded: capability remains partially usable under declared impairment;
- restricted: currently blocked by policy, risk, session or platform constraint;
- unknown: support cannot be established and must not be assumed.

A missing or unknown capability is never emulated by invoking a broader tool.

## 2. Manifest refresh and invalidation

Manifest revision is monotonic within one adapter identity. A manifest is refreshed after enrollment/reconnect, adapter update, OS/compositor/provider version change, permission/scope change, resource reconfiguration, or detected capability/health change. It has observed_at and finite expires_at; expired manifests can support diagnostics but cannot authorize execution.

The manifest and resource observations are projections from the resource-owning node/provider. Live desktop/window/process state remains owned by the local desktop node. Provider state remains provider-authoritative. Manifest content cannot write canonical workflow state, grants or memory.

## 3. Resource reference and generation

A resource reference binds owner, resource type, opaque resource ID, generation and source revision. Display labels are not security identifiers. Reused window handles or recreated app/provider objects must change generation. A stale resource generation fails before effect and is not retargeted by title/name matching.

Each observation includes source adapter/version, method, resource revision/generation, observed-at, freshness expiry, sensitivity/scope and correlation/evidence reference. Observations are timestamped facts, not timeless truth.

## 4. Operation path

The operation executor performs these checks in order immediately before side effect:
1. Authenticate issuer/session, signed target, expiry and replay state.
2. Resolve target device, adapter, resource ID and generation.
3. Negotiate manifest, capability and operation schema versions.
4. Re-check current grant, purpose, scope, risk floor, confirmation, revocation and policy version.
5. Re-read resource state and validate freshness/preconditions.
6. Validate typed arguments and immutable argument/approval digest.
7. Execute only the declared capability; no undeclared adapter chaining.
8. Verify actual resource/provider state using the declared method.
9. Report typed outcome and evidence to the workflow authority.

The coordinator routes and supervises. The resource-owning adapter executes and verifies. A capability manifest is not permission; an authenticated session is not a standing grant; a worker lease is not authorization.

## 5. Idempotency, retries and cancellation

Each capability declares whether effects are idempotent_target_state, provider_idempotency_key, deduplicate_by_operation_id, or non_idempotent. A retry must follow the declared mechanism and current authority. The same logical effect retains its stable operation ID; each command dispatch attempt has a unique command ID and each worker attempt has a unique attempt ID.

A timeout after a possible effect produces outcome_unknown or reconciliation_required. The adapter inspects actual state before retry or reassignment. Cancellation is not rollback: an adapter declares safe cancellation points and reports whether the effect may already have occurred.

## 6. Typed outcomes

The adapter reports one of:
- rejected_before_effect;
- failed_before_effect;
- succeeded_verified;
- completed_with_warning;
- partial_effect;
- outcome_unknown;
- cancelled_before_effect;
- cancellation_requested_after_effect.

Result fields include operation/command/attempt IDs, target/resource/capability, adapter and schema revisions, start/end time, resource revision before/after, outcome/error code, retry/reconciliation instructions and evidence references. A transport acknowledgement or process exit is not verified success.

Evidence identifies source adapter/version, exact resource generation, observation time, method, before/after revision, content digest and verification strength. Stale or inferred evidence cannot be promoted to verified.

## 7. Adapter conformance suite

Every adapter implementation runs the same semantic contract suite plus adapter-specific fixtures. A platform is supported only for the capabilities whose tests pass on the stated OS/runtime/compositor/provider version.

| ID | Fixture / condition | Expected behavior |
|---|---|---|
| ADP-001 | Valid current manifest | Schema validates; capability list and version are accepted |
| ADP-002 | Missing required manifest identity or owner | Reject manifest; no capability exposure |
| ADP-003 | Unknown manifest major version | Fail closed; mark adapter incompatible |
| ADP-004 | Expired manifest | No new operation dispatch; rediscover/refresh first |
| ADP-005 | Capability marked unknown/unsupported | Planner cannot select it; no fallback to shell/global input |
| ADP-006 | Supported capability but no user grant | Report permission required/deny before effect |
| ADP-007 | Grant revoked after planning | Current executor check denies before effect |
| ADP-008 | Resource ID reused with different generation | Reject stale resource; do not retarget another window/device |
| ADP-009 | Resource observation older than freshness bound | Re-read or block; no stale-state action |
| ADP-010 | Arguments contain unknown field or out-of-range value | Strict schema/semantic validation rejects before effect |
| ADP-011 | Command target/account/credential generation mismatch | Reject signature/envelope binding |
| ADP-012 | Duplicate command ID | No second dispatch/effect; return prior disposition |
| ADP-013 | Same operation ID, new attempt after known pre-effect failure | Retry only within finite policy and current grant |
| ADP-014 | Timeout after effect may have occurred | outcome_unknown; reconciliation required; no blind retry |
| ADP-015 | Worker lease generation is stale | Evidence may be retained; stale worker cannot advance canonical state |
| ADP-016 | Cancellation before effect | Stop safely and report cancelled_before_effect |
| ADP-017 | Cancellation after possible effect | Report actual boundary; verify/reconcile; never claim rollback |
| ADP-018 | Adapter advertises verify but has no independent evidence | Conformance fails; capability cannot claim verified success |
| ADP-019 | Provider event/webhook duplicated or unauthenticated | Reject/quarantine or dedupe; no authority mutation |
| ADP-020 | Cross-account resource reference | Deny without revealing object existence or metadata |
| ADP-021 | Health degraded for one capability only | Mark that capability degraded; unaffected capabilities keep separate state |
| ADP-022 | Adapter update/downgrade | Re-negotiate versions; revoked grants and deleted data remain revoked/deleted |
| ADP-023 | Logs include secret/raw capture/unrestricted memory | Privacy test fails; redact/remove and block release |
| ADP-024 | Godot/presentation client attempts OS operation | Rust bridge rejects; presentation process has no OS executor authority |

## 8. First profile: Hyprland Wayland

The first Linux profile is a specific Hyprland Wayland session, not generic Wayland. The manifest shall report:
- Hyprland version and compositor/session identity;
- active user/session and output/workspace capability profile;
- whether the supported IPC request/event sockets are reachable;
- window identity/generation and geometry observation freshness;
- the exact supported capability subset for geometry observation and same-display window movement;
- unsupported states where the required compositor interface or session is absent.

Request socket connections are short-lived and closed after each synchronous request. The event socket is separate. After a move request, the agent reads back geometry/output/workspace and compares with the requested target. IPC acknowledgement alone is not verification. Reconnect/event loss invalidates affected freshness and triggers rediscovery.

No generic Wayland window-control capability is claimed. X11, other compositors, Windows UI Automation and macOS are separate conformance profiles.

## 9. Evidence needed to mark an adapter Supported

A conformance report records contract revision/hash, adapter build digest, OS/runtime/compositor/provider version, manifest fixture, account/device identity using synthetic data, operation ID/command ID/attempt ID, grant/policy conditions, expected/actual results, redacted logs, evidence references, failures, reviewer and date. A capability is marked supported only for the tested matrix; passing Hyprland does not establish general Linux or Windows support.
