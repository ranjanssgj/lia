//! End-to-end conversation turn lifecycle coordinator.
//!
//! Connects presentation input, context packaging, synthetic provider invocation,
//! deterministic broker evaluation, operation dispatch, and outcome reporting.
//!
//! Architectural Invariants:
//! 1. Model output is strictly an untrusted proposal, NEVER execution authority.
//! 2. Deterministic authorities validate and authorize operations.
//! 3. The Rust device agent owns local OS operations; renderer owns presentation only.
//! 4. Operation outcomes distinguish verified success, verified failure, and unknown outcomes.
//! 5. Acknowledgement or delivery is never interpreted as successful execution.

use super::context::{ContextItem, ContextPackage};
use super::providers::{SyntheticModelProvider, SyntheticSpeechProvider};
use super::types::{
    DataLocality, ModelResponse, ProviderPolicy, Sensitivity, UntrustedToolProposal,
};
use crate::TrustedClock;
use crate::hyprland::{
    HyprlandTransport, MoveOperationResult, SameDisplayMoveCommand, execute_and_record_move,
    inspect_active_window, inspect_work_area,
};
use crate::presentation_policy::{PermissionAuthority, RiskTier};
use crate::signatures;
use crate::store::{AgentStore, CommandClaims, OperationClaims};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

#[derive(Clone, Debug, PartialEq)]
pub enum TurnResult {
    TextReply(String),
    OperationExecuted {
        tool_name: String,
        result: MoveOperationResult,
    },
    PermissionDenied {
        action_key: String,
    },
    UnsupportedCapability {
        tool_name: String,
    },
    Refused {
        reason: String,
    },
    Interrupted,
    Failed {
        error: String,
    },
}

pub struct ConversationCoordinator<'a, T: HyprlandTransport, A: PermissionAuthority> {
    pub account_id: String,
    pub device_id: String,
    pub session_id: String,
    pub session_sequence: AtomicI64,
    pub model_provider: SyntheticModelProvider,
    pub speech_provider: SyntheticSpeechProvider,
    pub transport: &'a T,
    pub store: &'a mut AgentStore,
    pub authority: &'a A,
    pub interrupted: AtomicBool,
}

impl<'a, T: HyprlandTransport, A: PermissionAuthority> ConversationCoordinator<'a, T, A> {
    pub fn new(
        account_id: impl Into<String>,
        device_id: impl Into<String>,
        session_id: impl Into<String>,
        transport: &'a T,
        store: &'a mut AgentStore,
        authority: &'a A,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            device_id: device_id.into(),
            session_id: session_id.into(),
            session_sequence: AtomicI64::new(0),
            model_provider: SyntheticModelProvider::new(),
            speech_provider: SyntheticSpeechProvider::new(),
            transport,
            store,
            authority,
            interrupted: AtomicBool::new(false),
        }
    }

    /// Signals immediate user interruption (AC-002 / chat.stop).
    pub fn request_interruption(&self) {
        self.interrupted.store(true, Ordering::SeqCst);
    }

    /// Handles typed text submission (`chat.input` -> `text_submitted`).
    pub fn process_text_turn(
        &mut self,
        user_text: &str,
        clock: &impl TrustedClock,
    ) -> Result<TurnResult, String> {
        let now_us = clock
            .now_us()
            .map_err(|e| format!("Clock read failure: {e}"))?;

        if self.interrupted.load(Ordering::SeqCst) {
            return Ok(TurnResult::Interrupted);
        }

        let turn_id = format!("turn-{}", now_us);
        let context_expires_at = now_us + 10_000_000; // 10s TTL bound

        let context = ContextPackage::new(
            format!("ctx-{}", now_us),
            "1".into(),
            self.account_id.clone(),
            "desktop_interaction".into(),
            DataLocality::LocalOnly,
            Sensitivity::S1,
            Vec::new(),
            vec![ContextItem {
                item_id: format!("item-{}", now_us),
                kind: "user_turn_text".into(),
                content: user_text.to_string(),
                sensitivity: Sensitivity::S1,
                source_reference_id: turn_id.clone(),
            }],
            context_expires_at,
        )
        .map_err(|e| format!("Context packaging failed: {e}"))?;

        let policy = ProviderPolicy::default();

        let model_resp = self
            .model_provider
            .invoke(user_text, &context, &policy, now_us, &self.interrupted)
            .map_err(|e| format!("Synthetic model invocation failed: {e}"))?;

        match model_resp {
            ModelResponse::Interrupted => Ok(TurnResult::Interrupted),
            ModelResponse::Refused(reason) => Ok(TurnResult::Refused { reason }),
            ModelResponse::Text(reply) => Ok(TurnResult::TextReply(reply)),
            ModelResponse::ToolProposal(proposal) => {
                self.evaluate_and_dispatch_tool_proposal(proposal, &turn_id, clock)
            }
        }
    }

    /// Handles push-to-talk speech input (AC-002).
    pub fn process_push_to_talk(
        &mut self,
        audio_token: &str,
        clock: &impl TrustedClock,
    ) -> Result<TurnResult, String> {
        let transcribed_text = self
            .speech_provider
            .transcribe(audio_token)
            .map_err(|e| format!("ASR transcription error: {e}"))?;

        self.process_text_turn(&transcribed_text, clock)
    }

    /// Evaluates untrusted tool proposal through deterministic broker (Invariant 1 & 2).
    fn evaluate_and_dispatch_tool_proposal(
        &mut self,
        proposal: UntrustedToolProposal,
        turn_id: &str,
        clock: &impl TrustedClock,
    ) -> Result<TurnResult, String> {
        // Broker check 1: Capability allowlist
        if proposal.tool_name != "desktop.window.move_same_display" {
            return Ok(TurnResult::UnsupportedCapability {
                tool_name: proposal.tool_name,
            });
        }

        // Broker check 2: Parse arguments
        let args: serde_json::Value = serde_json::from_str(&proposal.arguments_json)
            .map_err(|e| format!("Malformed tool arguments: {e}"))?;
        let target_x = args
            .get("target_x")
            .and_then(|v| v.as_i64())
            .ok_or("Missing target_x")? as i32;
        let target_y = args
            .get("target_y")
            .and_then(|v| v.as_i64())
            .ok_or("Missing target_y")? as i32;

        // Broker check 3: Permission check
        let action_key = "desktop.window.move_same_display";
        if !self.authority.has_permission(action_key, RiskTier::R3) {
            return Ok(TurnResult::PermissionDenied {
                action_key: action_key.to_string(),
            });
        }

        // Observation 1: Active window inspection
        let snapshot = inspect_active_window(self.transport, clock, turn_id)
            .map_err(|e| format!("Failed to inspect active window: {e}"))?;

        // Observation 2: Display work area inspection
        let work_area = inspect_work_area(self.transport, snapshot.identity.monitor_id)
            .map_err(|e| format!("Failed to inspect work area: {e}"))?;

        // Pre-flight check: display bounds
        let cmd = SameDisplayMoveCommand {
            window: snapshot.identity.clone(),
            target_x,
            target_y,
            preserved_width: snapshot.geometry.width,
            preserved_height: snapshot.geometry.height,
            preserved_workspace: snapshot.identity.workspace_id,
            preserved_monitor: snapshot.identity.monitor_id,
        };

        if !work_area.contains_rect(
            cmd.target_x,
            cmd.target_y,
            cmd.preserved_width,
            cmd.preserved_height,
        ) {
            return Ok(TurnResult::Failed {
                error: format!(
                    "Target rectangle ({}, {}, {}, {}) exceeds display bounds",
                    cmd.target_x, cmd.target_y, cmd.preserved_width, cmd.preserved_height
                ),
            });
        }

        let op_id = format!("op-{}", turn_id);
        let cmd_id = format!("cmd-{}", turn_id);
        let attempt_id = format!("att-{}", turn_id);
        let now_us = clock.now_us().map_err(|e| e.to_string())?;
        let now_iso = crate::time::format_rfc3339_us(now_us);
        let expires_iso = crate::time::format_rfc3339_us(now_us + 60_000_000);

        let op_claims = OperationClaims {
            operation_id: op_id.clone(),
            account_id: self.account_id.clone(),
            target_device_id: self.device_id.clone(),
            target_credential_generation: 1,
            capability_id: "desktop.window.move_same_display".into(),
            capability_schema_version: 1,
            resource_type: "wayland.window".into(),
            resource_id: snapshot.identity.address.clone(),
            resource_generation: snapshot.identity.observed_at_us.to_string(),
            expected_resource_revision: None,
            resource_observed_at: Some(now_iso.clone()),
            resource_owning_device_id: Some(self.device_id.clone()),
            arguments_schema_uri: "schema://lia/desktop/window/move/v1".into(),
            arguments_sha256: signatures::sha256(proposal.arguments_json.as_bytes()),
            grant_reference: "grant-user-direct-turn".into(),
            policy_version: 1,
            revocation_epoch: 1,
            risk_tier: 3,
            purpose: "user_turn_move".into(),
            originating_task_id: Some(turn_id.to_string()),
            received_at: now_iso.clone(),
            expires_at: expires_iso.clone(),
        };

        let seq = self.session_sequence.fetch_add(1, Ordering::SeqCst);
        let cmd_claims = CommandClaims {
            command_id: cmd_id,
            nonce: format!("nonce-{}", turn_id),
            session_id: self.session_id.clone(),
            session_sequence: seq,
            not_before: now_iso.clone(),
            expires_at: expires_iso.clone(),
            nonce_expires_at: expires_iso.clone(),
        };

        // Open session and insert operation in store
        let _ = self
            .store
            .open_session(&self.session_id, &now_iso, &expires_iso);
        let _ = self.store.insert_operation(&op_claims);

        // Consume-before-effect
        let permit = self
            .store
            .consume_command_before_effect(&op_claims, &cmd_claims, &attempt_id, 1, 1, &now_iso)
            .map_err(|e| format!("Failed to obtain EffectPermit: {e}"))?;

        // Dispatch with read-back verification
        let result = execute_and_record_move(
            &cmd,
            snapshot.geometry,
            self.transport,
            clock,
            self.store,
            &permit,
            &work_area,
        )
        .map_err(|e| format!("Move execution error: {e}"))?;

        Ok(TurnResult::OperationExecuted {
            tool_name: proposal.tool_name,
            result,
        })
    }
}
