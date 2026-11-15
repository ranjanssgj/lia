# M2 Unix-socket IPC threat-to-test map (spike entry criteria)

Scope: Linux local Unix-domain socket between the device agent and the presentation renderer. The renderer displays state and sends user-control requests; it does not receive authority to execute tools, grant permissions, or mutate canonical state. Socket IPC is a local transport, not an authentication substitute for policy checks.

| Threat / assumption | Required control | Test and evidence |
| --- | --- | --- |
| Attacker pre-creates or swaps the socket path | Private runtime directory; reject symlink/non-socket; create with restrictive mode; safe cleanup tied to inode | Pre-create regular file, symlink and foreign-owned socket; verify startup fails closed and records path/mode/owner |
| Other local user connects | Check `SO_PEERCRED` UID against agent's expected user; never trust claimed PID/UID in payload | Connect as another UID where CI permits; compare kernel credentials with forged JSON identity; deny |
| Same-user process impersonates renderer | Peer UID is not renderer identity or authorization; require an agent-issued session/capability for allowed controls | Same-UID client sends privileged/unknown request; deny; demonstrate no permission or adapter call |
| Frame is malformed, oversized, truncated or has trailing bytes | Length-prefixed bounded frames; strict JSON/schema; one complete frame per message | Invalid length, over-limit, invalid UTF-8/JSON, truncated header/body, duplicate keys and trailing data; bounded memory and clean reject |
| Client disconnects during a write or reply | Per-connection state is disposable; canonical operation outcome remains in the store | Disconnect at header/body/reply points; restart renderer and query current state without replaying effects |
| Renderer restarts or sends stale state/control sequence | Session-bound monotonically increasing sequence for IPC controls; stale snapshot revision cannot overwrite newer state | Reconnect, replay prior sequence/revision, out-of-order update; reject stale data and preserve current state |
| Slow reader or message flood exhausts resources | Backpressure, bounded queues, read/write deadlines, per-peer quotas | Stop reading, flood valid/invalid frames, exceed queue cap; verify bounded RSS/FDs and responsiveness of healthy client |
| Network exposure accidentally appears | Bind only `AF_UNIX`; no TCP listener or remote fallback | Inspect open/listening sockets and configuration; assert no TCP listener in test process |
| Renderer message attempts to claim operation success | Agent emits outcome; renderer may request controls only from explicit allowlist | Attempt fabricated success/evidence and forbidden method; ensure UI state derives only from agent snapshot/outcome |

Before implementation, pin the supported Linux baseline and define the frame limit, protocol version negotiation, socket-directory ownership/mode, deadlines, and same-user threat assumptions. Preserve test traces and hashes as IPC-01 evidence; all gates remain Not Run until the complete matrix is executed against the implementation.
