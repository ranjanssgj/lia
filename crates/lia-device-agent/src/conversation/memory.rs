//! Preference memory manager supporting AC-009.
//!
//! Manages low-sensitivity (S1) user preference persistence with full provenance,
//! inspection, and deletion through the canonical tombstone pattern.

use crate::TrustedClock;
use crate::store::{AgentStore, StoreError, UserPreferenceRecord};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryError {
    Store(String),
    DisallowedSensitivity,
    MissingSourceProvenance,
    TombstoneBlocked,
}

impl From<StoreError> for MemoryError {
    fn from(e: StoreError) -> Self {
        Self::Store(e.to_string())
    }
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(msg) => write!(f, "Store error: {msg}"),
            Self::DisallowedSensitivity => {
                write!(f, "Only S0 and S1 preferences are permitted in this slice")
            }
            Self::MissingSourceProvenance => {
                write!(
                    f,
                    "Preference persistence requires valid source reference provenance"
                )
            }
            Self::TombstoneBlocked => {
                write!(f, "Operation blocked by active deletion tombstone")
            }
        }
    }
}

impl std::error::Error for MemoryError {}

/// Memory manager for single low-sensitivity user preference (AC-009).
pub struct PreferenceMemoryManager<'a> {
    store: &'a mut AgentStore,
    account_id: String,
}

impl<'a> PreferenceMemoryManager<'a> {
    pub fn new(store: &'a mut AgentStore, account_id: impl Into<String>) -> Self {
        Self {
            store,
            account_id: account_id.into(),
        }
    }

    /// Records a user preference with provenance and revision tracking.
    pub fn save_preference(
        &mut self,
        key: &str,
        value: &str,
        source_turn_id: &str,
        clock: &impl TrustedClock,
    ) -> Result<UserPreferenceRecord, MemoryError> {
        if source_turn_id.trim().is_empty() {
            return Err(MemoryError::MissingSourceProvenance);
        }

        let now_us = clock
            .now_us()
            .map_err(|e| MemoryError::Store(format!("Clock failure: {e}")))?;

        let record = self.store.set_user_preference(
            &self.account_id,
            key,
            value,
            "account",
            &self.account_id,
            source_turn_id,
            now_us,
        )?;

        Ok(record)
    }

    /// Retrieves an active preference for inspection.
    pub fn inspect_preference(
        &self,
        key: &str,
    ) -> Result<Option<UserPreferenceRecord>, MemoryError> {
        Ok(self.store.get_user_preference(&self.account_id, key)?)
    }

    /// Deletes a preference and applies an authoritative deletion tombstone.
    pub fn delete_preference(
        &mut self,
        key: &str,
        source_event_id: &str,
        clock: &impl TrustedClock,
    ) -> Result<bool, MemoryError> {
        let now_us = clock
            .now_us()
            .map_err(|e| MemoryError::Store(format!("Clock failure: {e}")))?;
        let now_iso = crate::time::format_rfc3339_us(now_us);

        Ok(self.store.delete_user_preference(
            &self.account_id,
            key,
            source_event_id,
            now_us,
            &now_iso,
        )?)
    }

    /// Lists all active preferences for this account.
    pub fn list_preferences(&self) -> Result<Vec<UserPreferenceRecord>, MemoryError> {
        Ok(self.store.list_user_preferences(&self.account_id)?)
    }
}
