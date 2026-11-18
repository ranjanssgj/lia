# M3 task plan — Hyprland v0.57.2 adapter

**Status:** In progress on `feat/p5-m3-hyprland-adapter`; adapter core implemented with title redaction, command allowlist, work-area bounds, and independent read-back verification. **Gate status:** ADAPTER-01, HYPR-01 and HYPR-02 remain Not Run.

**Selected profile:** Hyprland v0.57.2 stable, release tag commit `11cbc6c577b84ffb1edf4df3fc5b6128c3ae6947`, on one Linux host and one supported display. Use Lua configuration (`hyprland.lua`); do not use the removed `.conf` configuration format. The release is documented at [Hyprland v0.57.2](https://github.com/hyprwm/Hyprland/releases/tag/v0.57.2). The versioned [0.57.0 configuration docs](https://wiki.hypr.land/0.57.0/configuring/core/) identify `hyprland.lua` as the config path. v0.57.2 includes a fix for accessing expired window references from Lua, so the adapter must still treat references as short-lived and revalidate before use.

This document defines the M3 work before any adapter code is added. It does not claim the selected build is installed, reproducible, or supported. Record exact source/package provenance, build flags and hashes before running HYPR-01/02. The selected tag and commit must be verified against the downloaded source and release signature as part of that procedure.

## Traceability and contracts

- Acceptance conditions: AC-003 (fresh active-window context), AC-004 (ambiguous/stale/unsupported target handling), AC-005 (current authorization), AC-006 (same-display move and verified read-back), AC-008 (truthful failure/unknown outcome), AC-011 (declared capability envelope).
- Gates: ADAPTER-01 cases applicable to the first profile, HYPR-01 discovery/reconnect/freshness, and HYPR-02 move/read-back. The shared P4 matrix remains authoritative; all three gates start Not Run.
- Contract inputs: P4 package revision 0.3.3 Draft, [adapter conformance v1](../../p4/adapters/adapter-conformance-v1.md), [capability manifest schema](../../p4/adapters/capability-manifest-v1.schema.json), [Hyprland fixture](../../p4/adapters/fixtures/hyprland-wayland-manifest.example.json), and the P4 proto/tool registry. Resolve any contract conflict in a versioned contract change before claiming conformance.
- Dependencies: completed M0 pins/environment, M1 operation/attempt/outcome/evidence persistence, and M2 IPC with renderer identity binding and authorized user-event policy. M3 cannot accept a renderer-originated mutation while those M2 controls remain incomplete.

## In-scope capabilities

1. Discover the active Hyprland instance and current session using validated environment/session metadata. Fail closed if the runtime directory, instance signature, compositor identity, or expected socket is missing, malformed, replaced, or inaccessible.
2. Observe only the focused/active window and the minimum metadata needed for geometry operations. Keep raw titles and application text out of logs, telemetry, evidence, and ordinary state events. If policy requires a label, return a bounded redacted label or opaque resource ID.
3. Produce a capability manifest that names the exact compositor build, session identity, freshness/expiry, supported subset, restrictions, and typed unavailable/permission-required states. Discovery never grants permission.
4. Support one operation: moving a currently authorized, uniquely identified window within the same display/output while preserving its size and workspace. Read back and independently compare geometry and display/workspace after the request.
5. Return typed unsupported, stale, ambiguous, denied, failed, unknown-outcome, and verified-success results. A command acknowledgement is not proof of effect.

Out of scope: closing/resizing windows, cross-display or cross-workspace moves, generic input automation, screen/audio capture, title-based targeting, app-content inspection, arbitrary `hyprctl` commands, arbitrary shell execution, and support claims for other Wayland compositors or X11.

## Hyprland integration boundary

Use Hyprland's documented local IPC only. The request socket is synchronous; open it immediately before one bounded request and close it immediately after receiving the bounded response. Never leave request sockets idle. Keep the event socket separate and long-lived only when needed; reconnect after EOF/error and invalidate freshness until a new discovery snapshot is complete. Treat event stream lines as hints that trigger re-discovery, not as authoritative resource state.

The [Hyprland 0.57.0 IPC documentation](https://wiki.hypr.land/0.57.0/IPC/) describes separate Unix request and event sockets and warns that an unclosed request connection can block the compositor until timeout. Pin the exact v0.57.2 interface behavior in the test report rather than assuming all prior-version details remain identical. The adapter uses a fixed command/query allowlist and structured argument construction. It must not concatenate user-controlled strings into commands or execute through a shell.

Use Lua only for the profile configuration and, if required by the proven v0.57.2 interface, narrowly scoped event/test hooks. Lua hooks may report lifecycle/invalidation hints; they may not authorize operations, persist canonical outcomes, or bypass the Rust adapter's freshness and policy checks. Keep the default profile minimal and include the exact `hyprland.lua` hash in evidence.

## Resource identity, freshness and move procedure

A window resource reference is ephemeral. Bind it to the compositor instance/session generation, a LIA generation token, observed window identity, output/workspace identity, observation timestamp and expiry. Never treat a reusable compositor address or title as a durable identifier. Re-discover immediately before an operation and reject if focus/identity/generation/geometry/output/workspace changed, the window disappeared, the display topology changed, or the reference expired.

### Planned Rust adapter interface specifications

```rust
pub struct WindowIdentity {
    pub address: String,          // Opaque hex address (e.g. "0x55a8f2c3b1a0")
    pub generation_token: String, // Ephemeral generation UUID
    pub workspace_id: i64,        // Workspace identifier
    pub monitor_id: i64,          // Monitor/display output identifier
    pub observed_at_us: i64,      // Integer microsecond timestamp
    pub fresh_until_us: i64,      // Expiration deadline (short-lived, e.g. 10s)
}

pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub struct SameDisplayMoveCommand {
    pub window: WindowIdentity,
    pub target_x: i32,
    pub target_y: i32,
    pub preserved_width: i32,
    pub preserved_height: i32,
    pub preserved_workspace: i64,
}

pub enum MoveOperationResult {
    VerifiedSuccess {
        before: WindowGeometry,
        after: WindowGeometry,
        attempt_id: String,
    },
    VerifiedFailure {
        reason: String,
        attempt_id: String,
    },
    UnknownOutcome {
        attempt_id: String,
        reconciliation_required: bool,
    },
}
```

For an authorized move:

1. Load the exact current manifest and policy decision; validate capability, resource, operation/attempt IDs, arguments and deadline.
2. Re-discover and snapshot the target window and its output/workspace. Check one-display scope and destination bounds using the current work area.
3. Persist the attempt as potentially effected before issuing the compositor request (M1 consume-before-effect invariant).
4. Issue only the fixed move command using validated coordinates; preserve original width, height and workspace. Do not retry automatically after a timeout or connection loss.
5. Re-read the window and output/workspace through a fresh request. Compare against the intended target and preserved dimensions. Record verified success only when the postcondition is observed; if the request may have executed but read-back is missing or conflicting, record unknown and require reconciliation.
6. Persist operation outcome and evidence through the M1 store. Include no title or captured content in the evidence payload.

Permission revocation, policy revision change, resource expiry, compositor restart, event loss, target disappearance and output/workspace change invalidate the operation before effect. Any uncertain post-effect condition is reconciled; never replay blindly.

## Required test and evidence plan

| Area | Required cases | Evidence to retain |
|---|---|---|
| Build identity | Exact v0.57.2 tag/commit, verified signature/provenance, package/build flags, Lua config; reject wrong version or dirty/unexpected build | Source and artifact hashes, signature verification output, compiler/dependency versions, config hash |
| HYPR-01 discovery | Session absent/malformed; request/event socket absent; reconnect; event loss; compositor restart; output hotplug; stale/reused window identity; capability manifest schema/expiry | Redacted session metadata, manifest fixtures/hashes, reconnect/invalidation trace, compositor version |
| ADAPTER-01 | Applicable ADP cases: unsupported/permission states, expiry, resource mismatch, stale policy/grant, duplicate operation, denial, bounded evidence, outcome taxonomy, privacy/log inspection | Case report, fixture hashes, policy decision IDs, operation/attempt/evidence IDs, redacted logs |
| HYPR-02 success | One synthetic floating window moved within one display; preserve dimensions/workspace; independent read-back proves target geometry | Before/after geometry and output/workspace, operation IDs, expected/actual comparison |
| HYPR-02 negative | Stale/reused identity, ambiguous focus, target closes, output changes, out-of-bounds target, denied/revoked grant, timeout before/after possible effect, compositor restart, contradictory read-back | No-effect assertions where provable; otherwise unknown/reconciliation record; request/event traces |
| Resource/privacy | Repeated discovery under load, bounded response/event sizes, title redaction, socket closure, no shell execution, no TCP listener | Latency distribution, memory/FD/socket counts, redaction scan and command audit |

Run only in a disposable test desktop with synthetic windows and data. Capture no screenshots/audio and no real window titles. Retain raw redacted logs, environment summary, contract/fixture/config/build hashes, command, commit, failures, limitations, reviewer and date under `evidence/HYPR-01/<run-id>/`, `evidence/HYPR-02/<run-id>/` and `evidence/ADAPTER-01/<run-id>/`. A compile or schema check alone does not pass any gate.

## Exit criteria

- Exact v0.57.2 build and Lua profile are reproducible and their provenance is retained.
- Resource freshness and identity invalidation tests pass.
- The one same-display move preserves size/workspace and has independent postcondition evidence.
- Denial, staleness, unsupported states and uncertain outcomes are truthful and cannot trigger blind retries.
- Applicable ADP cases and privacy/resource limits pass with reviewed evidence.
- The P4 conformance matrix and P5 status report are updated from Not Run only after all required evidence is reviewed. No general Hyprland/Wayland support is inferred from this single profile.
