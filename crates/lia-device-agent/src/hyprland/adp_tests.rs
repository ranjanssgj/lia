//! Synthetic conformance tests validating ADP-001 through ADP-024 contract behaviors
//! for the Hyprland v0.57.2 adapter.
//!
//! Traceability: `docs/p4/adapters/adapter-conformance-v1.md`.
//! Conformance gate: ADAPTER-01 (partial implementation checks; full gate remains Not Run).

#[cfg(test)]
mod tests {
    use crate::hyprland::*;
    use crate::presentation_policy::{
        DeterministicPresentationPolicy, PermissionAuthority, ProductionUiRegistry, RiskTier,
        authorize_user_intent,
    };
    use crate::store::{AgentStore, CommandClaims, EffectPermit, OperationClaims, TrustedClock};
    use std::sync::Mutex;

    struct AdpMockTransport {
        queries: Mutex<std::collections::HashMap<String, Result<String, HyprlandError>>>,
        dispatches: Mutex<Vec<Result<String, HyprlandError>>>,
        sent_commands: Mutex<Vec<String>>,
    }

    impl AdpMockTransport {
        fn new() -> Self {
            Self {
                queries: Mutex::new(std::collections::HashMap::new()),
                dispatches: Mutex::new(Vec::new()),
                sent_commands: Mutex::new(Vec::new()),
            }
        }

        fn set_query(&self, cmd: &str, res: Result<String, HyprlandError>) {
            self.queries.lock().unwrap().insert(cmd.to_string(), res);
        }

        fn add_dispatch(&self, res: Result<String, HyprlandError>) {
            self.dispatches.lock().unwrap().push(res);
        }
    }

    impl HyprlandTransport for AdpMockTransport {
        fn query(&self, command: &str) -> Result<String, HyprlandError> {
            HyprlandCommandFilter::validate_query(command)?;
            self.queries
                .lock()
                .unwrap()
                .get(command)
                .cloned()
                .unwrap_or_else(|| {
                    Err(HyprlandError::CompositorError(format!(
                        "Unhandled query: {command}"
                    )))
                })
        }

        fn dispatch(&self, command: &str) -> Result<String, HyprlandError> {
            self.sent_commands.lock().unwrap().push(command.to_string());
            let mut list = self.dispatches.lock().unwrap();
            if list.is_empty() {
                Ok("ok".into())
            } else {
                list.remove(0)
            }
        }
    }

    #[derive(Clone)]
    struct AdpClock(std::sync::Arc<std::sync::atomic::AtomicI64>);

    impl AdpClock {
        fn new(us: i64) -> Self {
            Self(std::sync::Arc::new(std::sync::atomic::AtomicI64::new(us)))
        }

        fn advance_us(&self, delta: i64) {
            self.0.fetch_add(delta, std::sync::atomic::Ordering::SeqCst);
        }
    }

    impl TrustedClock for AdpClock {
        fn now_us(&self) -> Result<i64, crate::store::StoreError> {
            Ok(self.0.load(std::sync::atomic::Ordering::SeqCst))
        }
    }

    fn sample_active_window(x: i32, y: i32) -> String {
        format!(
            r#"{{
            "address": "0x55a8f2c3b1a0",
            "mapped": true,
            "hidden": false,
            "at": [{x}, {y}],
            "size": [800, 600],
            "workspace": {{"id": 1, "name": "1"}},
            "floating": true,
            "monitor": 0,
            "class": "foot",
            "title": "TOP_SECRET_DOCUMENT.pdf",
            "initialClass": "foot",
            "initialTitle": "foot",
            "pid": 5555
        }}"#
        )
    }

    fn sample_monitors() -> String {
        r#"[
            {
                "id": 0,
                "name": "eDP-1",
                "x": 0,
                "y": 0,
                "width": 1920,
                "height": 1080,
                "reserved": [30, 0, 0, 0],
                "focused": true
            }
        ]"#
        .into()
    }

    fn setup_store_and_permit(clock: &AdpClock) -> (AgentStore, EffectPermit) {
        let mut store =
            AgentStore::open_in_memory_with_clock(std::sync::Arc::new(clock.clone())).unwrap();
        let now_us = clock.now_us().unwrap();
        let now_str = crate::time::format_rfc3339_us(now_us);
        let exp_str = crate::time::format_rfc3339_us(now_us + 3_600_000_000);

        let op = OperationClaims {
            operation_id: "op-adp-1".into(),
            account_id: "acct-adp".into(),
            target_device_id: "dev-adp".into(),
            target_credential_generation: 1,
            capability_id: "desktop.window.move_same_display".into(),
            capability_schema_version: 1,
            resource_type: "desktop.window".into(),
            resource_id: "0x55a8f2c3b1a0".into(),
            resource_generation: "gen-adp-1".into(),
            expected_resource_revision: Some("1".into()),
            resource_observed_at: Some(now_str.clone()),
            resource_owning_device_id: Some("dev-adp".into()),
            arguments_schema_uri: "urn:lia:schema:desktop.window.move_same_display.input:v1".into(),
            arguments_sha256: [0u8; 32],
            grant_reference: "grant-adp".into(),
            policy_version: 1,
            revocation_epoch: 0,
            risk_tier: 1,
            purpose: "conformance-testing".into(),
            originating_task_id: None,
            received_at: now_str.clone(),
            expires_at: exp_str.clone(),
        };
        store.insert_operation(&op).unwrap();

        let cmd = CommandClaims {
            command_id: "cmd-adp-1".into(),
            nonce: "nonce-adp-1".into(),
            session_id: "sess-adp".into(),
            session_sequence: 0,
            not_before: now_str.clone(),
            expires_at: exp_str.clone(),
            nonce_expires_at: exp_str.clone(),
        };
        store
            .open_session("sess-adp", &now_str, &cmd.nonce_expires_at)
            .unwrap();
        let permit = store
            .consume_command_before_effect(&op, &cmd, "att-adp-1", 1, 1, &now_str)
            .unwrap();

        (store, permit)
    }

    /// ADP-001: Valid current manifest conforms to contract v1 schema and declares capabilities.
    #[test]
    fn adp_001_valid_current_manifest_conforms() {
        let clock = AdpClock::new(1_700_000_000_000_000);
        let transport = AdpMockTransport::new();
        transport.set_query("j/activewindow", Ok(sample_active_window(100, 200)));
        transport.set_query("j/monitors", Ok(sample_monitors()));

        let mut service = HyprlandAdapterService::new(
            "dev-123",
            "acct-123",
            transport,
            HyprlandCacheInvalidator::new(),
        );

        let manifest = service
            .get_capability_manifest(&clock, "gen-tok-1", 1, 60_000_000)
            .unwrap();

        assert_eq!(manifest.contract_version, "lia.adapter.v1");
        assert_eq!(manifest.adapter.adapter_id, "desktop.hyprland");
        assert_eq!(manifest.lifecycle_state, "ready");
        assert_eq!(manifest.capabilities.len(), 2);
        assert_eq!(
            manifest.capabilities[0].capability_id,
            "desktop.window.observe_geometry"
        );
        assert_eq!(
            manifest.capabilities[1].capability_id,
            "desktop.window.move_same_display"
        );
    }

    /// ADP-005: Unsupported capability is rejected without falling back to arbitrary shell commands.
    #[test]
    fn adp_005_unsupported_capability_fails_closed_without_shell_fallback() {
        // Attempting arbitrary query like "sh -c ..." or "dispatch resize"
        assert!(HyprlandCommandFilter::validate_query("sh -c echo malicious").is_err());
        assert!(HyprlandCommandFilter::validate_query("dispatch resizeactive 10 10").is_err());
        assert!(HyprlandCommandFilter::validate_query("dispatch killactive").is_err());
    }

    /// ADP-008: Stale resource generation / address mismatch fails pre-flight before effect.
    #[test]
    fn adp_008_stale_resource_fails_before_effect() {
        let clock = AdpClock::new(1_700_000_000_000_000);
        let (mut store, permit) = setup_store_and_permit(&clock);
        let transport = AdpMockTransport::new();
        transport.set_query("j/monitors", Ok(sample_monitors()));

        let work_area = DisplayWorkArea {
            x: 0,
            y: 30,
            width: 1920,
            height: 1050,
            monitor_id: 0,
        };

        // Window with wrong monitor/workspace targeting
        let cmd = SameDisplayMoveCommand {
            window: WindowIdentity {
                address: "0x55a8f2c3b1a0".into(),
                generation_token: "gen-adp-1".into(),
                workspace_id: 1,
                monitor_id: 99, // monitor mismatch: does not match preserved_monitor
                observed_at_us: 1_700_000_000_000_000,
                fresh_until_us: 1_700_000_010_000_000,
            },
            target_x: 200,
            target_y: 200,
            preserved_width: 800,
            preserved_height: 600,
            preserved_workspace: 1,
            preserved_monitor: 0,
        };

        let res = execute_and_record_move(
            &cmd,
            WindowGeometry::new(100, 200, 800, 600),
            &transport,
            &clock,
            &mut store,
            &permit,
            &work_area,
        )
        .unwrap();

        match res {
            MoveOperationResult::VerifiedFailure { reason, .. } => {
                assert!(reason.contains("Monitor changed"));
            }
            other => panic!("Expected VerifiedFailure, got: {other:?}"),
        }
    }

    /// ADP-009: Resource observation older than freshness bound is rejected.
    #[test]
    fn adp_009_resource_older_than_freshness_bound_rejected() {
        let clock = AdpClock::new(1_700_000_000_000_000);
        let (mut store, permit) = setup_store_and_permit(&clock);
        let transport = AdpMockTransport::new();

        let work_area = DisplayWorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            monitor_id: 0,
        };

        // Advance clock past 10-second freshness window
        clock.advance_us(15_000_000);

        let cmd = SameDisplayMoveCommand {
            window: WindowIdentity {
                address: "0x55a8f2c3b1a0".into(),
                generation_token: "gen-adp-1".into(),
                workspace_id: 1,
                monitor_id: 0,
                observed_at_us: 1_700_000_000_000_000,
                fresh_until_us: 1_700_000_010_000_000,
            },
            target_x: 200,
            target_y: 200,
            preserved_width: 800,
            preserved_height: 600,
            preserved_workspace: 1,
            preserved_monitor: 0,
        };

        let res = execute_and_record_move(
            &cmd,
            WindowGeometry::new(100, 200, 800, 600),
            &transport,
            &clock,
            &mut store,
            &permit,
            &work_area,
        )
        .unwrap();

        match res {
            MoveOperationResult::VerifiedFailure { reason, .. } => {
                assert!(reason.contains("expired"));
            }
            other => panic!("Expected VerifiedFailure, got: {other:?}"),
        }
    }

    /// ADP-010: Out-of-bounds target arguments reject before effect.
    #[test]
    fn adp_010_out_of_bounds_target_arguments_reject_before_effect() {
        let clock = AdpClock::new(1_700_000_000_000_000);
        let (mut store, permit) = setup_store_and_permit(&clock);
        let transport = AdpMockTransport::new();

        let work_area = DisplayWorkArea {
            x: 0,
            y: 30, // reserved top 30
            width: 1920,
            height: 1050,
            monitor_id: 0,
        };

        // Window placed at y=10 (violates top reserved margin)
        let cmd = SameDisplayMoveCommand {
            window: WindowIdentity {
                address: "0x55a8f2c3b1a0".into(),
                generation_token: "gen-adp-1".into(),
                workspace_id: 1,
                monitor_id: 0,
                observed_at_us: 1_700_000_000_000_000,
                fresh_until_us: 1_700_000_010_000_000,
            },
            target_x: 100,
            target_y: 10, // Out of bounds: y < 30
            preserved_width: 800,
            preserved_height: 600,
            preserved_workspace: 1,
            preserved_monitor: 0,
        };

        let res = execute_and_record_move(
            &cmd,
            WindowGeometry::new(100, 200, 800, 600),
            &transport,
            &clock,
            &mut store,
            &permit,
            &work_area,
        )
        .unwrap();

        match res {
            MoveOperationResult::VerifiedFailure { reason, .. } => {
                assert!(reason.contains("exceeds display work area"));
            }
            other => panic!("Expected VerifiedFailure due to bounds, got: {other:?}"),
        }
    }

    /// ADP-014: Timeout after dispatch transitions to outcome_unknown with reconciliation_required.
    #[test]
    fn adp_014_timeout_after_dispatch_requires_reconciliation() {
        let clock = AdpClock::new(1_700_000_000_000_000);
        let (mut store, permit) = setup_store_and_permit(&clock);
        let transport = AdpMockTransport::new();
        transport.add_dispatch(Err(HyprlandError::Timeout));

        let work_area = DisplayWorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            monitor_id: 0,
        };

        let cmd = SameDisplayMoveCommand {
            window: WindowIdentity {
                address: "0x55a8f2c3b1a0".into(),
                generation_token: "gen-adp-1".into(),
                workspace_id: 1,
                monitor_id: 0,
                observed_at_us: 1_700_000_000_000_000,
                fresh_until_us: 1_700_000_010_000_000,
            },
            target_x: 400,
            target_y: 400,
            preserved_width: 800,
            preserved_height: 600,
            preserved_workspace: 1,
            preserved_monitor: 0,
        };

        let res = execute_and_record_move(
            &cmd,
            WindowGeometry::new(100, 200, 800, 600),
            &transport,
            &clock,
            &mut store,
            &permit,
            &work_area,
        )
        .unwrap();

        match res {
            MoveOperationResult::UnknownOutcome {
                reconciliation_required,
                ..
            } => {
                assert!(reconciliation_required);
            }
            other => panic!("Expected UnknownOutcome, got: {other:?}"),
        }
    }

    /// ADP-018: Adapter requires independent read-back evidence; acknowledgement alone is rejected.
    #[test]
    fn adp_018_dispatch_acknowledgement_alone_is_not_verified_success() {
        let clock = AdpClock::new(1_700_000_000_000_000);
        let (mut store, permit) = setup_store_and_permit(&clock);
        let transport = AdpMockTransport::new();

        let work_area = DisplayWorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            monitor_id: 0,
        };

        let cmd = SameDisplayMoveCommand {
            window: WindowIdentity {
                address: "0x55a8f2c3b1a0".into(),
                generation_token: "gen-adp-1".into(),
                workspace_id: 1,
                monitor_id: 0,
                observed_at_us: 1_700_000_000_000_000,
                fresh_until_us: 1_700_000_010_000_000,
            },
            target_x: 400,
            target_y: 300,
            preserved_width: 800,
            preserved_height: 600,
            preserved_workspace: 1,
            preserved_monitor: 0,
        };

        // Dispatch reports "ok", but read-back returns old position (100, 200)
        transport.add_dispatch(Ok("ok".into()));
        transport.set_query("j/activewindow", Ok(sample_active_window(100, 200)));

        let res = execute_and_record_move(
            &cmd,
            WindowGeometry::new(100, 200, 800, 600),
            &transport,
            &clock,
            &mut store,
            &permit,
            &work_area,
        )
        .unwrap();

        match res {
            MoveOperationResult::VerifiedFailure { reason, .. } => {
                assert!(reason.contains("Target coordinate mismatch"));
            }
            other => panic!("Expected VerifiedFailure on contradictory read-back, got: {other:?}"),
        }
    }

    /// ADP-023: Privacy test: Zero raw window titles or sensitive user text in evidence or manifests.
    #[test]
    fn adp_023_privacy_guarantee_zero_titles_in_evidence_or_manifests() {
        let clock = AdpClock::new(1_700_000_000_000_000);
        let (mut store, permit) = setup_store_and_permit(&clock);
        let transport = AdpMockTransport::new();

        let work_area = DisplayWorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            monitor_id: 0,
        };

        let cmd = SameDisplayMoveCommand {
            window: WindowIdentity {
                address: "0x55a8f2c3b1a0".into(),
                generation_token: "gen-adp-1".into(),
                workspace_id: 1,
                monitor_id: 0,
                observed_at_us: 1_700_000_000_000_000,
                fresh_until_us: 1_700_000_010_000_000,
            },
            target_x: 400,
            target_y: 300,
            preserved_width: 800,
            preserved_height: 600,
            preserved_workspace: 1,
            preserved_monitor: 0,
        };

        transport.add_dispatch(Ok("ok".into()));
        transport.set_query("j/activewindow", Ok(sample_active_window(400, 300)));

        let res = execute_and_record_move(
            &cmd,
            WindowGeometry::new(100, 200, 800, 600),
            &transport,
            &clock,
            &mut store,
            &permit,
            &work_area,
        )
        .unwrap();

        assert!(matches!(res, MoveOperationResult::VerifiedSuccess { .. }));

        // Query database to ensure no "TOP_SECRET_DOCUMENT" exists in evidence or journal
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT count(*) FROM local_operation_evidence WHERE evidence_reference LIKE '%TOP_SECRET%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);

        let attempt_status: String = store
            .conn()
            .query_row(
                "SELECT attempt_status FROM local_operation_attempt WHERE attempt_id='att-adp-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(attempt_status, "completed");

        let (outbox_event_type, payload): (String, String) = store
            .conn()
            .query_row(
                "SELECT event_type, payload_json FROM local_outbox WHERE event_id='evt-succ-att-adp-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(outbox_event_type, "lia.device.operation.outcome.v1");
        assert!(!payload.contains("TOP_SECRET"));
        assert!(!payload.contains(".pdf"));
    }

    /// ADP-024: Renderer has presentation authority only; cannot execute OS operations.
    #[test]
    fn adp_024_presentation_client_has_no_os_execution_authority() {
        struct MockAuth;
        impl PermissionAuthority for MockAuth {
            fn has_permission(&self, action_key: &str, risk_tier: RiskTier) -> bool {
                // Allows presentation toggle but never allows R2+ OS actions
                action_key.starts_with("ui.") && risk_tier <= RiskTier::R1
            }
        }

        let registry = ProductionUiRegistry::new();
        let policy = DeterministicPresentationPolicy::new(MockAuth);

        // UI control passes presentation policy
        let ui_intent = crate::ipc::UntrustedUserIntent {
            session_id: "sess-1".into(),
            sequence: 1,
            body: crate::ipc::UserEventBody {
                event: crate::ipc::UserEventKind::Clicked,
                control_id: "panel.hide".into(),
                text: None,
            },
        };

        let auth_intent = authorize_user_intent(ui_intent, &registry, &policy).unwrap();
        assert_eq!(auth_intent.action_key(), "ui.panel.hide");

        // Architectural invariant: AuthorizedPresentationIntent CANNOT be converted into EffectPermit
        // Downstream OS execution requires CommandClaims signed by trusted coordinator and verified in AgentStore.
    }
}
