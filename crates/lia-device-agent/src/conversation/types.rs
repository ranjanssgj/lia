//! Core types and schemas for the conversation, context, and provider loop.
//! Reference: docs/p4/providers/provider-interfaces-v1.ts and docs/p4/context/context-package-v1.md

use serde::{Deserialize, Serialize};

/// Data locality constraints for model invocation and context egress.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataLocality {
    LocalOnly,
    ApprovedRegion,
    AnyApprovedRegion,
}

/// Sensitivity levels (S0 least sensitive to S3 secret).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Sensitivity {
    S0,
    S1,
    S2,
    S3,
}

/// Message role in conversation turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    Developer,
    User,
    Assistant,
}

/// Trust level for prompt inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    TrustedInstruction,
    UserContent,
    UntrustedData,
}

/// A conversation turn message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: Role,
    pub content: String,
    pub trust: TrustLevel,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_reference_ids: Vec<String>,
}

/// Lifecycle status of a conversation turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnStatus {
    Created,
    Active,
    Streaming,
    Completed,
    Interrupted,
    Refused,
    Failed,
}

/// Scoped reference for provenance and data lineage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedReference {
    pub reference_id: String,
    pub owner_domain: String,
    pub scope: String,
    pub sensitivity: Sensitivity,
    pub revision: String,
    pub digest_sha256: String,
}

/// Model invocation policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderPolicy {
    pub allowed_provider_ids: Vec<String>,
    pub locality: DataLocality,
    pub maximum_sensitivity: Sensitivity,
    pub maximum_wall_time_ms: u64,
}

impl Default for ProviderPolicy {
    fn default() -> Self {
        Self {
            allowed_provider_ids: vec!["synthetic-local-v1".into()],
            locality: DataLocality::LocalOnly,
            maximum_sensitivity: Sensitivity::S1,
            maximum_wall_time_ms: 2000,
        }
    }
}

/// Proposed tool invocation from an untrusted model.
///
/// Invariant 1: Model output is strictly an untrusted proposal, NEVER execution authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UntrustedToolProposal {
    pub tool_name: String,
    pub arguments_json: String,
}

/// Model generation result from synthetic provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelResponse {
    Text(String),
    ToolProposal(UntrustedToolProposal),
    Refused(String),
    Interrupted,
}
