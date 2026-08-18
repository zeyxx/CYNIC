//! REST API handlers for data access — /crystals, /metrics/usage, /observations, /audit.

pub mod audit;
pub mod compliance;
pub mod crystals;
pub mod observations;
pub mod usage;

#[cfg(test)]
mod tests;

pub use audit::{
    AuditQuery, ProofOfHistoryQuery, VerdictsQuery, audit_handler, proof_of_history_handler,
    verdicts_handler,
};
pub use compliance::{ComplianceTrendQuery, compliance_handler, compliance_trend_handler};
pub use crystals::{
    CreateCrystalRequest, CrystalsQuery, ObserveCrystalHyphaRequest, ShatterCrystalRequest,
    create_crystal_handler, crystal_handler, crystals_handler, delete_crystal_handler,
    observe_crystal_hypha_handler, shatter_crystal_handler,
};
pub use observations::{
    ObservationsQuery, PublicObservationsQuery, SessionsQuery, observations_handler,
    public_observations_handler, sessions_handler,
};
pub use usage::usage_handler;
