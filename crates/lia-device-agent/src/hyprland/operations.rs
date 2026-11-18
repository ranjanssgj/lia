//! Execution and post-operation verification for same-display window movement.
//!
//! Architectural Invariants:
//! 1. Pre-flight verification: validate window freshness, single-display scope, and work-area bounds.
//! 2. Invariant 8: Dispatch command acknowledgement is NEVER interpreted as execution success.
//! 3. Independent read-back verification: Query active window geometry through a distinct read.
//! 4. Invariant 7: Distinguish verified success, verified failure, and unknown outcomes.
//! 5. Consume-before-effect: Requires a valid [`EffectPermit`] from [`crate::AgentStore`].

use super::client::{HyprlandCommandFilter, HyprlandTransport};
use super::discovery::inspect_active_window;
use super::types::{
    DisplayWorkArea, HyprlandError, MoveOperationResult, SameDisplayMoveCommand, WindowGeometry,
};
use crate::TrustedClock;
use crate::store::{AgentStore, AttemptCompletion, EffectPermit, EvidenceRecord, ObservedOutcome};

/// Executes a same-display window move, performs independent post-operation read-back,
/// and atomically commits the outcome to the [`AgentStore`].
pub fn execute_and_record_move<T, C>(
    command: &SameDisplayMoveCommand,
    before_geometry: WindowGeometry,
    transport: &T,
    clock: &C,
    store: &mut AgentStore,
    permit: &EffectPermit,
    work_area: &DisplayWorkArea,
) -> Result<MoveOperationResult, HyprlandError>
where
    T: HyprlandTransport,
    C: TrustedClock,
{
    let now_us = clock
        .now_us()
        .map_err(|e| HyprlandError::CompositorError(format!("Clock failure: {e}")))?;

    // 1. Pre-flight Freshness Check
    if !command.window.is_fresh(now_us) {
        let err = HyprlandError::ExpiredResource {
            observed_at_us: command.window.observed_at_us,
            fresh_until_us: command.window.fresh_until_us,
            now_us,
        };
        record_terminal_failure(store, permit, clock, &err.to_string())?;
        return Ok(MoveOperationResult::VerifiedFailure {
            reason: err.to_string(),
            attempt_id: permit.attempt_id().to_string(),
        });
    }

    // 2. Pre-flight Monitor & Scope Check
    if command.window.monitor_id != command.preserved_monitor
        || command.window.monitor_id != work_area.monitor_id
    {
        let err = HyprlandError::MonitorMismatch {
            expected: command.preserved_monitor,
            actual: command.window.monitor_id,
        };
        record_terminal_failure(store, permit, clock, &err.to_string())?;
        return Ok(MoveOperationResult::VerifiedFailure {
            reason: err.to_string(),
            attempt_id: permit.attempt_id().to_string(),
        });
    }

    // 3. Pre-flight Work Area Bounds Check
    if !work_area.contains_rect(
        command.target_x,
        command.target_y,
        command.preserved_width,
        command.preserved_height,
    ) {
        let err = HyprlandError::OutOfBounds {
            target_x: command.target_x,
            target_y: command.target_y,
            width: command.preserved_width,
            height: command.preserved_height,
            work_area: *work_area,
        };
        record_terminal_failure(store, permit, clock, &err.to_string())?;
        return Ok(MoveOperationResult::VerifiedFailure {
            reason: err.to_string(),
            attempt_id: permit.attempt_id().to_string(),
        });
    }

    // 4. Construct Structured Dispatch Command
    let dispatch_cmd = HyprlandCommandFilter::format_same_display_move(
        command.target_x,
        command.target_y,
        &command.window.address,
    )?;

    // 5. Issue Dispatch Request to Compositor
    let dispatch_res = transport.dispatch(&dispatch_cmd);
    if let Err(e) = dispatch_res {
        // If socket error occurred after connect/send, outcome is unknown and needs reconciliation
        let is_unknown = matches!(e, HyprlandError::Timeout | HyprlandError::SocketIoError(_));
        if is_unknown {
            record_unknown_outcome(store, permit, clock)?;
            return Ok(MoveOperationResult::UnknownOutcome {
                attempt_id: permit.attempt_id().to_string(),
                reconciliation_required: true,
            });
        } else {
            record_terminal_failure(store, permit, clock, &e.to_string())?;
            return Ok(MoveOperationResult::VerifiedFailure {
                reason: e.to_string(),
                attempt_id: permit.attempt_id().to_string(),
            });
        }
    }

    // 6. Independent Read-Back Verification (Invariant 8)
    let read_back = inspect_active_window(transport, clock, &command.window.generation_token);
    let after_snapshot = match read_back {
        Ok(snap) => snap,
        Err(_) => {
            // Read-back failed: we cannot verify whether move succeeded or failed
            record_unknown_outcome(store, permit, clock)?;
            return Ok(MoveOperationResult::UnknownOutcome {
                attempt_id: permit.attempt_id().to_string(),
                reconciliation_required: true,
            });
        }
    };

    // 7. Verify Post-Condition Invariants
    if after_snapshot.identity.address != command.window.address {
        let reason = format!(
            "Address mismatch on read-back: expected {}, observed {}",
            command.window.address, after_snapshot.identity.address
        );
        record_terminal_failure(store, permit, clock, &reason)?;
        return Ok(MoveOperationResult::VerifiedFailure {
            reason,
            attempt_id: permit.attempt_id().to_string(),
        });
    }

    if after_snapshot.identity.monitor_id != command.preserved_monitor {
        let reason = format!(
            "Monitor mismatch on read-back: expected {}, observed {}",
            command.preserved_monitor, after_snapshot.identity.monitor_id
        );
        record_terminal_failure(store, permit, clock, &reason)?;
        return Ok(MoveOperationResult::VerifiedFailure {
            reason,
            attempt_id: permit.attempt_id().to_string(),
        });
    }

    if after_snapshot.identity.workspace_id != command.preserved_workspace {
        let reason = format!(
            "Workspace mismatch on read-back: expected {}, observed {}",
            command.preserved_workspace, after_snapshot.identity.workspace_id
        );
        record_terminal_failure(store, permit, clock, &reason)?;
        return Ok(MoveOperationResult::VerifiedFailure {
            reason,
            attempt_id: permit.attempt_id().to_string(),
        });
    }

    if after_snapshot.geometry.width != command.preserved_width
        || after_snapshot.geometry.height != command.preserved_height
    {
        let reason = format!(
            "Dimension preservation failed: expected {}x{}, observed {}x{}",
            command.preserved_width,
            command.preserved_height,
            after_snapshot.geometry.width,
            after_snapshot.geometry.height
        );
        record_terminal_failure(store, permit, clock, &reason)?;
        return Ok(MoveOperationResult::VerifiedFailure {
            reason,
            attempt_id: permit.attempt_id().to_string(),
        });
    }

    if after_snapshot.geometry.x != command.target_x
        || after_snapshot.geometry.y != command.target_y
    {
        let reason = format!(
            "Target coordinate mismatch: expected ({},{}), observed ({},{})",
            command.target_x,
            command.target_y,
            after_snapshot.geometry.x,
            after_snapshot.geometry.y
        );
        record_terminal_failure(store, permit, clock, &reason)?;
        return Ok(MoveOperationResult::VerifiedFailure {
            reason,
            attempt_id: permit.attempt_id().to_string(),
        });
    }

    // 8. Commit Verified Success with Redacted Evidence
    record_verified_success(
        store,
        permit,
        clock,
        before_geometry,
        after_snapshot.geometry,
        command.preserved_monitor,
        command.preserved_workspace,
    )?;

    Ok(MoveOperationResult::VerifiedSuccess {
        before: before_geometry,
        after: after_snapshot.geometry,
        attempt_id: permit.attempt_id().to_string(),
    })
}

fn record_terminal_failure<C: TrustedClock>(
    store: &mut AgentStore,
    permit: &EffectPermit,
    clock: &C,
    reason: &str,
) -> Result<(), HyprlandError> {
    let now_us = clock
        .now_us()
        .map_err(|e| HyprlandError::CompositorError(format!("Clock failure: {e}")))?;
    let timestamp = crate::time::format_rfc3339_us(now_us);
    let expires = crate::time::format_rfc3339_us(now_us.saturating_add(3_600_000_000));

    let evidence = EvidenceRecord {
        evidence_id: format!("ev-fail-{}", permit.attempt_id()),
        evidence_kind: "hyprland_operation_failure".into(),
        outcome: ObservedOutcome::VerifiedFailure,
        sha256: crate::signatures::sha256(reason.as_bytes()),
        reference: format!("urn:lia:evidence:attempt:{}", permit.attempt_id()),
        sensitivity: 1, // S1 sensitivity
        observed_at: timestamp.clone(),
    };

    let completion = AttemptCompletion {
        operation_id: permit.operation_id().to_string(),
        attempt_id: permit.attempt_id().to_string(),
        event_id: format!("evt-fail-{}", permit.attempt_id()),
        correlation_id: format!("corr-{}", permit.attempt_id()),
        occurred_at: timestamp,
        outbox_expires_at: expires,
        outcome: ObservedOutcome::VerifiedFailure,
        failure_code: Some("ADAPTER_VERIFICATION_FAILED".into()),
        evidence: Some(evidence),
    };

    store
        .finish_attempt(&completion)
        .map_err(|e| HyprlandError::CompositorError(format!("Store persistence failure: {e}")))
}

fn record_unknown_outcome<C: TrustedClock>(
    store: &mut AgentStore,
    permit: &EffectPermit,
    clock: &C,
) -> Result<(), HyprlandError> {
    let now_us = clock
        .now_us()
        .map_err(|e| HyprlandError::CompositorError(format!("Clock failure: {e}")))?;
    let timestamp = crate::time::format_rfc3339_us(now_us);
    let expires = crate::time::format_rfc3339_us(now_us.saturating_add(3_600_000_000));

    let completion = AttemptCompletion {
        operation_id: permit.operation_id().to_string(),
        attempt_id: permit.attempt_id().to_string(),
        event_id: format!("evt-unknown-{}", permit.attempt_id()),
        correlation_id: format!("corr-{}", permit.attempt_id()),
        occurred_at: timestamp,
        outbox_expires_at: expires,
        outcome: ObservedOutcome::Unknown,
        failure_code: None,
        evidence: None,
    };

    store
        .finish_attempt(&completion)
        .map_err(|e| HyprlandError::CompositorError(format!("Store persistence failure: {e}")))
}

fn record_verified_success<C: TrustedClock>(
    store: &mut AgentStore,
    permit: &EffectPermit,
    clock: &C,
    before: WindowGeometry,
    after: WindowGeometry,
    monitor_id: i64,
    workspace_id: i64,
) -> Result<(), HyprlandError> {
    let now_us = clock
        .now_us()
        .map_err(|e| HyprlandError::CompositorError(format!("Clock failure: {e}")))?;
    let timestamp = crate::time::format_rfc3339_us(now_us);
    let expires = crate::time::format_rfc3339_us(now_us.saturating_add(3_600_000_000));

    // Zero raw window titles or content in evidence payload
    let evidence_json = format!(
        r#"{{"before":{{"x":{},"y":{},"width":{},"height":{}}},"after":{{"x":{},"y":{},"width":{},"height":{}}},"monitor_id":{},"workspace_id":{}}}"#,
        before.x,
        before.y,
        before.width,
        before.height,
        after.x,
        after.y,
        after.width,
        after.height,
        monitor_id,
        workspace_id
    );

    let evidence = EvidenceRecord {
        evidence_id: format!("ev-succ-{}", permit.attempt_id()),
        evidence_kind: "window_geometry_observation".into(),
        outcome: ObservedOutcome::VerifiedSuccess,
        sha256: crate::signatures::sha256(evidence_json.as_bytes()),
        reference: format!("urn:lia:evidence:attempt:{}", permit.attempt_id()),
        sensitivity: 1, // S1 sensitivity
        observed_at: timestamp.clone(),
    };

    let completion = AttemptCompletion {
        operation_id: permit.operation_id().to_string(),
        attempt_id: permit.attempt_id().to_string(),
        event_id: format!("evt-succ-{}", permit.attempt_id()),
        correlation_id: format!("corr-{}", permit.attempt_id()),
        occurred_at: timestamp,
        outbox_expires_at: expires,
        outcome: ObservedOutcome::VerifiedSuccess,
        failure_code: None,
        evidence: Some(evidence),
    };

    store
        .finish_attempt(&completion)
        .map_err(|e| HyprlandError::CompositorError(format!("Store persistence failure: {e}")))
}
