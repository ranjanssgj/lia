//! Local persistence and cryptographic primitives for the first P5 device-agent slice.
//! This crate is not a complete protobuf verifier or production security boundary.

pub mod signatures;
mod store;

pub use store::{
    AgentStore, CommandClaims, EffectPermit, LeaseClaims, LeaseConsumption, OperationClaims,
    StoreError,
};
