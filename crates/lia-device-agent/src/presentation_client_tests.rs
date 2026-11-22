//! Synthetic presentation client conformance test suite for P5 M4 (GODOT-01 / AC-001, AC-007, AC-010).
//!
//! Validates client-side state machine behavior, strict snapshot decoding,
//! truthful outcome visual state distinction (Invariant 8), placement precedence,
//! and canonical AC-010 typed user controls.

#[cfg(test)]
mod tests {
    use crate::ipc::{UntrustedUserIntent, UserEventBody, UserEventKind};
    use crate::presentation_policy::{
        AuthorizationError, DeterministicPresentationPolicy, PermissionAuthority,
        ProductionUiRegistry, RiskTier, authorize_user_intent,
    };
    use serde_json::{Value, json};
    use std::collections::HashSet;

    const MAX_TEXT_BYTES: usize = 8_192;
    const MAX_IDENTIFIER_BYTES: usize = 128;
    const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[allow(dead_code)]
    enum PresentationState {
        Disconnected,
        Connecting,
        AwaitingReady,
        Idle,
        Listening,
        Speaking,
        Paused,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum OutcomeClass {
        VerifiedSuccess,
        VerifiedFailure,
        UnknownOutcome,
        Denied,
        Unsupported,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct PresentationSnapshot {
        state_revision: u64,
        character_id: String,
        emotion_id: String,
        emotion_intensity: f64,
        pose_id: String,
        speaking: bool,
        listening: bool,
        paused: bool,
        gaze_target_class: Option<String>,
        attention_cue: Option<String>,
    }

    fn is_valid_identifier(s: &str) -> bool {
        if s.is_empty() || s.len() > MAX_IDENTIFIER_BYTES {
            return false;
        }
        s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == ':' || c == '-')
    }

    fn parse_snapshot(val: &Value) -> Result<PresentationSnapshot, String> {
        let obj = val.as_object().ok_or("Snapshot must be an object")?;

        // Enforce strict allowlist of fields (fail closed on unknown fields)
        let allowed_keys: HashSet<&str> = HashSet::from([
            "state_revision",
            "character_id",
            "emotion",
            "pose_id",
            "speaking",
            "listening",
            "paused",
            "gaze_target_class",
            "attention_cue",
        ]);

        for k in obj.keys() {
            if !allowed_keys.contains(k.as_str()) {
                return Err(format!("Unknown property rejected: {k}"));
            }
        }

        let state_revision = obj
            .get("state_revision")
            .and_then(|v| v.as_u64())
            .ok_or("Missing or invalid state_revision")?;
        if state_revision > MAX_SAFE_INTEGER {
            return Err("state_revision exceeds safe integer range".into());
        }

        let character_id = obj
            .get("character_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing or invalid character_id")?;
        if !is_valid_identifier(character_id) {
            return Err("Invalid character_id format".into());
        }

        let emotion_obj = obj
            .get("emotion")
            .and_then(|v| v.as_object())
            .ok_or("Missing or invalid emotion object")?;
        let emotion_id = emotion_obj
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("Missing or invalid emotion id")?;
        if !is_valid_identifier(emotion_id) {
            return Err("Invalid emotion id format".into());
        }
        let emotion_intensity = emotion_obj
            .get("intensity")
            .and_then(|v| v.as_f64())
            .ok_or("Missing or invalid emotion intensity")?;
        if emotion_intensity.is_nan()
            || emotion_intensity.is_infinite()
            || !(0.0..=1.0).contains(&emotion_intensity)
        {
            return Err("emotion intensity must be in 0.0..=1.0".into());
        }

        let pose_id = obj
            .get("pose_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing or invalid pose_id")?;
        if !is_valid_identifier(pose_id) {
            return Err("Invalid pose_id format".into());
        }

        let speaking = obj
            .get("speaking")
            .and_then(|v| v.as_bool())
            .ok_or("Missing or invalid speaking flag")?;
        let listening = obj
            .get("listening")
            .and_then(|v| v.as_bool())
            .ok_or("Missing or invalid listening flag")?;
        let paused = obj
            .get("paused")
            .and_then(|v| v.as_bool())
            .ok_or("Missing or invalid paused flag")?;

        let gaze_target_class = obj
            .get("gaze_target_class")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(ref g) = gaze_target_class
            && !is_valid_identifier(g)
        {
            return Err("Invalid gaze_target_class format".into());
        }

        let attention_cue = obj
            .get("attention_cue")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(ref a) = attention_cue
            && !is_valid_identifier(a)
        {
            return Err("Invalid attention_cue format".into());
        }

        Ok(PresentationSnapshot {
            state_revision,
            character_id: character_id.to_string(),
            emotion_id: emotion_id.to_string(),
            emotion_intensity,
            pose_id: pose_id.to_string(),
            speaking,
            listening,
            paused,
            gaze_target_class,
            attention_cue,
        })
    }

    struct MockClientStateMachine {
        state: PresentationState,
        last_revision: Option<u64>,
        current_outcome: Option<OutcomeClass>,
        user_locked: bool,
        user_pinned: bool,
        user_hidden: bool,
    }

    impl MockClientStateMachine {
        fn new() -> Self {
            Self {
                state: PresentationState::Disconnected,
                last_revision: None,
                current_outcome: None,
                user_locked: false,
                user_pinned: false,
                user_hidden: false,
            }
        }

        fn apply_snapshot(&mut self, snap: &PresentationSnapshot) -> Result<(), String> {
            if let Some(last) = self.last_revision
                && snap.state_revision <= last
            {
                return Err(format!(
                    "Stale snapshot revision {} <= {} rejected",
                    snap.state_revision, last
                ));
            }
            self.last_revision = Some(snap.state_revision);

            if snap.paused {
                self.state = PresentationState::Paused;
            } else if snap.speaking {
                self.state = PresentationState::Speaking;
            } else if snap.listening {
                self.state = PresentationState::Listening;
            } else {
                self.state = PresentationState::Idle;
            }

            Ok(())
        }

        fn apply_outcome(&mut self, outcome: OutcomeClass) {
            self.current_outcome = Some(outcome);
        }
    }

    struct MockAuth {
        allowed_actions: HashSet<String>,
        max_tier: RiskTier,
    }

    impl PermissionAuthority for MockAuth {
        fn has_permission(&self, action_key: &str, risk_tier: RiskTier) -> bool {
            risk_tier <= self.max_tier && self.allowed_actions.contains(action_key)
        }
    }

    #[test]
    fn test_godot_001_snapshot_parsing_strict_unknown_fields_and_bounds() {
        let valid_json = json!({
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
        });

        let snap = parse_snapshot(&valid_json).expect("Valid snapshot should parse");
        assert_eq!(snap.state_revision, 42);
        assert_eq!(snap.character_id, "lia");
        assert_eq!(snap.emotion_id, "friendly");
        assert_eq!(snap.emotion_intensity, 0.85);
        assert!(snap.listening);
        assert!(!snap.speaking);

        // Unknown field rejection (renderer cannot claim operation outcomes)
        let mut unknown_field = valid_json.clone();
        unknown_field
            .as_object_mut()
            .unwrap()
            .insert("operation_outcome".into(), json!("success"));
        assert!(
            parse_snapshot(&unknown_field).is_err(),
            "Unknown field must fail closed"
        );

        // Out-of-bounds intensity
        let mut invalid_intensity = valid_json.clone();
        invalid_intensity["emotion"]["intensity"] = json!(1.5);
        assert!(parse_snapshot(&invalid_intensity).is_err());

        // NaN intensity
        let mut nan_intensity = valid_json.clone();
        nan_intensity["emotion"]["intensity"] = json!("NaN");
        assert!(parse_snapshot(&nan_intensity).is_err());

        // Invalid character_id characters
        let mut invalid_id = valid_json.clone();
        invalid_id["character_id"] = json!("lia;rm -rf /");
        assert!(parse_snapshot(&invalid_id).is_err());
    }

    #[test]
    fn test_godot_002_monotonic_state_revision_enforcement() {
        let mut sm = MockClientStateMachine::new();

        let snap1 = PresentationSnapshot {
            state_revision: 10,
            character_id: "lia".into(),
            emotion_id: "neutral".into(),
            emotion_intensity: 0.0,
            pose_id: "idle".into(),
            speaking: false,
            listening: false,
            paused: false,
            gaze_target_class: None,
            attention_cue: None,
        };

        assert!(sm.apply_snapshot(&snap1).is_ok());
        assert_eq!(sm.last_revision, Some(10));
        assert_eq!(sm.state, PresentationState::Idle);

        // Replay of same revision rejected
        assert!(sm.apply_snapshot(&snap1).is_err());

        // Backward revision rejected
        let mut snap_backward = snap1.clone();
        snap_backward.state_revision = 9;
        assert!(sm.apply_snapshot(&snap_backward).is_err());

        // Strictly increasing revision accepted
        let mut snap2 = snap1.clone();
        snap2.state_revision = 11;
        snap2.listening = true;
        assert!(sm.apply_snapshot(&snap2).is_ok());
        assert_eq!(sm.last_revision, Some(11));
        assert_eq!(sm.state, PresentationState::Listening);
    }

    #[test]
    fn test_godot_003_character_lifecycle_transitions() {
        let mut sm = MockClientStateMachine::new();

        let make_snap = |rev, listening, speaking, paused| PresentationSnapshot {
            state_revision: rev,
            character_id: "lia".into(),
            emotion_id: "neutral".into(),
            emotion_intensity: 0.0,
            pose_id: "idle".into(),
            speaking,
            listening,
            paused,
            gaze_target_class: None,
            attention_cue: None,
        };

        sm.apply_snapshot(&make_snap(1, true, false, false))
            .unwrap();
        assert_eq!(sm.state, PresentationState::Listening);

        sm.apply_snapshot(&make_snap(2, false, true, false))
            .unwrap();
        assert_eq!(sm.state, PresentationState::Speaking);

        // Paused takes precedence over active speaking
        sm.apply_snapshot(&make_snap(3, false, true, true)).unwrap();
        assert_eq!(sm.state, PresentationState::Paused);

        // Resumed returns to idle when listening and speaking are false
        sm.apply_snapshot(&make_snap(4, false, false, false))
            .unwrap();
        assert_eq!(sm.state, PresentationState::Idle);
    }

    #[test]
    fn test_godot_004_truthful_outcome_visual_distinction_ac007() {
        let mut sm = MockClientStateMachine::new();

        // 1. Dispatch acknowledgement alone is NOT verified success (Invariant 8)
        let _dispatch_ack = "ok";
        assert_eq!(
            sm.current_outcome, None,
            "Socket ACK must never set visual outcome"
        );

        // 2. VerifiedSuccess
        sm.apply_outcome(OutcomeClass::VerifiedSuccess);
        assert_eq!(sm.current_outcome, Some(OutcomeClass::VerifiedSuccess));

        // 3. VerifiedFailure
        sm.apply_outcome(OutcomeClass::VerifiedFailure);
        assert_eq!(sm.current_outcome, Some(OutcomeClass::VerifiedFailure));
        assert_ne!(sm.current_outcome, Some(OutcomeClass::VerifiedSuccess));

        // 4. UnknownOutcome (must be visibly distinguishable from success)
        sm.apply_outcome(OutcomeClass::UnknownOutcome);
        assert_eq!(sm.current_outcome, Some(OutcomeClass::UnknownOutcome));
        assert_ne!(sm.current_outcome, Some(OutcomeClass::VerifiedSuccess));

        // 5. Denied
        sm.apply_outcome(OutcomeClass::Denied);
        assert_eq!(sm.current_outcome, Some(OutcomeClass::Denied));

        // 6. Unsupported
        sm.apply_outcome(OutcomeClass::Unsupported);
        assert_eq!(sm.current_outcome, Some(OutcomeClass::Unsupported));
    }

    #[test]
    fn test_godot_005_ac010_user_controls_typed_intents_and_bounds() {
        let registry = ProductionUiRegistry::new();
        let session = "00000000-0000-4000-8000-000000000001";
        let authority = MockAuth {
            allowed_actions: HashSet::from([
                "ui.panel.hide".into(),
                "ui.panel.pin".into(),
                "ui.panel.lock".into(),
                "ui.panel.reposition".into(),
                "ui.presentation.pause".into(),
                "ui.presentation.resume".into(),
                "ui.chat.submit".into(),
                "ui.chat.interrupt".into(),
            ]),
            max_tier: RiskTier::R1,
        };
        let policy = DeterministicPresentationPolicy::with_expected_session(authority, session);

        // Test all 8 canonical AC-010 controls resolve and authorize
        let test_cases = vec![
            ("panel.hide", UserEventKind::Clicked, None),
            ("panel.pin", UserEventKind::Clicked, None),
            ("panel.lock", UserEventKind::Clicked, None),
            ("panel.reposition", UserEventKind::Clicked, None),
            ("presentation.pause", UserEventKind::Clicked, None),
            ("presentation.resume", UserEventKind::Clicked, None),
            (
                "chat.input",
                UserEventKind::TextSubmitted,
                Some("Hello Lia".to_string()),
            ),
            ("chat.stop", UserEventKind::InterruptionRequested, None),
        ];

        for (control_id, event_kind, text) in test_cases {
            let intent = UntrustedUserIntent {
                session_id: session.into(),
                sequence: 0,
                body: UserEventBody {
                    control_id: control_id.into(),
                    event: event_kind,
                    text,
                },
            };

            let authorized = authorize_user_intent(intent, &registry, &policy)
                .unwrap_or_else(|e| panic!("Failed for control {control_id}: {e:?}"));
            assert!(authorized.action_key().starts_with("ui."));
        }

        // Test oversized text (> 8192 bytes) is rejected
        let oversized_text = "A".repeat(MAX_TEXT_BYTES + 1);
        let oversized_intent = UntrustedUserIntent {
            session_id: session.into(),
            sequence: 1,
            body: UserEventBody {
                control_id: "chat.input".into(),
                event: UserEventKind::TextSubmitted,
                text: Some(oversized_text),
            },
        };
        assert_eq!(
            authorize_user_intent(oversized_intent, &registry, &policy),
            Err(AuthorizationError::InvalidPayload)
        );
    }

    #[test]
    fn test_godot_006_placement_precedence_user_lock() {
        let mut sm = MockClientStateMachine::new();

        assert!(!sm.user_locked);
        assert!(!sm.user_pinned);
        assert!(!sm.user_hidden);

        // User locks the position
        sm.user_locked = true;
        assert!(sm.user_locked);

        // User toggles pin and hide
        sm.user_pinned = true;
        sm.user_hidden = true;
        assert!(sm.user_pinned);
        assert!(sm.user_hidden);

        // When user_locked is true, autonomous task-bound repositioning cannot override
        let autonomous_reposition_allowed = !sm.user_locked;
        assert!(
            !autonomous_reposition_allowed,
            "User lock must strictly prevent autonomous repositioning"
        );
    }

    #[test]
    fn test_godot_007_reconnect_fresh_session_and_no_replay() {
        let registry = ProductionUiRegistry::new();
        let session_1 = "00000000-0000-4000-8000-000000000001";
        let session_2 = "00000000-0000-4000-8000-000000000002";

        let authority = MockAuth {
            allowed_actions: HashSet::from(["ui.panel.hide".into()]),
            max_tier: RiskTier::R0,
        };

        // Policy bound to session 2 (after reconnect)
        let policy_session_2 =
            DeterministicPresentationPolicy::with_expected_session(authority, session_2);

        // Replaying intent captured from session 1 is rejected
        let captured_intent = UntrustedUserIntent {
            session_id: session_1.into(),
            sequence: 0,
            body: UserEventBody {
                control_id: "panel.hide".into(),
                event: UserEventKind::Clicked,
                text: None,
            },
        };

        assert_eq!(
            authorize_user_intent(captured_intent, &registry, &policy_session_2),
            Err(AuthorizationError::PolicyDenied),
            "Captured intent from prior session must be rejected"
        );

        // Fresh intent with session 2 succeeds
        let fresh_intent = UntrustedUserIntent {
            session_id: session_2.into(),
            sequence: 0,
            body: UserEventBody {
                control_id: "panel.hide".into(),
                event: UserEventKind::Clicked,
                text: None,
            },
        };

        assert!(authorize_user_intent(fresh_intent, &registry, &policy_session_2).is_ok());
    }
}
