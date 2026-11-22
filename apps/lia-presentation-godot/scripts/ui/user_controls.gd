class_name UserControls
extends RefCounted

## Approved user controls implementing canonical AC-010 bindings
## Reference: docs/p4/ipc/presentation-agent-v1.md §4.1 and AC-010

signal intent_emitted(control_id: String, event_name: String, text_content: String)

var _ipc_client: PresentationIpcClient = null
var _state_machine: PresentationStateMachine = null

func _init(ipc_client: PresentationIpcClient, state_machine: PresentationStateMachine) -> void:
	_ipc_client = ipc_client
	_state_machine = state_machine

## AC-010: Hide / minimize character presence
func request_hide() -> Error:
	_state_machine.user_hidden = true
	return _dispatch(ProtocolTypes.CONTROL_PANEL_HIDE, ProtocolTypes.EVENT_CLICKED)

## AC-010: Toggle stay-on-top window pin
func toggle_pin() -> Error:
	_state_machine.user_pinned = not _state_machine.user_pinned
	return _dispatch(ProtocolTypes.CONTROL_PANEL_PIN, ProtocolTypes.EVENT_CLICKED)

## AC-010: Toggle position lock (overrides autonomous repositioning)
func toggle_lock() -> Error:
	_state_machine.user_locked = not _state_machine.user_locked
	return _dispatch(ProtocolTypes.CONTROL_PANEL_LOCK, ProtocolTypes.EVENT_CLICKED)

## AC-010: Request user repositioning
func request_reposition() -> Error:
	return _dispatch(ProtocolTypes.CONTROL_PANEL_REPOSITION, ProtocolTypes.EVENT_CLICKED)

## AC-010: Request pause of character presence / speech
func request_pause() -> Error:
	return _dispatch(ProtocolTypes.CONTROL_PRESENTATION_PAUSE, ProtocolTypes.EVENT_CLICKED)

## AC-010: Request resumption from paused state
func request_resume() -> Error:
	return _dispatch(ProtocolTypes.CONTROL_PRESENTATION_RESUME, ProtocolTypes.EVENT_CLICKED)

## AC-010: Submit text message input
func submit_chat_text(text: String) -> Error:
	if text.strip_edges().is_empty():
		return ERR_INVALID_PARAMETER
	return _dispatch(ProtocolTypes.CONTROL_CHAT_INPUT, ProtocolTypes.EVENT_TEXT_SUBMITTED, text)

## AC-010: Request immediate interruption of active speech or action
func request_stop() -> Error:
	return _dispatch(ProtocolTypes.CONTROL_CHAT_STOP, ProtocolTypes.EVENT_INTERRUPTION_REQUESTED)

func _dispatch(control_id: String, event_name: String, text_content: String = "") -> Error:
	emit_signal("intent_emitted", control_id, event_name, text_content)
	if _ipc_client != null and _ipc_client.is_active():
		return _ipc_client.send_user_event(control_id, event_name, text_content)
	return ERR_UNAVAILABLE
