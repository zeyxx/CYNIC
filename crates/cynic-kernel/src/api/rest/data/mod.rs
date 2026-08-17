//! REST API handlers for data access — /crystals, /metrics/usage, /observations, /audit.

pub mod audit;
pub mod compliance;
pub mod crystals;
pub mod observations;
pub mod usage;

#[cfg(test)]
mod tests;

pub use audit::{audit_handler, proof_of_history_handler, verdicts_handler, AuditQuery, ProofOfHistoryQuery, VerdictsQuery};
pub use compliance::{compliance_handler, compliance_trend_handler, ComplianceTrendQuery};
pub use crystals::{
    create_crystal_handler, crystal_handler, crystals_handler, delete_crystal_handler,
    observe_crystal_hypha_handler, shatter_crystal_handler, CreateCrystalRequest, CrystalsQuery,
    ObserveCrystalHyphaRequest, ShatterCrystalRequest,
};
pub use observations::{
    observations_handler, public_observations_handler, sessions_handler, ObservationsQuery,
    PublicObservationsQuery, SessionsQuery,
};
pub use usage::usage_handler;
