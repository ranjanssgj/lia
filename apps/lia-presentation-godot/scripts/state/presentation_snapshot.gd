class_name PresentationSnapshot
extends RefCounted

## Strict parser and representation of presentation.state.replace
## Reference: docs/p4/ipc/presentation-agent-v1.md §4.1

var state_revision: int = 0
var character_id: String = ""
var emotion_id: String = "neutral"
var emotion_intensity: float = 0.0
var pose_id: String = "idle"
var speaking: bool = false
var listening: bool = false
var paused: bool = false
var gaze_target_class: String = ""
var attention_cue: String = ""

const MAX_SAFE_INTEGER: int = 9007199254740991

static func from_dict(dict: Dictionary) -> PresentationSnapshot:
	# Enforce strictly known fields
	var allowed_keys := {
		"state_revision": true,
		"character_id": true,
		"emotion": true,
		"pose_id": true,
		"speaking": true,
		"listening": true,
		"paused": true,
		"gaze_target_class": true,
		"attention_cue": true
	}
	for k in dict.keys():
		if not allowed_keys.has(k):
			push_error("PresentationSnapshot: unknown field rejected: %s" % str(k))
			return null

	# Validate state_revision
	if not dict.has("state_revision") or typeof(dict["state_revision"]) != TYPE_INT and typeof(dict["state_revision"]) != TYPE_FLOAT:
		return null
	var rev: int = int(dict["state_revision"])
	if rev < 0 or rev > MAX_SAFE_INTEGER:
		return null

	# Validate character_id
	if not dict.has("character_id") or typeof(dict["character_id"]) != TYPE_STRING:
		return null
	var cid: String = dict["character_id"]
	if not ProtocolTypes.is_valid_identifier(cid):
		return null

	# Validate emotion
	if not dict.has("emotion") or typeof(dict["emotion"]) != TYPE_DICTIONARY:
		return null
	var emo_dict: Dictionary = dict["emotion"]
	for ek in emo_dict.keys():
		if ek != "id" and ek != "intensity":
			return null
	if not emo_dict.has("id") or typeof(emo_dict["id"]) != TYPE_STRING:
		return null
	var emo_id: String = emo_dict["id"]
	if not ProtocolTypes.is_valid_identifier(emo_id):
		return null
	if not emo_dict.has("intensity") or typeof(emo_dict["intensity"]) != TYPE_FLOAT and typeof(emo_dict["intensity"]) != TYPE_INT:
		return null
	var intensity: float = float(emo_dict["intensity"])
	if is_nan(intensity) or is_inf(intensity) or intensity < 0.0 or intensity > 1.0:
		return null

	# Validate pose_id
	if not dict.has("pose_id") or typeof(dict["pose_id"]) != TYPE_STRING:
		return null
	var pid: String = dict["pose_id"]
	if not ProtocolTypes.is_valid_identifier(pid):
		return null

	# Validate booleans
	if not dict.has("speaking") or typeof(dict["speaking"]) != TYPE_BOOL:
		return null
	if not dict.has("listening") or typeof(dict["listening"]) != TYPE_BOOL:
		return null
	if not dict.has("paused") or typeof(dict["paused"]) != TYPE_BOOL:
		return null

	var snap := PresentationSnapshot.new()
	snap.state_revision = rev
	snap.character_id = cid
	snap.emotion_id = emo_id
	snap.emotion_intensity = intensity
	snap.pose_id = pid
	snap.speaking = dict["speaking"]
	snap.listening = dict["listening"]
	snap.paused = dict["paused"]

	# Optional fields
	if dict.has("gaze_target_class") and dict["gaze_target_class"] != null:
		if typeof(dict["gaze_target_class"]) != TYPE_STRING or not ProtocolTypes.is_valid_identifier(dict["gaze_target_class"]):
			return null
		snap.gaze_target_class = dict["gaze_target_class"]

	if dict.has("attention_cue") and dict["attention_cue"] != null:
		if typeof(dict["attention_cue"]) != TYPE_STRING or not ProtocolTypes.is_valid_identifier(dict["attention_cue"]):
			return null
		snap.attention_cue = dict["attention_cue"]

	return snap
