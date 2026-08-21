//! Contribution tracking for E-Score aggregation.
//!
//! This module is a placeholder. The contribution tracking logic
//! will be formalized in the E-Score specification.

/// Identifies a contribution source in the E-Score model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContributionSource {
    DirectObservation,
    Inferred,
    Delegated,
}

/// Records a single contribution event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContributionEvent {
    pub source: ContributionSource,
    pub weight_bps: u32,
}

impl ContributionEvent {
    pub fn new(source: ContributionSource, weight_bps: u32) -> Self {
        Self { source, weight_bps }
    }
}
