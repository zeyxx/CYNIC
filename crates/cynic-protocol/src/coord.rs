pub const LEASE_VERIFICATION_SCHEMA: &str = "cynic.coord.lease_verification.v1";

// Re-export canonical lease types from `coordination_lease` to avoid an
// ambiguous glob re-export with `pub use coord::*;` / `pub use coordination_lease::*;`.
pub use crate::coordination_lease::LeaseVerification;

// Re-export the canonical schema under a coord-specific alias only.
#[allow(dead_code)]
pub const COORD_LEASE_VERIFICATION_SCHEMA: &str = LEASE_VERIFICATION_SCHEMA;

pub fn coord_register_path() -> &'static str {
    "/coord/register"
}

pub fn coord_who_path() -> &'static str {
    "/coord/who"
}

pub fn coord_claim_path() -> &'static str {
    "/coord/claim"
}

pub fn coord_release_path() -> &'static str {
    "/coord/release"
}

pub fn coord_verify_path() -> &'static str {
    "/coord/verify"
}

pub fn coord_finalize_path() -> &'static str {
    "/coord/finalize"
}
