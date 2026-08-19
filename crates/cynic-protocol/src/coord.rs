use serde::{Deserialize, Serialize};

use crate::LeaseGuard;

pub const LEASE_VERIFICATION_SCHEMA: &str = "cynic.coord.lease_verification.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseVerification {
    pub schema: String,
    pub status: String,
    pub guard: LeaseGuard,
    pub expires_at: Option<String>,
    pub blockers: Vec<String>,
}

impl LeaseVerification {
    pub fn proved(guard: LeaseGuard, expires_at: Option<String>) -> Self {
        Self {
            schema: LEASE_VERIFICATION_SCHEMA.to_string(),
            status: "proved".to_string(),
            guard,
            expires_at,
            blockers: Vec::new(),
        }
    }

    pub fn rejected(guard: LeaseGuard) -> Self {
        Self {
            schema: LEASE_VERIFICATION_SCHEMA.to_string(),
            status: "rejected".to_string(),
            guard,
            expires_at: None,
            blockers: vec!["lease_expired_superseded_or_guard_mismatch".to_string()],
        }
    }

    pub fn unavailable(guard: LeaseGuard, blocker: impl Into<String>) -> Self {
        Self {
            schema: LEASE_VERIFICATION_SCHEMA.to_string(),
            status: "unavailable".to_string(),
            guard,
            expires_at: None,
            blockers: vec![blocker.into()],
        }
    }
}

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
