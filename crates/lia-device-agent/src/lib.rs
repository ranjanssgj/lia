//! Local persistence and cryptographic primitives for the first P5 device-agent slice.
//! This crate is not a complete protobuf verifier or production security boundary.

#[cfg(target_os = "linux")]
pub mod ipc;
pub mod signatures;
mod store;
mod time;

pub use store::{
    AgentStore, AttemptCompletion, CommandClaims, EffectPermit, EvidenceRecord, LeaseClaims,
    LeaseConsumption, ObservedOutcome, OperationClaims, OperationOutcomeState, StoreError,
    SystemTrustedClock, TrustedClock,
};
