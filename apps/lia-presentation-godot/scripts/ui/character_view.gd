class_name CharacterView
extends Control

## Visual representation controller for LIA presentation surface
## Implements AC-001, AC-007, and AC-010 visual behaviors

@export var character_name: String = "Lia"

var _state_machine: PresentationStateMachine
var _user_controls: UserControls
var _is_dragging: bool = false
var _drag_start_position: Vector2 = Vector2.ZERO

func setup(state_machine: PresentationStateMachine, user_controls: UserControls) -> void:
	_state_machine = state_machine
	_user_controls = user_controls

	_state_machine.state_changed.connect(_on_state_changed)
	_state_machine.outcome_displayed.connect(_on_outcome_displayed)
	_state_machine.emotion_updated.connect(_on_emotion_updated)
	_state_machine.pose_updated.connect(_on_pose_updated)
	_state_machine.attention_cue_triggered.connect(_on_attention_cue)

func _gui_input(event: InputEvent) -> void:
	# User drag/reposition handling with placement precedence (AC-010)
	if event is InputEventMouseButton:
		var mb := event as InputEventMouseButton
		if mb.button_index == MOUSE_BUTTON_LEFT:
			if mb.pressed:
				# Cannot drag if user has locked position
				if _state_machine.user_locked:
					return
				_is_dragging = true
				_drag_start_position = mb.global_position
			else:
				if _is_dragging:
					_is_dragging = false
					_user_controls.request_reposition()
	elif event is InputEventMouseMotion and _is_dragging:
		var mm := event as InputEventMouseMotion
		var window := get_window()
		if window:
			window.position += Vector2i(mm.relative)

func _on_state_changed(old_state: ProtocolTypes.PresentationState, new_state: ProtocolTypes.PresentationState) -> void:
	match new_state:
		ProtocolTypes.PresentationState.DISCONNECTED:
			modulate = Color(0.6, 0.6, 0.6, 0.8) # Degraded / disconnected tint
		ProtocolTypes.PresentationState.IDLE:
			modulate = Color(1.0, 1.0, 1.0, 1.0)
		ProtocolTypes.PresentationState.LISTENING:
			modulate = Color(0.9, 0.95, 1.0, 1.0) # Attentive highlight
		ProtocolTypes.PresentationState.SPEAKING:
			modulate = Color(1.0, 1.0, 0.95, 1.0) # Conversational warmth
		ProtocolTypes.PresentationState.PAUSED:
			modulate = Color(0.7, 0.7, 0.8, 0.6) # Dimmed paused state

func _on_outcome_displayed(outcome: ProtocolTypes.OutcomeClass, action_id: String) -> void:
	# Truthful outcome visualization: Strictly distinguishable indicators (AC-007)
	match outcome:
		ProtocolTypes.OutcomeClass.VERIFIED_SUCCESS:
			# Affirmative, confident visual feedback
			_flash_cue(Color(0.2, 0.8, 0.3, 1.0))
		ProtocolTypes.OutcomeClass.VERIFIED_FAILURE:
			# Explicit non-success indicator
			_flash_cue(Color(0.85, 0.3, 0.2, 1.0))
		ProtocolTypes.OutcomeClass.UNKNOWN_OUTCOME:
			# Visual caution, strictly distinguished from success
			_flash_cue(Color(0.9, 0.7, 0.1, 1.0))
		ProtocolTypes.OutcomeClass.DENIED:
			# Polite refusal cue
			_flash_cue(Color(0.7, 0.4, 0.7, 1.0))
		ProtocolTypes.OutcomeClass.UNSUPPORTED:
			# Neutral incapacity indicator
			_flash_cue(Color(0.5, 0.5, 0.6, 1.0))

func _on_emotion_updated(emotion_id: String, intensity: float) -> void:
	# Applies facial / shader expression parameters
	pass

func _on_pose_updated(pose_id: String) -> void:
	# Triggers character skeleton pose animation
	pass

func _on_attention_cue(cue_id: String) -> void:
	# Visual focus indicator
	pass

func _flash_cue(color: Color) -> void:
	var tween := create_tween()
	if tween:
		tween.tween_property(self, "modulate", color, 0.15)
		tween.tween_property(self, "modulate", Color.WHITE, 0.35)
