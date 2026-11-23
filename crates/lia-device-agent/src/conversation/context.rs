//! Purpose-bound, expiring, digest-verified context packages.
//! Reference: docs/p4/context/context-package-v1.md

use super::types::{DataLocality, ScopedReference, Sensitivity};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextItem {
    pub item_id: String,
    pub kind: String,
    pub content: String,
    pub sensitivity: Sensitivity,
    pub source_reference_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPackage {
    pub context_id: String,
    pub revision: String,
    pub digest_sha256: String,
    pub principal_id: String,
    pub purpose: String,
    pub locality: DataLocality,
    pub sensitivity: Sensitivity,
    pub source_references: Vec<ScopedReference>,
    pub items: Vec<ContextItem>,
    pub expires_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextError {
    Expired,
    DigestMismatch,
    LocalityViolation(String),
    SerializationError(String),
}

impl std::fmt::Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expired => write!(f, "Context package has expired"),
            Self::DigestMismatch => write!(f, "Context package digest mismatch"),
            Self::LocalityViolation(msg) => write!(f, "Locality violation: {msg}"),
            Self::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
        }
    }
}

impl std::error::Error for ContextError {}

impl ContextPackage {
    /// Builds and digests a purpose-bound context package.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        context_id: String,
        revision: String,
        principal_id: String,
        purpose: String,
        locality: DataLocality,
        sensitivity: Sensitivity,
        source_references: Vec<ScopedReference>,
        items: Vec<ContextItem>,
        expires_at_us: i64,
    ) -> Result<Self, ContextError> {
        let digest_input = serde_json::json!({
            "context_id": &context_id,
            "revision": &revision,
            "principal_id": &principal_id,
            "purpose": &purpose,
            "locality": locality,
            "sensitivity": sensitivity,
            "items": &items,
            "expires_at_us": expires_at_us
        });

        let canonical_bytes = serde_jcs::to_vec(&digest_input)
            .map_err(|e| ContextError::SerializationError(e.to_string()))?;
        let hash = crate::signatures::sha256(&canonical_bytes);
        let digest_sha256 = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();

        Ok(Self {
            context_id,
            revision,
            digest_sha256,
            principal_id,
            purpose,
            locality,
            sensitivity,
            source_references,
            items,
            expires_at_us,
        })
    }

    /// Verifies freshness and cryptographic integrity of the context package.
    pub fn verify(&self, now_us: i64) -> Result<(), ContextError> {
        if now_us >= self.expires_at_us {
            return Err(ContextError::Expired);
        }

        let digest_input = serde_json::json!({
            "context_id": &self.context_id,
            "revision": &self.revision,
            "principal_id": &self.principal_id,
            "purpose": &self.purpose,
            "locality": self.locality,
            "sensitivity": self.sensitivity,
            "items": &self.items,
            "expires_at_us": self.expires_at_us
        });

        let canonical_bytes = serde_jcs::to_vec(&digest_input)
            .map_err(|e| ContextError::SerializationError(e.to_string()))?;
        let hash = crate::signatures::sha256(&canonical_bytes);
        let calculated = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();

        if calculated != self.digest_sha256 {
            return Err(ContextError::DigestMismatch);
        }

        Ok(())
    }

    /// Checks that data locality constraints prevent unauthorized egress.
    pub fn check_locality(&self, target_locality: DataLocality) -> Result<(), ContextError> {
        if self.locality == DataLocality::LocalOnly && target_locality != DataLocality::LocalOnly {
            return Err(ContextError::LocalityViolation(
                "Context marked local_only cannot egress to non-local target".into(),
            ));
        }
        Ok(())
    }
}
