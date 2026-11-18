//! Capability manifest generator and models for the Hyprland v0.57.2 adapter.
//!
//! Conforms to `docs/p4/adapters/capability-manifest-v1.schema.json`.
//! Architectural Invariants:
//! 1. The capability manifest describes technical availability; it is never an authorization grant.
//! 2. Declared capabilities explicitly enforce locality: `local_only`, `raw_capture: false`.
//! 3. All window resources are ephemeral, carrying S1 sensitivity and finite freshness.

use super::types::ActiveWindowSnapshot;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HyprlandCapabilityManifest {
    pub contract_version: String,
    pub manifest_revision: i64,
    pub adapter: AdapterMeta,
    pub execution_context: ExecutionContext,
    pub compatibility: CompatibilityInfo,
    pub lifecycle_state: String,
    pub observed_at: String,
    pub expires_at: String,
    pub resources: Vec<ManifestResource>,
    pub capabilities: Vec<CapabilityDef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterMeta {
    pub adapter_id: String,
    pub installation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<String>,
    pub owner_principal_id: String,
    pub version: String,
    pub publisher_id: String,
    pub contract_family: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub device_id: String,
    pub account_id: String,
    pub platform_id: String,
    pub platform_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compositor_or_provider_profile: Option<String>,
    pub session_state: String,
    pub execution_locality: String,
    pub trust_state: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityInfo {
    pub supported_contract_versions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_host_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_host_version: Option<String>,
    pub required_features: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestResource {
    pub resource_type: String,
    pub resource_id: String,
    pub generation: String,
    pub owner_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_label: Option<String>,
    pub observed_revision: String,
    pub observed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fresh_until: Option<String>,
    pub sensitivity: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDef {
    pub capability_id: String,
    pub schema_version: i64,
    pub operation_type: String,
    pub resource_types: Vec<String>,
    pub availability: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability_reason_code: Option<String>,
    pub requires_interactive_session: bool,
    pub required_scopes: Vec<String>,
    pub minimum_risk_tier: String,
    pub confirmation_required: bool,
    pub input_schema_uri: String,
    pub output_schema_uri: String,
    pub preconditions: Preconditions,
    pub data_governance: DataGovernance,
    pub operation_semantics: OperationSemantics,
    pub verification: VerificationPolicy,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub events: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preconditions {
    pub connectivity: String,
    pub resource_freshness_required: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub required_session_states: Vec<String>,
    pub required_user_presence: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub constraints: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataGovernance {
    pub maximum_sensitivity: String,
    pub locality: String,
    pub retention_class: String,
    pub raw_capture: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress_description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationSemantics {
    pub idempotency_class: String,
    pub retry_requires_reconciliation: bool,
    pub maximum_attempts: i64,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub cancellation_points: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationPolicy {
    pub method: String,
    pub independent_state_read: bool,
    pub evidence_required: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub evidence_types: Vec<String>,
}

/// Generates a standardized Hyprland capability manifest conforming to contract v1.
pub fn generate_hyprland_manifest(
    device_id: &str,
    account_id: &str,
    revision: i64,
    observed_at: &str,
    expires_at: &str,
    active_windows: &[ActiveWindowSnapshot],
) -> HyprlandCapabilityManifest {
    let resources = active_windows
        .iter()
        .map(|w| ManifestResource {
            resource_type: "desktop.window".into(),
            resource_id: w.identity.address.clone(),
            generation: w.identity.generation_token.clone(),
            owner_id: format!("device:{device_id}"),
            display_label: Some(w.app_class.clone()),
            observed_revision: "1".into(),
            observed_at: crate::time::format_rfc3339_us(w.identity.observed_at_us),
            fresh_until: Some(crate::time::format_rfc3339_us(w.identity.fresh_until_us)),
            sensitivity: "S1".into(),
        })
        .collect();

    let observe_cap = CapabilityDef {
        capability_id: "desktop.window.observe_geometry".into(),
        schema_version: 1,
        operation_type: "observe".into(),
        resource_types: vec!["desktop.window".into()],
        availability: "supported".into(),
        availability_reason_code: None,
        requires_interactive_session: true,
        required_scopes: vec!["desktop.window.observe_geometry".into()],
        minimum_risk_tier: "R0".into(),
        confirmation_required: false,
        input_schema_uri: "urn:lia:schema:desktop.window.observe_geometry:v1".into(),
        output_schema_uri: "urn:lia:schema:desktop.window.geometry:v1".into(),
        preconditions: Preconditions {
            connectivity: "local_session".into(),
            resource_freshness_required: true,
            required_session_states: vec!["active".into()],
            required_user_presence: false,
            constraints: vec!["Hyprland IPC sockets available".into()],
        },
        data_governance: DataGovernance {
            maximum_sensitivity: "S1".into(),
            locality: "local_only".into(),
            retention_class: "transient".into(),
            raw_capture: false,
            egress_description: Some(
                "Window geometry only; no screenshot or window content".into(),
            ),
        },
        operation_semantics: OperationSemantics {
            idempotency_class: "idempotent_target_state".into(),
            retry_requires_reconciliation: false,
            maximum_attempts: 2,
            cancellation_points: vec!["before_read".into()],
        },
        verification: VerificationPolicy {
            method: "read_geometry_from_hyprland".into(),
            independent_state_read: true,
            evidence_required: true,
            evidence_types: vec!["window_geometry_observation".into()],
        },
        events: vec!["desktop.window.geometry.changed".into()],
        rate_limit: Some(20),
    };

    let move_cap = CapabilityDef {
        capability_id: "desktop.window.move_same_display".into(),
        schema_version: 1,
        operation_type: "control".into(),
        resource_types: vec!["desktop.window".into()],
        availability: "supported".into(),
        availability_reason_code: None,
        requires_interactive_session: true,
        required_scopes: vec!["desktop.window.move".into()],
        minimum_risk_tier: "R1".into(),
        confirmation_required: false,
        input_schema_uri: "urn:lia:schema:desktop.window.move_same_display.input:v1".into(),
        output_schema_uri: "urn:lia:schema:desktop.window.move_same_display.output:v1".into(),
        preconditions: Preconditions {
            connectivity: "local_session".into(),
            resource_freshness_required: true,
            required_session_states: vec!["active".into()],
            required_user_presence: false,
            constraints: vec![
                "Same output only".into(),
                "Preserve width and height".into(),
                "Exact resource generation required".into(),
            ],
        },
        data_governance: DataGovernance {
            maximum_sensitivity: "S1".into(),
            locality: "local_only".into(),
            retention_class: "transient".into(),
            raw_capture: false,
            egress_description: Some("Window coordinates and geometry remain local".into()),
        },
        operation_semantics: OperationSemantics {
            idempotency_class: "idempotent_target_state".into(),
            retry_requires_reconciliation: true,
            maximum_attempts: 2,
            cancellation_points: vec!["before_dispatch".into()],
        },
        verification: VerificationPolicy {
            method: "post_move_geometry_readback".into(),
            independent_state_read: true,
            evidence_required: true,
            evidence_types: vec![
                "pre_move_geometry".into(),
                "post_move_geometry".into(),
                "display_identity".into(),
            ],
        },
        events: vec!["desktop.window.geometry.changed".into()],
        rate_limit: Some(5),
    };

    HyprlandCapabilityManifest {
        contract_version: "lia.adapter.v1".into(),
        manifest_revision: revision,
        adapter: AdapterMeta {
            adapter_id: "desktop.hyprland".into(),
            installation_id: device_id.to_string(),
            connector_id: None,
            owner_principal_id: format!("device:{device_id}"),
            version: "0.1.0".into(),
            publisher_id: "lia.local-agent".into(),
            contract_family: "lia.desktop.v1".into(),
        },
        execution_context: ExecutionContext {
            device_id: device_id.to_string(),
            account_id: account_id.to_string(),
            platform_id: "linux".into(),
            platform_version: "Linux (Hyprland v0.57.2 Profile)".into(),
            compositor_or_provider_profile: Some("Hyprland v0.57.2 stable".into()),
            session_state: "active".into(),
            execution_locality: "device_local".into(),
            trust_state: "enrolled".into(),
        },
        compatibility: CompatibilityInfo {
            supported_contract_versions: vec!["lia.adapter.v1".into()],
            minimum_host_version: None,
            maximum_host_version: None,
            required_features: vec!["hyprland.ipc.request".into(), "hyprland.ipc.event".into()],
        },
        lifecycle_state: "ready".into(),
        observed_at: observed_at.to_string(),
        expires_at: expires_at.to_string(),
        resources,
        capabilities: vec![observe_cap, move_cap],
    }
}
