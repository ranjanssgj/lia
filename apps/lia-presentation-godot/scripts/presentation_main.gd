class_name PresentationMain
extends Node

## Root coordinator for the Godot presentation application
## Reference: docs/p5/tasks/M4-godot-presentation.md

@onready var character_view: CharacterView = $CharacterView

var ipc_client: PresentationIpcClient
var state_machine: PresentationStateMachine
var user_controls: UserControls

func _ready() -> void:
	state_machine = PresentationStateMachine.new()
	ipc_client = PresentationIpcClient.new()
	user_controls = UserControls.new(ipc_client, state_machine)

	ipc_client.snapshot_received.connect(state_machine.handle_snapshot)
	ipc_client.outcome_received.connect(state_machine.handle_outcome)
	ipc_client.connection_lost.connect(_on_connection_lost)

	if character_view:
		character_view.setup(state_machine, user_controls)

	_attempt_connection()

func _process(_delta: float) -> void:
	if ipc_client:
		ipc_client.poll()

func _attempt_connection() -> void:
	state_machine.handle_connecting()
	var err := ipc_client.connect_to_endpoint()
	if err != OK:
		state_machine.handle_connection_lost()

func _on_connection_lost(reason: String) -> void:
	push_warning("PresentationMain: Disconnected from device agent: %s" % reason)
	state_machine.handle_connection_lost()
	# Retry connection after cooldown
	await get_tree().create_timer(1.0).timeout
	_attempt_connection()
