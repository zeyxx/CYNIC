//! Pure domain models and ports for personal anchoring.
//!
//! Observation types are imported from `cynic_protocol::observation_record`
//! (the universal primitive). This crate adds anchoring-specific business
//! logic: sobriety milestones, general return computation, outcomes.
//!
//! Hexagonal layout:
//! - `domain/` — pure domain logic (classification, milestones, return, outcomes).
//! - `port/` — driven ports (trait definitions for event store, projection, sync).
//! - `adapters/` — storage adapters.

pub mod domain;
pub mod port;
pub mod adapters;

pub use domain::{
    AnchorDomain, AnchorOutcome, GeneralReturn, SobrietyMilestone, SobrietySubstance,
};
pub use domain::observation::{
    is_anchor_observation, anchor_domain_of, duration_ms_of,
};
pub use port::{
    AnchorEventStore, AnchorOutcomeStore, AnchorProjection, AnchorSyncPort,
};
pub use adapters::InMemoryAnchorStore;