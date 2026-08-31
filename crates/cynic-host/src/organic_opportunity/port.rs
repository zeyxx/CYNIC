//! Port definition for organic opportunity observation.
//!
//! Enforces two core framework laws:
//! 1. "Each adapter reports unavailable sources distinctly from empty evidence."
//! 2. "Raw session text is not retained in the resulting opportunity report."
//!
//! Substrate (files, processes, runner) is injected via traits; no hardcoded
//! machine paths live here.

use cynic_protocol::organic_opportunity::{OpportunitySourceKind, OrganicOpportunity};
use serde::{Deserialize, Serialize};

/// Observation status for one evidence family.
///
/// Distinguishes unavailable sources (missing files, blocked daemon, permission denied)
/// from genuinely empty evidence (observed zero opportunities).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum SourceObservationStatus<T> {
    /// Source was observed successfully, yielding typed evidence.
    Observed { data: T },
    /// Source is legitimately empty (no opportunities detected).
    Empty,
    /// Source was requested but unavailable; residual unknown preserved.
    Unavailable {
        kind: OpportunitySourceKind,
        reason: String,
    },
}

impl<T> SourceObservationStatus<T> {
    pub fn is_unavailable(&self) -> bool {
        matches!(self, SourceObservationStatus::Unavailable { .. })
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, SourceObservationStatus::Empty)
    }

    pub fn observed_data(self) -> Option<T> {
        match self {
            SourceObservationStatus::Observed { data } => Some(data),
            _ => None,
        }
    }
}

/// Port trait implemented by all host opportunity observation adapters.
pub trait OrganicOpportunityObserver: Send + Sync {
    /// Name of this observer for logging and audit attribution.
    fn observer_name(&self) -> &'static str;

    /// The source kind this observer specializes in.
    fn source_kind(&self) -> OpportunitySourceKind;

    /// Observe opportunities without mutating source or runtime state.
    fn observe(&self) -> SourceObservationStatus<Vec<OrganicOpportunity>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_is_distinct_from_empty() {
        let empty: SourceObservationStatus<Vec<OrganicOpportunity>> =
            SourceObservationStatus::Empty;
        let unavail: SourceObservationStatus<Vec<OrganicOpportunity>> =
            SourceObservationStatus::Unavailable {
                kind: OpportunitySourceKind::SessionTrace,
                reason: "directory not found".to_string(),
            };

        assert!(!empty.is_unavailable());
        assert!(empty.is_empty());
        assert!(unavail.is_unavailable());
        assert!(!unavail.is_empty());
    }
}
