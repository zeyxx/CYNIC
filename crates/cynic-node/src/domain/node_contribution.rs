//! Node contribution tracking for inference domain.
//!
//! Placeholder stub. Logic will be formalized separately.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContributionKind {
    Observed,
    Computed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeContribution {
    pub kind: ContributionKind,
    pub value: u64,
}
