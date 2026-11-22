class_name PresentationIpcClient
extends RefCounted

## Bounded length-prefixed Unix domain socket client for presentation-agent/1
## Reference: docs/p4/ipc/presentation-agent-v1.md

signal connection_established(session_id: String)
signal connection_lost(reason: String)
signal snapshot_received(snapshot: PresentationSnapshot)
signal outcome_received(action_id: String, outcome: ProtocolTypes.OutcomeClass)

enum ClientState {
	DISCONNECTED,
	CONNECTING,
	AWAITING_HELLO_RESPONSE,
	AWAITING_READY_ACKNOWLEDGEMENT,
	ACTIVE
}

var _state: ClientState = ClientState.DISCONNECTED
var _peer: StreamPeerUnix = null
var _socket_path: String = ""
var _session_id: String = ""
var _next_session_sequence: int = 0
var _last_state_revision: int = -1

# Buffer for length-prefixed frame reading
var _read_buffer: PackedByteArray = PackedByteArray()
var _expected_frame_length: int = -1

func _init(socket_path: String = "") -> void:
	if socket_path.is_empty():
		var xdg_runtime := OS.get_environment("XDG_RUNTIME_DIR")
		if xdg_runtime.is_empty():
			xdg_runtime = "/tmp"
		_socket_path = xdg_runtime.path_join("lia/presentation.sock")
	else:
		_socket_path = socket_path

func get_state() -> ClientState:
	return _state

func get_session_id() -> String:
	return _session_id

func is_active() -> bool:
	return _state == ClientState.ACTIVE

## Connects to the local Unix domain socket endpoint
func connect_to_endpoint() -> Error:
	disconnect_from_endpoint("Reconnecting")
	_peer = StreamPeerUnix.new()
	var err := _peer.connect_to_host(_socket_path)
	if err != OK:
		_state = ClientState.DISCONNECTED
		return err

	_state = ClientState.CONNECTING
	_session_id = ""
	_next_session_sequence = 0
	_last_state_revision = -1
	_read_buffer.clear()
	_expected_frame_length = -1

	# Send presentation.hello immediately
	return _send_hello_request()

## Disconnects and resets session invariants
func disconnect_from_endpoint(reason: String) -> void:
	if _peer != null:
		_peer = null
	_state = ClientState.DISCONNECTED
	_session_id = ""
	_next_session_sequence = 0
	_last_state_revision = -1
	_read_buffer.clear()
	_expected_frame_length = -1
	emit_signal("connection_lost", reason)

## Polls the socket for incoming frames. Should be called every frame or idle tick.
func poll() -> void:
	if _peer == null or _state == ClientState.DISCONNECTED:
		return

	var status := _peer.get_status()
	if status != StreamPeerUnix.STATUS_CONNECTED:
		disconnect_from_endpoint("Socket peer disconnected or errored: %d" % status)
		return

	var available := _peer.get_available_bytes()
	if available <= 0:
		return

	var read_chunk := _peer.get_data(available)
	if read_chunk[0] != OK:
		disconnect_from_endpoint("Socket read failed")
		return

	_read_buffer.append_array(read_chunk[1])
	_process_incoming_buffer()

func _process_incoming_buffer() -> void:
	while true:
		if _expected_frame_length < 0:
			# Need at least 4 bytes for big-endian length prefix
			if _read_buffer.size() < 4:
				break
			var len_bytes := _read_buffer.slice(0, 4)
			_expected_frame_length = (len_bytes[0] << 24) | (len_bytes[1] << 16) | (len_bytes[2] << 8) | len_bytes[3]

			if _expected_frame_length <= 0 or _expected_frame_length > ProtocolTypes.MAX_FRAME_BYTES:
				disconnect_from_endpoint("Oversized or zero-length frame rejected: %d bytes" % _expected_frame_length)
				return
			_read_buffer = _read_buffer.slice(4)

		# Wait until full frame payload is available
		if _read_buffer.size() < _expected_frame_length:
			break

		var frame_data := _read_buffer.slice(0, _expected_frame_length)
		_read_buffer = _read_buffer.slice(_expected_frame_length)
		_expected_frame_length = -1

		var json_str := frame_data.get_string_from_utf8()
		if json_str.is_empty() and frame_data.size() > 0:
			disconnect_from_endpoint("Invalid UTF-8 frame data rejected")
			return

		var parse_result = JSON.parse_string(json_str)
		if typeof(parse_result) != TYPE_DICTIONARY:
			disconnect_from_endpoint("Malformed frame JSON rejected")
			return

		_handle_incoming_frame(parse_result)

func _handle_incoming_frame(frame: Dictionary) -> void:
	# Enforce protocol version agreement
	if frame.get("protocol_version") != ProtocolTypes.PROTOCOL_VERSION:
		disconnect_from_endpoint("Protocol version mismatch")
		return

	var kind: String = str(frame.get("kind", ""))
	var method: String = str(frame.get("method", ""))

	match _state:
		ClientState.AWAITING_HELLO_RESPONSE:
			if method == ProtocolTypes.METHOD_HELLO and kind == ProtocolTypes.KIND_RESPONSE:
				var session: String = str(frame.get("session_id", ""))
				if session.is_empty():
					disconnect_from_endpoint("Hello response missing session_id")
					return
				_session_id = session
				_state = ClientState.AWAITING_READY_ACKNOWLEDGEMENT
				_send_ready_request()
			else:
				disconnect_from_endpoint("Expected hello response, received %s/%s" % [kind, method])

		ClientState.AWAITING_READY_ACKNOWLEDGEMENT, ClientState.ACTIVE:
			# Verify session binding
			if frame.get("session_id") != _session_id:
				disconnect_from_endpoint("Cross-session frame rejected")
				return

			if method == ProtocolTypes.METHOD_STATE_REPLACE and kind == ProtocolTypes.KIND_EVENT:
				_handle_state_replace(frame.get("body", {}))
			elif method == ProtocolTypes.METHOD_ACTION_PLAY and kind == ProtocolTypes.KIND_EVENT:
				_handle_action_play(frame.get("body", {}))
			elif method == ProtocolTypes.METHOD_SHUTDOWN and kind == ProtocolTypes.KIND_EVENT:
				disconnect_from_endpoint("Received agent shutdown event")
			else:
				# Other valid contract events or telemetry
				pass

func _handle_state_replace(body: Dictionary) -> void:
	var snapshot := PresentationSnapshot.from_dict(body)
	if snapshot == null:
		push_error("PresentationIpcClient: Failed to parse state.replace snapshot")
		return

	# Monotonic state revision check per live session
	if snapshot.state_revision <= _last_state_revision:
		push_warning("PresentationIpcClient: Stale state revision %d <= %d rejected" % [
			snapshot.state_revision, _last_state_revision
		])
		return

	_last_state_revision = snapshot.state_revision

	if _state == ClientState.AWAITING_READY_ACKNOWLEDGEMENT:
		_state = ClientState.ACTIVE
		emit_signal("connection_established", _session_id)

	emit_signal("snapshot_received", snapshot)

func _handle_action_play(body: Dictionary) -> void:
	var action_id: String = str(body.get("action_id", ""))
	if action_id.is_empty() or not ProtocolTypes.is_valid_identifier(action_id):
		return

	var outcome: ProtocolTypes.OutcomeClass = ProtocolTypes.OutcomeClass.UNKNOWN_OUTCOME
	if body.has("outcome"):
		match str(body["outcome"]):
			"verified_success": outcome = ProtocolTypes.OutcomeClass.VERIFIED_SUCCESS
			"verified_failure": outcome = ProtocolTypes.OutcomeClass.VERIFIED_FAILURE
			"denied": outcome = ProtocolTypes.OutcomeClass.DENIED
			"unsupported": outcome = ProtocolTypes.OutcomeClass.UNSUPPORTED
			_: outcome = ProtocolTypes.OutcomeClass.UNKNOWN_OUTCOME

	emit_signal("outcome_received", action_id, outcome)

func _send_hello_request() -> Error:
	var frame := {
		"protocol_version": ProtocolTypes.PROTOCOL_VERSION,
		"message_id": ProtocolTypes.generate_uuid(),
		"kind": ProtocolTypes.KIND_REQUEST,
		"method": ProtocolTypes.METHOD_HELLO,
		"sent_at": ProtocolTypes.get_iso_timestamp(),
		"deadline_at": null,
		"body": {
			"protocol_version": ProtocolTypes.PROTOCOL_VERSION
		}
	}
	_state = ClientState.AWAITING_HELLO_RESPONSE
	return _send_frame(frame)

func _send_ready_request() -> Error:
	var frame := {
		"protocol_version": ProtocolTypes.PROTOCOL_VERSION,
		"message_id": ProtocolTypes.generate_uuid(),
		"session_id": _session_id,
		"kind": ProtocolTypes.KIND_REQUEST,
		"method": ProtocolTypes.METHOD_READY,
		"sent_at": ProtocolTypes.get_iso_timestamp(),
		"deadline_at": null,
		"body": {
			"ready": true
		}
	}
	return _send_frame(frame)

## Sends a typed user event with monotonic session_sequence
func send_user_event(control_id: String, event_name: String, text_content: String = "") -> Error:
	if _state != ClientState.ACTIVE:
		push_error("PresentationIpcClient: Cannot send user event in state %d" % _state)
		return ERR_UNAVAILABLE

	if not ProtocolTypes.is_valid_identifier(control_id):
		push_error("PresentationIpcClient: Invalid control_id: %s" % control_id)
		return ERR_INVALID_PARAMETER

	var body := {
		"control_id": control_id,
		"event": event_name
	}
	if event_name == ProtocolTypes.EVENT_TEXT_SUBMITTED:
		if text_content.to_utf8_buffer().size() > ProtocolTypes.MAX_TEXT_BYTES:
			push_error("PresentationIpcClient: Text exceeds 8192 byte limit")
			return ERR_INVALID_PARAMETER
		body["text"] = text_content

	var seq_str := str(_next_session_sequence)
	_next_session_sequence += 1

	var frame := {
		"protocol_version": ProtocolTypes.PROTOCOL_VERSION,
		"message_id": ProtocolTypes.generate_uuid(),
		"session_id": _session_id,
		"session_sequence": seq_str,
		"kind": ProtocolTypes.KIND_REQUEST,
		"method": ProtocolTypes.METHOD_USER_EVENT,
		"sent_at": ProtocolTypes.get_iso_timestamp(),
		"deadline_at": null,
		"body": body
	}

	return _send_frame(frame)

func _send_frame(frame: Dictionary) -> Error:
	if _peer == null:
		return ERR_CONNECTION_ERROR

	var json_str := JSON.stringify(frame)
	var payload := json_str.to_utf8_buffer()
	var payload_len := payload.size()

	if payload_len == 0 or payload_len > ProtocolTypes.MAX_FRAME_BYTES:
		return ERR_INVALID_DATA

	var header := PackedByteArray([
		(payload_len >> 24) & 0xff,
		(payload_len >> 16) & 0xff,
		(payload_len >> 8) & 0xff,
		payload_len & 0xff
	])

	var full_data := PackedByteArray()
	full_data.append_array(header)
	full_data.append_array(payload)

	return _peer.put_data(full_data)
