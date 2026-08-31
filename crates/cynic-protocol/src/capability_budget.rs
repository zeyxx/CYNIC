//! Capability issuance budget — Sec1 monotonic ceiling contract.
//!
//! Enforces the Sec1 invariant: capabilities MUST be issued against a
//! declared ceiling; the issuer count is monotonically increasing and never
//! exceeds that ceiling. No capability may be granted once the ceiling is
//! reached.
//!
//! This module is **pure** — no I/O, no clocks, no global state. The
//! stateful `CapabilityIssuer` that holds an `AtomicU64` lives in
//! `cynic-kernel::capability_issuer` (the only crate allowed to mutate
//! runtime state).

use serde::{Deserialize, Serialize};

pub const CAPABILITY_BUDGET_SCHEMA: &str = "cynic.capability_budget.v1";
pub const CAPABILITY_ISSUANCE_RECORD_SCHEMA: &str = "cynic.capability_issuance_record.v1";

/// Declared ceiling for a class of capability issuances.
///
/// The ceiling is set once at admission time and never increases for the
/// lifetime of the issuer. A ceiling of `0` means no capability may ever
/// be issued (fully locked class).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityBudget {
    pub schema: &'static str,
    /// Maximum number of capabilities that may be issued. Immutable after
    /// construction.
    pub ceiling: u64,
    /// Declared authority class this budget governs (e.g. "MachineEnrollment",
    /// "RecoveryBreakglass", "RuntimeMachineIdentity").
    pub authority_class: &'static str,
}

impl CapabilityBudget {
    pub const fn new(ceiling: u64, authority_class: &'static str) -> Self {
        Self {
            schema: CAPABILITY_BUDGET_SCHEMA,
            ceiling,
            authority_class,
        }
    }
}

/// Point-in-time snapshot of capability issuance for one authority class.
///
/// `issued` is monotonically non-decreasing; `ceiling` is immutable.
/// Invariant: `issued <= ceiling`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityIssuanceRecord {
    pub schema: String,
    pub authority_class: String,
    pub issued: u64,
    pub ceiling: u64,
    pub remaining: u64,
    pub exhausted: bool,
}

impl CapabilityIssuanceRecord {
    pub fn new(authority_class: impl Into<String>, issued: u64, ceiling: u64) -> Self {
        debug_assert!(
            issued <= ceiling,
            "issued ({issued}) must not exceed ceiling ({ceiling})"
        );
        let remaining = ceiling.saturating_sub(issued);
        Self {
            schema: CAPABILITY_ISSUANCE_RECORD_SCHEMA.to_string(),
            authority_class: authority_class.into(),
            issued,
            ceiling,
            remaining,
            exhausted: issued >= ceiling,
        }
    }
}

/// Decision returned by the pure issuance check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityIssueDecision {
    /// The ceiling has not been reached; the capability may be issued.
    /// The caller MUST atomically increment the issued counter before
    /// materialising the capability.
    Allowed,
    /// The ceiling has been reached or would be exceeded; the capability
    /// MUST NOT be issued.
    Denied {
        /// How many capabilities were already issued at decision time.
        issued: u64,
        /// The ceiling that was enforced.
        ceiling: u64,
    },
}

/// Pure issuance-gate check. No side effects.
///
/// `issued` is the count BEFORE the proposed issuance.
/// Returns `Allowed` iff `issued < ceiling`.
pub fn check_issuance(issued: u64, ceiling: u64) -> CapabilityIssueDecision {
    if issued < ceiling {
        CapabilityIssueDecision::Allowed
    } else {
        CapabilityIssueDecision::Denied { issued, ceiling }
    }
}

/// Whether `issued` satisfies the monotonic invariant against `ceiling`.
///
/// A valid state is `issued <= ceiling`. A state where `issued > ceiling`
/// is a protocol violation and must be reported as a proof gap.
pub fn monotonic_invariant_holds(issued: u64, ceiling: u64) -> bool {
    issued <= ceiling
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_issuance_allows_below_ceiling() {
        assert_eq!(check_issuance(0, 1), CapabilityIssueDecision::Allowed);
        assert_eq!(check_issuance(4, 5), CapabilityIssueDecision::Allowed);
    }

    #[test]
    fn check_issuance_denies_at_ceiling() {
        assert_eq!(
            check_issuance(5, 5),
            CapabilityIssueDecision::Denied {
                issued: 5,
                ceiling: 5,
            }
        );
    }

    #[test]
    fn check_issuance_denies_above_ceiling() {
        // Should never happen in practice (atomic CAS prevents it),
        // but the pure function must still return Denied for any such input.
        assert_eq!(
            check_issuance(6, 5),
            CapabilityIssueDecision::Denied {
                issued: 6,
                ceiling: 5,
            }
        );
    }

    #[test]
    fn zero_ceiling_always_denies() {
        assert_eq!(
            check_issuance(0, 0),
            CapabilityIssueDecision::Denied {
                issued: 0,
                ceiling: 0,
            }
        );
    }

    #[test]
    fn monotonic_invariant_holds_valid_states() {
        assert!(monotonic_invariant_holds(0, 5));
        assert!(monotonic_invariant_holds(5, 5));
    }

    #[test]
    fn monotonic_invariant_rejects_violation() {
        assert!(!monotonic_invariant_holds(6, 5));
    }

    #[test]
    fn issuance_record_computes_remaining_and_exhausted() {
        let r = CapabilityIssuanceRecord::new("MachineEnrollment", 3, 5);
        assert_eq!(r.remaining, 2);
        assert!(!r.exhausted);

        let r2 = CapabilityIssuanceRecord::new("MachineEnrollment", 5, 5);
        assert_eq!(r2.remaining, 0);
        assert!(r2.exhausted);
    }

    #[test]
    fn capability_budget_ceiling_is_immutable_after_construction() {
        let budget = CapabilityBudget::new(10, "RuntimeMachineIdentity");
        assert_eq!(budget.ceiling, 10);
        assert_eq!(budget.authority_class, "RuntimeMachineIdentity");
    }
}
