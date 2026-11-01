//! Pure domain logic for personal anchoring.
//!
//! Observation types are imported from `cynic_protocol::observation_record`
//! (the universal primitive). This crate adds the anchoring-specific
//! business logic: sobriety milestones, general return computation, outcomes.

pub mod observation;
pub mod sobriety;
pub mod return_t;
pub mod outcome;

pub use observation::*;
pub use sobriety::*;
pub use return_t::*;
pub use outcome::*;