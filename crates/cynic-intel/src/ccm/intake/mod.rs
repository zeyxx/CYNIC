//! CCM Intake — content hashing, pattern extraction, observation building.
//!
//! Ingestion side of the CCM pipeline: normalize content, extract patterns
//! from observations, infer domains, build observation records.

pub mod builder;
pub mod hash;
pub mod slug;

#[cfg(test)]
mod tests;

pub use builder::{build_observation, build_observation_with_ledger, infer_domain};
pub use hash::{content_hash, normalize_for_hash};
pub use slug::semantic_slug;
