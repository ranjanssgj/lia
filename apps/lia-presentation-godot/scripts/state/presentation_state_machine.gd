class_name PresentationStateMachine
extends RefCounted

## Character lifecycle and truthful outcome state machine
## Implements AC-001, AC-007, and AC-010

signal state_changed(old_state: ProtocolTypes.PresentationState, new_state: ProtocolTypes.PresentationState)
signal outcome_displayed(outcome: ProtocolTypes.OutcomeClass, action_id: String)
signal emotion_updated(emotion_id: String, intensity: float)
signal pose_updated(pose_id: String)
signal attention_cue_triggered(cue_id: String)

var current_state: ProtocolTypes.PresentationState = ProtocolTypes.PresentationState.DISCONNECTED
var current_emotion_id: String = "neutral"
var current_emotion_intensity: float = 0.0
var current_pose_id: String = "idle"
var current_attention_cue: String = ""

# Placement precedence invariants (AC-010)
var user_locked: bool = false
var user_pinned: bool = false
var user_hidden: bool = false

func handle_snapshot(snapshot: PresentationSnapshot) -> void:
	if snapshot == null:
		return

	# Update emotional and pose state
	if current_emotion_id != snapshot.emotion_id or not is_equal_approx(current_emotion_intensity, snapshot.emotion_intensity):
		current_emotion_id = snapshot.emotion_id
		current_emotion_intensity = snapshot.emotion_intensity
		emit_signal("emotion_updated", current_emotion_id, current_emotion_intensity)

	if current_pose_id != snapshot.pose_id:
		current_pose_id = snapshot.pose_id
		emit_signal("pose_updated", current_pose_id)

	if snapshot.attention_cue != current_attention_cue:
		current_attention_cue = snapshot.attention_cue
		if not current_attention_cue.is_empty():
			emit_signal("attention_cue_triggered", current_attention_cue)

	# Determine active lifecycle state
	var new_state := ProtocolTypes.PresentationState.IDLE
	if snapshot.paused:
		new_state = ProtocolTypes.PresentationState.PAUSED
	elif snapshot.speaking:
		new_state = ProtocolTypes.PresentationState.SPEAKING
	elif snapshot.listening:
		new_state = ProtocolTypes.PresentationState.LISTENING

	_transition_to(new_state)

## Displays truthful outcome (AC-007):
## Strictly distinguishes VerifiedSuccess, VerifiedFailure, UnknownOutcome, Denied, Unsupported.
## Never depicts optimistic success upon dispatch.
func handle_outcome(action_id: String, outcome: ProtocolTypes.OutcomeClass) -> void:
	emit_signal("outcome_displayed", outcome, action_id)

func handle_connection_lost() -> void:
	current_emotion_id = "neutral"
	current_emotion_intensity = 0.0
	current_pose_id = "idle"
	current_attention_cue = ""
	_transition_to(ProtocolTypes.PresentationState.DISCONNECTED)

func handle_connecting() -> void:
	_transition_to(ProtocolTypes.PresentationState.CONNECTING)

func handle_awaiting_ready() -> void:
	_transition_to(ProtocolTypes.PresentationState.AWAITING_READY)

func _transition_to(new_state: ProtocolTypes.PresentationState) -> void:
	if current_state == new_state:
		return
	var old_state := current_state
	current_state = new_state
	emit_signal("state_changed", old_state, new_state)
