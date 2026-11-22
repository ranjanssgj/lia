class_name ProtocolTypes
extends RefCounted

## Protocol definitions for presentation-agent/1
## Reference: docs/p4/ipc/presentation-agent-v1.md

const PROTOCOL_VERSION: String = "1"
const MAX_FRAME_BYTES: int = 262144 # 256 KiB
const MAX_TEXT_BYTES: int = 8192
const MAX_IDENTIFIER_BYTES: int = 128
const IO_TIMEOUT_SECONDS: float = 1.0

# Method names
const METHOD_HELLO: String = "presentation.hello"
const METHOD_READY: String = "presentation.ready"
const METHOD_USER_EVENT: String = "presentation.user_event"
const METHOD_STATE_REPLACE: String = "presentation.state.replace"
const METHOD_STATE_PATCH: String = "presentation.state.patch"
const METHOD_ACTION_PLAY: String = "presentation.action.play"
const METHOD_AUDIO_STATE: String = "presentation.audio.state"
const METHOD_HEALTH: String = "presentation.health"
const METHOD_SHUTDOWN: String = "presentation.shutdown"

# Message kinds
const KIND_REQUEST: String = "request"
const KIND_RESPONSE: String = "response"
const KIND_EVENT: String = "event"

# User Event types
const EVENT_CLICKED: String = "clicked"
const EVENT_DISMISSED: String = "dismissed"
const EVENT_TEXT_SUBMITTED: String = "text_submitted"
const EVENT_INTERRUPTION_REQUESTED: String = "interruption_requested"

# Canonical AC-010 Control IDs
const CONTROL_PANEL_HIDE: String = "panel.hide"
const CONTROL_PANEL_PIN: String = "panel.pin"
const CONTROL_PANEL_LOCK: String = "panel.lock"
const CONTROL_PANEL_REPOSITION: String = "panel.reposition"
const CONTROL_PRESENTATION_PAUSE: String = "presentation.pause"
const CONTROL_PRESENTATION_RESUME: String = "presentation.resume"
const CONTROL_CHAT_INPUT: String = "chat.input"
const CONTROL_CHAT_STOP: String = "chat.stop"

# Outcome classes
enum OutcomeClass {
	VERIFIED_SUCCESS,
	VERIFIED_FAILURE,
	UNKNOWN_OUTCOME,
	DENIED,
	UNSUPPORTED
}

# Presentation Visual States
enum PresentationState {
	DISCONNECTED,
	CONNECTING,
	AWAITING_READY,
	IDLE,
	LISTENING,
	SPEAKING,
	PAUSED,
	DEGRADED
}

## Validates an ASCII identifier according to contract: 1..128 bytes matching [A-Za-z0-9._:-]+
static func is_valid_identifier(id_str: String) -> bool:
	if id_str.is_empty() or id_str.length() > MAX_IDENTIFIER_BYTES:
		return false
	var regex := RegEx.new()
	regex.compile("^[A-Za-z0-9._:-]+$")
	return regex.search(id_str) != null

## Generates a pseudo-UUIDv4 string for message_id correlation
static func generate_uuid() -> String:
	var b := PackedByteArray()
	b.resize(16)
	for i in range(16):
		b[i] = randi() % 256
	# Set version 4 (bits 12-15 = 0100)
	b[6] = (b[6] & 0x0f) | 0x40
	# Set variant RFC 4122 (bits 6-7 = 10)
	b[8] = (b[8] & 0x3f) | 0x80
	return "%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%02x%02x%02x" % [
		b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
		b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
	]

## Generates RFC3339 UTC timestamp
static func get_iso_timestamp() -> String:
	var dt := Time.get_datetime_dict_from_system(true)
	return "%04d-%02d-%02dT%02d:%02d:%02dZ" % [
		dt["year"], dt["month"], dt["day"],
		dt["hour"], dt["minute"], dt["second"]
	]
