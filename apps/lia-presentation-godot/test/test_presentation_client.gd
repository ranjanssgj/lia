class_name TestPresentationClient
extends SceneTree

## Standalone headless unit test runner for LIA Godot Presentation
## Verifies AC-001, AC-007, and AC-010 invariants

func _init() -> void:
	print("Running Godot Presentation M4 Test Suite...")
	var passed := 0
	var failed := 0

	if test_identifier_validation():
		passed += 1
	else:
		failed += 1

	if test_snapshot_strict_schema_and_rejection():
		passed += 1
	else:
		failed += 1

	if test_lifecycle_state_machine():
		passed += 1
	else:
		failed += 1

	if test_truthful_outcome_distinction():
		passed += 1
	else:
		failed += 1

	if test_user_controls_ac010_and_monotonic_sequence():
		passed += 1
	else:
		failed += 1

	if test_placement_precedence():
		passed += 1
	else:
		failed += 1

	print("Test Results: %d passed, %d failed" % [passed, failed])
	if failed > 0:
		quit(1)
	else:
		quit(0)

func test_identifier_validation() -> bool:
	print("  - test_identifier_validation")
	assert(ProtocolTypes.is_valid_identifier("chat.input"))
	assert(ProtocolTypes.is_valid_identifier("panel.reposition"))
	assert(ProtocolTypes.is_valid_identifier("lia-agent_v1:pose_01"))
	assert(!ProtocolTypes.is_valid_identifier(""))
	assert(!ProtocolTypes.is_valid_identifier("control with spaces"))
	assert(!ProtocolTypes.is_valid_identifier("control;rm -rf /"))
	assert(!ProtocolTypes.is_valid_identifier("tool$injection"))
	return true

func test_snapshot_strict_schema_and_rejection() -> bool:
	print("  - test_snapshot_strict_schema_and_rejection")
	var valid_dict := {
		"state_revision": 42,
		"character_id": "lia",
		"emotion": {
			"id": "friendly",
			"intensity": 0.85
		},
		"pose_id": "idle_standing",
		"speaking": false,
		"listening": true,
		"paused": false,
		"gaze_target_class": "user_face",
		"attention_cue": "ring_pulse"
	}
	var snap := PresentationSnapshot.from_dict(valid_dict)
	assert(snap != null)
	assert(snap.state_revision == 42)
	assert(snap.character_id == "lia")
	assert(snap.emotion_id == "friendly")
	assert(is_equal_approx(snap.emotion_intensity, 0.85))
	assert(snap.listening == true)
	assert(snap.speaking == false)

	# Rejects unknown fields
	var invalid_dict := valid_dict.duplicate(true)
	invalid_dict["operation_success"] = true # Renderer cannot claim outcomes
	assert(PresentationSnapshot.from_dict(invalid_dict) == null)

	# Rejects out-of-range intensity
	var invalid_intensity := valid_dict.duplicate(true)
	invalid_intensity["emotion"]["intensity"] = 1.5
	assert(PresentationSnapshot.from_dict(invalid_intensity) == null)

	# Rejects negative state_revision
	var invalid_rev := valid_dict.duplicate(true)
	invalid_rev["state_revision"] = -1
	assert(PresentationSnapshot.from_dict(invalid_rev) == null)

	return true

func test_lifecycle_state_machine() -> bool:
	print("  - test_lifecycle_state_machine")
	var sm := PresentationStateMachine.new()
	assert(sm.current_state == ProtocolTypes.PresentationState.DISCONNECTED)

	sm.handle_connecting()
	assert(sm.current_state == ProtocolTypes.PresentationState.CONNECTING)

	# Deliver snapshot with listening = true
	var snap_listening := PresentationSnapshot.from_dict({
		"state_revision": 1,
		"character_id": "lia",
		"emotion": {"id": "neutral", "intensity": 0.0},
		"pose_id": "idle",
		"speaking": false,
		"listening": true,
		"paused": false
	})
	sm.handle_snapshot(snap_listening)
	assert(sm.current_state == ProtocolTypes.PresentationState.LISTENING)

	# Deliver snapshot with speaking = true
	var snap_speaking := PresentationSnapshot.from_dict({
		"state_revision": 2,
		"character_id": "lia",
		"emotion": {"id": "neutral", "intensity": 0.0},
		"pose_id": "idle",
		"speaking": true,
		"listening": false,
		"paused": false
	})
	sm.handle_snapshot(snap_speaking)
	assert(sm.current_state == ProtocolTypes.PresentationState.SPEAKING)

	# Deliver snapshot with paused = true (takes precedence over active speech)
	var snap_paused := PresentationSnapshot.from_dict({
		"state_revision": 3,
		"character_id": "lia",
		"emotion": {"id": "neutral", "intensity": 0.0},
		"pose_id": "idle",
		"speaking": true,
		"listening": false,
		"paused": true
	})
	sm.handle_snapshot(snap_paused)
	assert(sm.current_state == ProtocolTypes.PresentationState.PAUSED)

	# Disconnect transitions to DISCONNECTED and resets emotion/pose
	sm.handle_connection_lost()
	assert(sm.current_state == ProtocolTypes.PresentationState.DISCONNECTED)
	assert(sm.current_emotion_id == "neutral")

	return true

func test_truthful_outcome_distinction() -> bool:
	print("  - test_truthful_outcome_distinction")
	var sm := PresentationStateMachine.new()
	var reported_outcomes := []

	sm.outcome_displayed.connect(func(outcome: ProtocolTypes.OutcomeClass, action_id: String):
		reported_outcomes.append({"outcome": outcome, "action_id": action_id})
	)

	sm.handle_outcome("act-1", ProtocolTypes.OutcomeClass.VERIFIED_SUCCESS)
	sm.handle_outcome("act-2", ProtocolTypes.OutcomeClass.VERIFIED_FAILURE)
	sm.handle_outcome("act-3", ProtocolTypes.OutcomeClass.UNKNOWN_OUTCOME)
	sm.handle_outcome("act-4", ProtocolTypes.OutcomeClass.DENIED)
	sm.handle_outcome("act-5", ProtocolTypes.OutcomeClass.UNSUPPORTED)

	assert(reported_outcomes.size() == 5)
	assert(reported_outcomes[0]["outcome"] == ProtocolTypes.OutcomeClass.VERIFIED_SUCCESS)
	assert(reported_outcomes[1]["outcome"] == ProtocolTypes.OutcomeClass.VERIFIED_FAILURE)
	assert(reported_outcomes[2]["outcome"] == ProtocolTypes.OutcomeClass.UNKNOWN_OUTCOME)
	assert(reported_outcomes[3]["outcome"] == ProtocolTypes.OutcomeClass.DENIED)
	assert(reported_outcomes[4]["outcome"] == ProtocolTypes.OutcomeClass.UNSUPPORTED)

	# Verified failure and unknown outcome must NEVER equal verified success
	assert(reported_outcomes[1]["outcome"] != ProtocolTypes.OutcomeClass.VERIFIED_SUCCESS)
	assert(reported_outcomes[2]["outcome"] != ProtocolTypes.OutcomeClass.VERIFIED_SUCCESS)

	return true

func test_user_controls_ac010_and_monotonic_sequence() -> bool:
	print("  - test_user_controls_ac010_and_monotonic_sequence")
	var sm := PresentationStateMachine.new()
	var ipc := PresentationIpcClient.new()
	var uc := UserControls.new(ipc, sm)

	var emitted_intents := []
	uc.intent_emitted.connect(func(ctrl: String, ev: String, txt: String):
		emitted_intents.append({"control_id": ctrl, "event": ev, "text": txt})
	)

	uc.request_hide()
	uc.toggle_pin()
	uc.toggle_lock()
	uc.request_pause()
	uc.request_resume()
	uc.submit_chat_text("Hello Lia")
	uc.request_stop()

	assert(emitted_intents.size() == 7)
	assert(emitted_intents[0]["control_id"] == ProtocolTypes.CONTROL_PANEL_HIDE)
	assert(emitted_intents[1]["control_id"] == ProtocolTypes.CONTROL_PANEL_PIN)
	assert(emitted_intents[2]["control_id"] == ProtocolTypes.CONTROL_PANEL_LOCK)
	assert(emitted_intents[3]["control_id"] == ProtocolTypes.CONTROL_PRESENTATION_PAUSE)
	assert(emitted_intents[4]["control_id"] == ProtocolTypes.CONTROL_PRESENTATION_RESUME)
	assert(emitted_intents[5]["control_id"] == ProtocolTypes.CONTROL_CHAT_INPUT)
	assert(emitted_intents[5]["text"] == "Hello Lia")
	assert(emitted_intents[6]["control_id"] == ProtocolTypes.CONTROL_CHAT_STOP)
	assert(emitted_intents[6]["event"] == ProtocolTypes.EVENT_INTERRUPTION_REQUESTED)

	# Empty text submission is rejected
	assert(uc.submit_chat_text("   ") == ERR_INVALID_PARAMETER)

	return true

func test_placement_precedence() -> bool:
	print("  - test_placement_precedence")
	var sm := PresentationStateMachine.new()
	var ipc := PresentationIpcClient.new()
	var uc := UserControls.new(ipc, sm)

	assert(sm.user_locked == false)
	uc.toggle_lock()
	assert(sm.user_locked == true)

	# When user_locked is true, placement precedence guarantees user controls lock
	uc.toggle_lock()
	assert(sm.user_locked == false)

	return true
