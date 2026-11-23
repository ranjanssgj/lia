//! Deterministic synthetic provider implementations for Model, ASR, and TTS.
//!
//! Zero-Egress Invariant:
//! No network requests or external sockets are ever opened.
//! All responses are generated deterministically in-memory according to specification fixtures.
//!
//! Architectural Invariant 1:
//! Tool-shaped output from any model is strictly an untrusted proposal.
//! It carries zero execution authority and must pass downstream validation.

use super::context::{ContextError, ContextPackage};
use super::types::{ModelResponse, ProviderPolicy, UntrustedToolProposal};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug, Default)]
pub struct SyntheticModelProvider {
    pub provider_id: String,
}

impl SyntheticModelProvider {
    pub fn new() -> Self {
        Self {
            provider_id: "synthetic-local-v1".into(),
        }
    }

    /// Evaluates prompt input against deterministic rules and emits untrusted model responses.
    pub fn invoke(
        &self,
        prompt: &str,
        context: &ContextPackage,
        policy: &ProviderPolicy,
        now_us: i64,
        interrupted: &AtomicBool,
    ) -> Result<ModelResponse, ContextError> {
        // 1. Verify context freshness and integrity
        context.verify(now_us)?;

        // 2. Enforce locality constraints (zero remote egress)
        context.check_locality(policy.locality)?;

        // 3. Check for early user interruption (AC-002)
        if interrupted.load(Ordering::SeqCst) {
            return Ok(ModelResponse::Interrupted);
        }

        let lower = prompt.to_lowercase();

        // 4. Adversarial detection: Prompt injection patterns
        if lower.contains("ignore previous instructions")
            || lower.contains("ignore all instructions")
            || lower.contains("drop table")
            || lower.contains("system prompt leak")
            || lower.contains("override security")
        {
            return Ok(ModelResponse::Refused(
                "Adversarial prompt injection pattern detected; request refused.".into(),
            ));
        }

        // 5. Tool proposals (Untrusted - Invariant 1)
        if lower.contains("move window to left") || lower.contains("move active window") {
            return Ok(ModelResponse::ToolProposal(UntrustedToolProposal {
                tool_name: "desktop.window.move_same_display".into(),
                arguments_json: r#"{"target_x":100,"target_y":100}"#.into(),
            }));
        }

        if lower.contains("move window out of bounds") {
            return Ok(ModelResponse::ToolProposal(UntrustedToolProposal {
                tool_name: "desktop.window.move_same_display".into(),
                arguments_json: r#"{"target_x":50000,"target_y":50000}"#.into(),
            }));
        }

        if lower.contains("execute shell") || lower.contains("undeclared tool") {
            return Ok(ModelResponse::ToolProposal(UntrustedToolProposal {
                tool_name: "os.arbitrary_shell".into(),
                arguments_json: r#"{"command":"echo hacked"}"#.into(),
            }));
        }

        // 6. Conversational responses
        if lower.contains("hello") || lower.contains("hi lia") {
            return Ok(ModelResponse::Text(
                "Hello! I am Lia, your embodied desktop companion. How may I assist you today?"
                    .into(),
            ));
        }

        if lower.contains("remember preference") || lower.contains("set preference") {
            return Ok(ModelResponse::Text(
                "I will record that preference for your desktop session.".into(),
            ));
        }

        // Default conversational reply
        Ok(ModelResponse::Text(
            "I have received your message and am ready to help.".into(),
        ))
    }
}

/// Deterministic synthetic speech provider (ASR and TTS).
#[derive(Clone, Debug, Default)]
pub struct SyntheticSpeechProvider;

impl SyntheticSpeechProvider {
    pub fn new() -> Self {
        Self
    }

    /// Transcribes synthetic audio input token (ASR).
    pub fn transcribe(&self, audio_token: &str) -> Result<String, String> {
        match audio_token {
            "token://voice/hello_lia" => Ok("Hello Lia".into()),
            "token://voice/move_window" => Ok("Move active window to left".into()),
            "token://voice/pause" => Ok("Pause presentation".into()),
            "token://voice/corrupted" => Err("Audio input corrupted or illegible".into()),
            _ => Ok("Synthetic voice transcription".into()),
        }
    }

    /// Computes synthetic speech playback duration in milliseconds (TTS).
    pub fn synthesize_speech_duration_ms(&self, text: &str) -> u64 {
        // ~50ms per character with minimum 500ms
        let char_count = text.len() as u64;
        (char_count * 50).max(500)
    }
}
