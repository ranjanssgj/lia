//! Hyprland v0.57.2 Linux Wayland desktop adapter.
//!
//! Architectural Invariants:
//! 1. Window references are ephemeral; stale or expired references fail closed.
//! 2. Active-window discovery strictly redacts raw window titles and application content.
//! 3. Only same-display moves preserving dimensions and workspace are permitted.
//! 4. Operation outcomes distinguish verified success, verified failure, and unknown outcomes.
//! 5. Dispatch acknowledgement is never interpreted as execution success (independent read-back required).
//! 6. Consume-before-effect: Requires valid [`crate::store::EffectPermit`] from the agent store.

pub mod client;
pub mod discovery;
pub mod events;
pub mod manifest;
pub mod operations;
pub mod types;

pub use client::{
    HYPRLAND_IO_TIMEOUT, HyprlandCommandFilter, HyprlandSocketClient, HyprlandTransport,
    MAX_HYPRLAND_RESPONSE_BYTES,
};
pub use discovery::{inspect_active_window, inspect_work_area};
pub use events::{HyprlandCacheInvalidator, HyprlandEvent, parse_event_line};
pub use manifest::{HyprlandCapabilityManifest, generate_hyprland_manifest};
pub use operations::execute_and_record_move;
pub use types::{
    ActiveWindowSnapshot, DisplayWorkArea, HyprlandError, MoveOperationResult,
    SameDisplayMoveCommand, WindowGeometry, WindowIdentity,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{AgentStore, CommandClaims, OperationClaims, TrustedClock};
    use std::sync::Mutex;

    /// Synthetic test mock for Hyprland transport.
    struct MockHyprlandTransport {
        query_responses: Mutex<std::collections::HashMap<String, Result<String, HyprlandError>>>,
        dispatch_responses: Mutex<Vec<Result<String, HyprlandError>>>,
        dispatched_commands: Mutex<Vec<String>>,
    }

    impl MockHyprlandTransport {
        fn new() -> Self {
            Self {
                query_responses: Mutex::new(std::collections::HashMap::new()),
                dispatch_responses: Mutex::new(Vec::new()),
                dispatched_commands: Mutex::new(Vec::new()),
            }
        }

        fn set_query_response(&self, cmd: &str, resp: Result<String, HyprlandError>) {
            self.query_responses
                .lock()
                .unwrap()
                .insert(cmd.to_string(), resp);
        }

        fn add_dispatch_response(&self, resp: Result<String, HyprlandError>) {
            self.dispatch_responses.lock().unwrap().push(resp);
        }

        fn get_dispatched_commands(&self) -> Vec<String> {
            self.dispatched_commands.lock().unwrap().clone()
        }
    }

    impl HyprlandTransport for MockHyprlandTransport {
        fn query(&self, command: &str) -> Result<String, HyprlandError> {
            HyprlandCommandFilter::validate_query(command)?;
            self.query_responses
                .lock()
                .unwrap()
                .get(command)
                .cloned()
                .unwrap_or_else(|| {
                    Err(HyprlandError::CompositorError(format!(
                        "Unhandled mock query: {command}"
                    )))
                })
        }

        fn dispatch(&self, command: &str) -> Result<String, HyprlandError> {
            self.dispatched_commands
                .lock()
                .unwrap()
                .push(command.to_string());
            let mut list = self.dispatch_responses.lock().unwrap();
            if list.is_empty() {
                Ok("ok".to_string())
            } else {
                list.remove(0)
            }
        }
    }

    #[derive(Clone)]
    struct MockClock {
        time_us: std::sync::Arc<std::sync::atomic::AtomicI64>,
    }

    impl MockClock {
        fn new(initial_us: i64) -> Self {
            Self {
                time_us: std::sync::Arc::new(std::sync::atomic::AtomicI64::new(initial_us)),
            }
        }

        fn advance_us(&self, delta: i64) {
            self.time_us
                .fetch_add(delta, std::sync::atomic::Ordering::SeqCst);
        }
    }

    impl TrustedClock for MockClock {
        fn now_us(&self) -> Result<i64, crate::store::StoreError> {
            Ok(self.time_us.load(std::sync::atomic::Ordering::SeqCst))
        }
    }

    fn sample_active_window_json(
        address: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        floating: bool,
    ) -> String {
        format!(
            r#"{{
            "address": "{address}",
            "mapped": true,
            "hidden": false,
            "at": [{x}, {y}],
            "size": [{w}, {h}],
            "workspace": {{"id": 1, "name": "1"}},
            "floating": {floating},
            "monitor": 0,
            "class": "foot",
            "title": "SUPER_SECRET_USER_DOCUMENT_DO_NOT_LEAK.txt",
            "initialClass": "foot",
            "initialTitle": "foot",
            "pid": 4242
        }}"#
        )
    }

    fn sample_monitors_json() -> String {
        r#"[
            {
                "id": 0,
                "name": "DP-1",
                "x": 0,
                "y": 0,
                "width": 1920,
                "height": 1080,
                "reserved": [30, 0, 0, 0],
                "focused": true
            }
        ]"#
        .to_string()
    }

    #[test]
    fn inspect_active_window_redacts_title_and_enforces_freshness() {
        let clock = MockClock::new(1_700_000_000_000_000);
        let transport = MockHyprlandTransport::new();
        transport.set_query_response(
            "j/activewindow",
            Ok(sample_active_window_json(
                "0x55a8f2c3b1a0",
                100,
                200,
                800,
                600,
                true,
            )),
        );

        let snapshot =
            inspect_active_window(&transport, &clock, "gen-token-123").expect("discovery failed");

        assert_eq!(snapshot.identity.address, "0x55a8f2c3b1a0");
        assert_eq!(snapshot.identity.generation_token, "gen-token-123");
        assert_eq!(snapshot.identity.workspace_id, 1);
        assert_eq!(snapshot.identity.monitor_id, 0);
        assert_eq!(snapshot.identity.observed_at_us, 1_700_000_000_000_000);
        assert_eq!(snapshot.identity.fresh_until_us, 1_700_000_010_000_000); // +10s
        assert_eq!(snapshot.geometry.x, 100);
        assert_eq!(snapshot.geometry.y, 200);
        assert_eq!(snapshot.geometry.width, 800);
        assert_eq!(snapshot.geometry.height, 600);
        assert_eq!(snapshot.app_class, "foot");
        assert!(snapshot.floating);
        assert!(snapshot.title_redacted);

        // Verify Privacy Invariant: raw title was NOT captured
        let serialized = serde_json::to_string(&snapshot).unwrap();
        assert!(!serialized.contains("SUPER_SECRET_USER_DOCUMENT"));
    }

    #[test]
    fn inspect_active_window_fails_when_absent_or_unmapped() {
        let clock = MockClock::new(1_700_000_000_000_000);
        let transport = MockHyprlandTransport::new();
        // Null window or unmapped
        transport.set_query_response(
            "j/activewindow",
            Ok(r#"{"address": "0x0", "mapped": false, "hidden": false}"#.into()),
        );

        let err = inspect_active_window(&transport, &clock, "gen-token").unwrap_err();
        assert_eq!(err, HyprlandError::ActiveWindowAbsent);
    }

    #[test]
    fn inspect_work_area_computes_reserved_margins() {
        let transport = MockHyprlandTransport::new();
        transport.set_query_response("j/monitors", Ok(sample_monitors_json()));

        let work_area = inspect_work_area(&transport, 0).expect("work area failed");
        assert_eq!(work_area.monitor_id, 0);
        assert_eq!(work_area.x, 0);
        assert_eq!(work_area.y, 30); // reserved top = 30
        assert_eq!(work_area.width, 1920);
        assert_eq!(work_area.height, 1050); // 1080 - 30 = 1050

        // Test work area containment
        assert!(work_area.contains_rect(0, 30, 800, 600));
        assert!(work_area.contains_rect(1120, 450, 800, 600));
        // Out of bounds: top exceeds reserved panel
        assert!(!work_area.contains_rect(0, 20, 800, 600));
        // Out of bounds: bottom exceeds screen
        assert!(!work_area.contains_rect(0, 500, 800, 600));
    }

    #[test]
    fn command_filter_validates_hex_addresses_and_disallowed_queries() {
        assert!(HyprlandCommandFilter::validate_hex_address("0x123abc").is_ok());
        assert!(HyprlandCommandFilter::validate_hex_address("0x55a8f2c3b1a0").is_ok());
        assert!(HyprlandCommandFilter::validate_hex_address("not_hex").is_err());
        assert!(HyprlandCommandFilter::validate_hex_address("0x").is_err());
        assert!(HyprlandCommandFilter::validate_hex_address("0x123;rm -rf /").is_err());

        assert!(HyprlandCommandFilter::validate_query("j/activewindow").is_ok());
        assert!(HyprlandCommandFilter::validate_query("j/monitors").is_ok());
        assert!(HyprlandCommandFilter::validate_query("dispatch killactive").is_err());
        assert!(HyprlandCommandFilter::validate_query("sh -c echo").is_err());
    }

    fn setup_test_store(clock: MockClock) -> (AgentStore, crate::store::EffectPermit) {
        let mut store =
            AgentStore::open_in_memory_with_clock(std::sync::Arc::new(clock.clone())).unwrap();

        let now_us = clock.now_us().unwrap();
        let now_str = crate::time::format_rfc3339_us(now_us);
        let exp_str = crate::time::format_rfc3339_us(now_us + 3_600_000_000);

        let op = OperationClaims {
            operation_id: "op-move-1".into(),
            account_id: "acct-test".into(),
            target_device_id: "dev-test".into(),
            target_credential_generation: 1,
            capability_id: "desktop.window.move_same_display".into(),
            capability_schema_version: 1,
            resource_type: "desktop.window".into(),
            resource_id: "0x55a8f2c3b1a0".into(),
            resource_generation: "gen-1".into(),
            expected_resource_revision: Some("rev-1".into()),
            resource_observed_at: Some(now_str.clone()),
            resource_owning_device_id: Some("dev-test".into()),
            arguments_schema_uri: "urn:lia:schema:desktop.window.move_same_display.input:v1".into(),
            arguments_sha256: [0u8; 32],
            grant_reference: "grant-test".into(),
            policy_version: 1,
            revocation_epoch: 0,
            risk_tier: 1,
            purpose: "same-display-move".into(),
            originating_task_id: None,
            received_at: now_str.clone(),
            expires_at: exp_str.clone(),
        };
        store.insert_operation(&op).unwrap();

        let cmd = CommandClaims {
            command_id: "cmd-move-1".into(),
            nonce: "nonce-1".into(),
            session_id: "sess-test".into(),
            session_sequence: 0,
            not_before: now_str.clone(),
            expires_at: exp_str.clone(),
            nonce_expires_at: exp_str.clone(),
        };
        store
            .open_session("sess-test", &now_str, &cmd.nonce_expires_at)
            .unwrap();
        let permit = store
            .consume_command_before_effect(&op, &cmd, "att-1", 1, 1, &now_str)
            .unwrap();

        (store, permit)
    }

    #[test]
    fn execute_same_display_move_verified_success_path() {
        let clock = MockClock::new(1_700_000_000_000_000);
        let (mut store, permit) = setup_test_store(clock.clone());
        let transport = MockHyprlandTransport::new();

        let work_area = DisplayWorkArea {
            x: 0,
            y: 30,
            width: 1920,
            height: 1050,
            monitor_id: 0,
        };

        let before_geom = WindowGeometry::new(100, 200, 800, 600);
        let target_x = 400;
        let target_y = 300;

        let cmd = SameDisplayMoveCommand {
            window: WindowIdentity {
                address: "0x55a8f2c3b1a0".into(),
                generation_token: "gen-1".into(),
                workspace_id: 1,
                monitor_id: 0,
                observed_at_us: 1_700_000_000_000_000,
                fresh_until_us: 1_700_000_010_000_000,
            },
            target_x,
            target_y,
            preserved_width: 800,
            preserved_height: 600,
            preserved_workspace: 1,
            preserved_monitor: 0,
        };

        // When dispatch succeeds, the subsequent independent read-back query returns target coordinates
        transport.add_dispatch_response(Ok("ok".into()));
        transport.set_query_response(
            "j/activewindow",
            Ok(sample_active_window_json(
                "0x55a8f2c3b1a0",
                target_x,
                target_y,
                800,
                600,
                true,
            )),
        );

        let result = execute_and_record_move(
            &cmd,
            before_geom,
            &transport,
            &clock,
            &mut store,
            &permit,
            &work_area,
        )
        .expect("move execution failed");

        match result {
            MoveOperationResult::VerifiedSuccess { before, after, .. } => {
                assert_eq!(before, before_geom);
                assert_eq!(after, WindowGeometry::new(target_x, target_y, 800, 600));
            }
            other => panic!("Expected VerifiedSuccess, got: {other:?}"),
        }

        // Verify dispatched command format
        let dispatched = transport.get_dispatched_commands();
        assert_eq!(dispatched.len(), 1);
        assert_eq!(
            dispatched[0],
            "dispatch movewindowpixel exact 400 300,address:0x55a8f2c3b1a0"
        );
    }

    #[test]
    fn execute_move_rejects_expired_reference() {
        let clock = MockClock::new(1_700_000_000_000_000);
        let (mut store, permit) = setup_test_store(clock.clone());
        // Time advances beyond 10s freshness window
        clock.advance_us(15_000_000);
        let transport = MockHyprlandTransport::new();

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
                generation_token: "gen-1".into(),
                workspace_id: 1,
                monitor_id: 0,
                observed_at_us: 1_700_000_000_000_000,
                fresh_until_us: 1_700_000_010_000_000, // expired 5 seconds ago
            },
            target_x: 200,
            target_y: 200,
            preserved_width: 800,
            preserved_height: 600,
            preserved_workspace: 1,
            preserved_monitor: 0,
        };

        let result = execute_and_record_move(
            &cmd,
            WindowGeometry::new(100, 100, 800, 600),
            &transport,
            &clock,
            &mut store,
            &permit,
            &work_area,
        )
        .expect("move call failed");

        match result {
            MoveOperationResult::VerifiedFailure { reason, .. } => {
                assert!(reason.contains("expired"));
            }
            other => panic!("Expected VerifiedFailure due to expiry, got: {other:?}"),
        }

        // Assert nothing was dispatched to compositor
        assert!(transport.get_dispatched_commands().is_empty());
    }

    #[test]
    fn execute_move_read_back_contradiction_yields_verified_failure() {
        let clock = MockClock::new(1_700_000_000_000_000);
        let (mut store, permit) = setup_test_store(clock.clone());
        let transport = MockHyprlandTransport::new();

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
                generation_token: "gen-1".into(),
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

        // Dispatch claims success ("ok"), but independent read-back shows the window didn't actually move!
        transport.add_dispatch_response(Ok("ok".into()));
        transport.set_query_response(
            "j/activewindow",
            Ok(sample_active_window_json(
                "0x55a8f2c3b1a0",
                100,
                100,
                800,
                600,
                true,
            )), // unchanged at (100,100)
        );

        let result = execute_and_record_move(
            &cmd,
            WindowGeometry::new(100, 100, 800, 600),
            &transport,
            &clock,
            &mut store,
            &permit,
            &work_area,
        )
        .expect("move call failed");

        match result {
            MoveOperationResult::VerifiedFailure { reason, .. } => {
                // Invariant 8: acknowledgement != execution; contradictory read-back fails
                assert!(reason.contains("Target coordinate mismatch"));
            }
            other => panic!("Expected VerifiedFailure, got: {other:?}"),
        }
    }

    #[test]
    fn execute_move_dispatch_timeout_yields_unknown_outcome() {
        let clock = MockClock::new(1_700_000_000_000_000);
        let (mut store, permit) = setup_test_store(clock.clone());
        let transport = MockHyprlandTransport::new();

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
                generation_token: "gen-1".into(),
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

        // Dispatch times out after possible socket write
        transport.add_dispatch_response(Err(HyprlandError::Timeout));

        let result = execute_and_record_move(
            &cmd,
            WindowGeometry::new(100, 100, 800, 600),
            &transport,
            &clock,
            &mut store,
            &permit,
            &work_area,
        )
        .expect("move call failed");

        match result {
            MoveOperationResult::UnknownOutcome {
                reconciliation_required,
                ..
            } => {
                // Invariant 7: Uncertain outcome requires reconciliation, not blind retry
                assert!(reconciliation_required);
            }
            other => panic!("Expected UnknownOutcome, got: {other:?}"),
        }
    }

    #[test]
    fn parse_event_line_redacts_title_and_handles_all_event_types() {
        // activewindow with title and class -> title MUST be redacted
        let ev1 = parse_event_line("activewindow>>Private Financial Report,firefox").unwrap();
        match ev1 {
            HyprlandEvent::ActiveWindowChanged {
                address,
                window_title_redacted,
            } => {
                assert_eq!(address, None);
                assert!(window_title_redacted);
            }
            other => panic!("Unexpected event: {other:?}"),
        }

        // activewindowv2 with address
        let ev2 = parse_event_line("activewindowv2>>0x55a8f2c3b1a0").unwrap();
        assert_eq!(
            ev2,
            HyprlandEvent::ActiveWindowChanged {
                address: Some("0x55a8f2c3b1a0".into()),
                window_title_redacted: true,
            }
        );

        // closewindow
        let ev3 = parse_event_line("closewindow>>0x55a8f2c3b1a0").unwrap();
        assert_eq!(
            ev3,
            HyprlandEvent::WindowClosed {
                address: "0x55a8f2c3b1a0".into()
            }
        );

        // movewindow
        let ev4 = parse_event_line("movewindow>>0x55a8f2c3b1a0,2").unwrap();
        assert_eq!(
            ev4,
            HyprlandEvent::WindowMoved {
                address: "0x55a8f2c3b1a0".into()
            }
        );

        // monitoradded and monitorremoved
        assert_eq!(
            parse_event_line("monitoradded>>DP-2").unwrap(),
            HyprlandEvent::MonitorAdded {
                name: "DP-2".into()
            }
        );
        assert_eq!(
            parse_event_line("monitorremoved>>HDMI-A-1").unwrap(),
            HyprlandEvent::MonitorRemoved {
                name: "HDMI-A-1".into()
            }
        );

        // workspace
        assert_eq!(
            parse_event_line("workspace>>3").unwrap(),
            HyprlandEvent::WorkspaceChanged { id: 3 }
        );

        // empty / whitespace line
        assert_eq!(parse_event_line("   \n"), None);
    }

    #[test]
    fn cache_invalidator_advances_epoch_on_relevant_events() {
        let invalidator = HyprlandCacheInvalidator::new();
        assert_eq!(invalidator.current_epoch(), 1);
        assert!(invalidator.is_valid(1));

        // Window change increments epoch
        let ev = HyprlandEvent::ActiveWindowChanged {
            address: Some("0x123".into()),
            window_title_redacted: true,
        };
        assert!(invalidator.handle_event(&ev));
        assert_eq!(invalidator.current_epoch(), 2);
        assert!(!invalidator.is_valid(1));
        assert!(invalidator.is_valid(2));

        // Unrecognized event does not invalidate
        let other = HyprlandEvent::Other("some_custom_event".into());
        assert!(!invalidator.handle_event(&other));
        assert_eq!(invalidator.current_epoch(), 2);
    }

    #[test]
    fn manifest_generation_conforms_to_contract_v1_and_preserves_privacy() {
        let snapshot = ActiveWindowSnapshot {
            identity: WindowIdentity {
                address: "0x55a8f2c3b1a0".into(),
                generation_token: "gen-tok-1".into(),
                workspace_id: 1,
                monitor_id: 0,
                observed_at_us: 1_700_000_000_000_000,
                fresh_until_us: 1_700_000_010_000_000,
            },
            geometry: WindowGeometry::new(100, 200, 800, 600),
            app_class: "foot".into(),
            title_redacted: true,
            floating: true,
        };

        let manifest = generate_hyprland_manifest(
            "79eab1e0-6e30-4ab5-9f4d-94ca97d3d0e1",
            "a6d3fc06-9e4d-4c56-b875-f49996cc22d8",
            1,
            "2026-11-18T00:00:00Z",
            "2026-11-18T00:01:00Z",
            &[snapshot],
        );

        assert_eq!(manifest.contract_version, "lia.adapter.v1");
        assert_eq!(manifest.manifest_revision, 1);
        assert_eq!(manifest.adapter.adapter_id, "desktop.hyprland");
        assert_eq!(manifest.capabilities.len(), 2);
        assert_eq!(
            manifest.capabilities[0].capability_id,
            "desktop.window.observe_geometry"
        );
        assert_eq!(
            manifest.capabilities[1].capability_id,
            "desktop.window.move_same_display"
        );

        // Verify resource declaration
        assert_eq!(manifest.resources.len(), 1);
        assert_eq!(manifest.resources[0].resource_id, "0x55a8f2c3b1a0");
        assert_eq!(manifest.resources[0].display_label, Some("foot".into()));
        assert_eq!(manifest.resources[0].sensitivity, "S1");

        // Verify serialization does not contain raw title
        let json_str = serde_json::to_string_pretty(&manifest).unwrap();
        assert!(!json_str.contains("title"));
        assert!(json_str.contains("desktop.window.move_same_display"));
    }
}
