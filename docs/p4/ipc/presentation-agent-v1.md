# Presentation ↔ Device Agent IPC Contract

**Protocol:** presentation-agent/1  
**Status:** Draft — IPC-01 proof pending  
**Consumers:** Godot presentation process and Rust/Tokio local agent

## 1. Responsibility boundary

The Rust agent owns OS integration, device identity, permission enforcement, local execution journals, operation dispatch, and platform adapters. Godot owns character rendering, animation, facial expression, pose, gaze, local visual transitions, and presentation input.

Godot is not a trusted executor. It cannot authorize an operation, mint a grant, dispatch a desktop action, read arbitrary files, run a shell command, access provider credentials, mutate workflow state, or claim an operation succeeded. A user interaction inside Godot becomes an intent/event sent to the Rust agent; the agent validates it against the current UI/session policy and exposes only approved actions.

The bridge carries presentation state and user-interaction events. It is not the general LIA service bus.

## 2. Transport selection

| Platform | Initial transport | Peer authentication |
|---|---|---|
| Linux | Unix domain stream socket in a per-user runtime directory | Socket ownership/mode plus SO_PEERCRED UID/PID validation |
| Windows | Named pipe with a per-user DACL | Pipe client token/SID validation and process identity check |
| macOS | Future adapter; not in the first-class P5 target | Platform-specific local IPC adapter |
| Development fallback | Supervised child-process stdin/stdout | Parent/child process relationship and inherited pipe handles |

No localhost TCP listener is permitted. A filesystem socket is created under the session's private runtime directory, not a predictable shared temporary directory. On Linux, the agent checks peer credentials after accept and before processing a frame. On Windows, the pipe ACL admits only the intended user/logon context and the agent verifies the connecting process identity. The endpoint is removed on clean shutdown; stale endpoints are handled without following symlinks or replacing another user's socket.

The final transport implementation remains gated by IPC-01. This document selects the OS-native local transport profile for the first implementation; supervised stdio remains a test/dev fallback, not a second production protocol.

## 3. Framing and message envelope

Transport is a full-duplex byte stream. Each frame is a 4-byte unsigned big-endian payload length followed by exactly that many UTF-8 bytes containing one JSON object. The length prefix is not included in the JSON. Maximum frame size is 262,144 bytes. A zero-length, oversized, truncated, invalid UTF-8, malformed JSON, or structurally invalid frame is rejected and counted. The connection is closed after repeated malformed frames; the threshold is a runtime policy value to be pinned in IPC-01.

Each frame uses this envelope:

~~~json
{
  "protocol_version": "1",
  "message_id": "uuid",
  "session_id": "uuid-or-omitted",
  "session_sequence": "uint64-decimal-string-or-omitted",
  "request_id": "uuid-or-null",
  "correlation_id": "uuid-or-null",
  "kind": "request-or-response-or-event",
  "method": "presentation.state.replace",
  "sent_at": "RFC3339 timestamp",
  "deadline_at": "RFC3339 timestamp-or-null",
  "body": {}
}
~~~

- message_id is unique per frame. request_id correlates a response with its request.
- A `presentation.hello` request omits `session_id` and `session_sequence`. Its successful response supplies a fresh random UUID `session_id`; the session sequence remains omitted.
- The first frame on a connection must be a non-expired `presentation.hello` request. Its response has a fresh `message_id`, echoes the request's optional `request_id` and `correlation_id`, and carries the newly issued `session_id`. The connection enters an awaiting-ready phase after hello; a rejected/expired hello does not change phase.
- `message_id` identifies one frame. For a request/response exchange, the response echoes the request's `request_id` unchanged; it does not substitute the request's `message_id` for that correlation value.
- Every subsequent frame carries that `session_id`. It is scoped to one authenticated connection and discarded on disconnect. A reconnect receives a new ID, so captured frames from the old process generation fail session validation.
- Before user events are accepted, the renderer must send exactly one non-expired `presentation.ready` request carrying the issued `session_id`. A repeated, expired, or wrong-session ready request does not enter the ready phase. After ready, the agent should publish a fresh snapshot; socket-level ordering and snapshot delivery remain part of IPC-01 integration.
- A `presentation.user_event` request carries `session_sequence` as a canonical unsigned decimal string to avoid JSON number precision loss. It begins at `"0"`; each later accepted user event must be strictly greater than the last accepted value. Equal, lower, noncanonical, or out-of-range values are rejected. Responses and agent events omit this field.
- The sequence watermark is per live presentation connection and is consumed before user-event dispatch. It is a replay/order control, not peer authentication, a permission grant, or proof that a UI action is allowed.
- correlation_id links a presentation update to the originating conversation, turn, or operation without including the full content.
- method is an allowlisted contract operation, not a dynamically resolved function name.
- deadline_at is checked by the receiver. Expired presentation events are discarded; stale user intents are never replayed as actions.
- Body schemas are versioned and validated with unknown properties rejected unless a method explicitly permits extension metadata.
- A frame can carry references and digests, not arbitrary binary media. Character assets are loaded through the presentation asset pipeline, not the IPC control channel.

## 4. Method allowlist

| Method | Direction | Purpose |
|---|---|---|
| presentation.hello | Godot → Rust | Negotiate protocol and renderer build |
| presentation.ready | Godot → Rust | Report renderer/asset readiness |
| presentation.user_event | Godot → Rust | Report explicit user interaction with a control owned by the UI |
| presentation.state.replace | Rust → Godot | Replace the derived visible character/UI state |
| presentation.state.patch | Rust → Godot | Apply a small versioned presentation update |
| presentation.action.play | Rust → Godot | Request an animation/gesture/pose by allowlisted action ID |
| presentation.audio.state | Rust → Godot | Report speech playback state; does not carry credentials or raw microphone input |
| presentation.health | Bidirectional | Health, frame-budget and asset-load telemetry |
| presentation.shutdown | Rust → Godot | Graceful renderer shutdown |

A user_event identifies a UI control ID and a typed event (clicked, dismissed, text submitted, interruption requested). It does not contain a free-form tool name or arbitrary command. The Rust agent first validates the live session ID and strictly increasing client sequence, then resolves the control ID against the current UI action registry and applies authentication, policy, resource and permission checks. Same-UID `SO_PEERCRED` does not identify a specific renderer; until the agent has a verified renderer-session binding, the IPC layer must not dispatch user events to state-changing handlers.

No method is defined for shell execution, arbitrary process launch, filesystem reads/writes, key injection, global pointer control, browser automation, provider API calls, memory mutation, or permission changes.

## 5. State model

The Rust agent publishes a presentation snapshot with:
- monotonically increasing state_revision;
- stable character_id and current presentation session;
- high-level emotional label and bounded intensity;
- pose/action ID and optional gaze target class;
- speaking/listening/paused presentation flags;
- approved notification/attention cue;
- target window/resource label only where that label is already authorized for presentation.

The snapshot is derived state. Godot cannot edit canonical personality, emotional memory, user profile, permissions, task status, or operation evidence. Unknown emotion/action identifiers resolve to a neutral fallback and are logged as contract incompatibilities, not dynamically interpreted as scripts.

## 6. Backpressure, cancellation and lifecycle

- One connection is active for one presentation process generation. A new process generation gets a new random session ID and invalidates the prior IPC session. The sequence watermark exists only for that live connection; no old user event is replayed after reconnect.
- A connection accepts no user event before one successful hello and one matching ready request. Hello and ready cannot be repeated to reset connection state. The receiver checks user-event deadlines against its trusted integer clock sample before consuming sequence.
- State replacement may coalesce older unsent snapshots. User interaction events, shutdown, and operation cancellation signals are not silently coalesced.
- Every request has a deadline and bounded response. There is no unbounded queue.
- Renderer restart does not replay old user actions. The Rust agent sends a fresh snapshot after a successful hello/ready exchange.
- Disconnecting the renderer does not terminate an OS operation or workflow; it only removes presentation. User interruption/stop is delivered separately to the Rust authority.
- A stale response is ignored by request_id and process generation. A stale user event is rejected by session ID or monotonic session sequence before dispatch.
- The Rust agent continues operating without Godot when safe; presentation failure is a degraded UI state, not loss of device authority.

## 7. Observability and data minimization

Every request/response carries message_id and correlation_id. Logs may include method, schema version, frame size, latency, peer identity class, error code, and process generation. They must not include raw screen frames, microphone audio, camera frames, secrets, full prompt/context packages, or private file contents.

## 8. IPC-01 acceptance evidence

1. Linux peer credentials reject another UID and an unexpected process; endpoint permissions are verified.
2. Windows named-pipe ACL and client identity reject another user/session.
3. Oversized, truncated, malformed, invalid-version and unknown-method frames fail closed; duplicate, backward, noncanonical and cross-session user-event sequences are rejected before dispatch.
4. A renderer restart establishes a new random session UUID and receives a state snapshot; stale frames and replayed/out-of-order user events are ignored. The current Rust session-state primitive tests hello/ready gating and correlation; socket-level restart/snapshot integration remains open.
5. A malicious/modified Godot client cannot invoke an OS capability or forge operation evidence.
6. Backpressure remains bounded; state coalescing does not drop user stop/interruption signals.
7. Rust agent and Godot can independently restart without stuck sockets, orphaned processes or stale action replay.
8. IPC latency and serialization overhead are recorded under the P2.2 resource/latency budget.

Passing these tests does not prove the Godot 4.7.2 migration, VRM import/retargeting, transparent-window behavior, compositor support, or desktop window movement. Those remain GODOT-01 and HYPR-01/02 evidence gates.
