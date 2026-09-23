//! E-Score contribution scoring domain.
//!
//! Pure domain: ScoreUnit arithmetic, EvidenceTier weighting, decay, merkle aggregation.
//! No I/O, no adapters. Adapters live in `cynic-intel-adapters`.

pub mod aggregator;
pub mod decay;
pub mod merkle;
pub mod metrics;

pub use cynic_intel::escore::types::*;
