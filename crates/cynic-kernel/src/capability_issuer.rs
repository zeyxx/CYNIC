//! Stateful capability issuer — Sec1 ceiling enforcement.
//!
//! This is the ONLY place in the codebase that holds the mutable issuance
//! counter for a capability class. All decisions are delegated to the pure
//! `cynic_protocol::capability_budget::check_issuance` function; the
//! kernel adapter owns only the `AtomicU64` issued counter.
//!
//! # Invariants
//!
//! - `issued` is monotonically non-decreasing (increment-only).
//! - `issued` never exceeds `ceiling` (CAS-checked before increment).
//! - `try_issue` is the only mutation surface; it never panics.
//! - The issuer is `Send + Sync`; all ops use `Ordering::SeqCst` for
//!   cross-thread visibility.

use std::sync::atomic::{AtomicU64, Ordering};

use cynic_protocol::capability_budget::{
    CAPABILITY_ISSUANCE_RECORD_SCHEMA, CapabilityBudget, CapabilityIssuanceRecord,
    CapabilityIssueDecision, check_issuance,
};

/// Error returned when a capability issuance is denied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuanceDenied {
    pub authority_class: &'static str,
    pub issued: u64,
    pub ceiling: u64,
}

impl std::fmt::Display for IssuanceDenied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "capability issuance denied for '{}': issued={} ceiling={}",
            self.authority_class, self.issued, self.ceiling
        )
    }
}

impl std::error::Error for IssuanceDenied {}

/// Thread-safe capability issuer with monotonic ceiling enforcement.
///
/// Create one `CapabilityIssuer` per authority class at service startup and
/// keep it for the process lifetime. The ceiling is immutable after
/// construction.
///
/// ```text
/// CapabilityIssuer::new(budget) → one per authority class, process lifetime
///   .try_issue() → Ok(()) = gate passed, caller may materialise the capability
///              → Err(IssuanceDenied) = ceiling reached, capability MUST NOT be issued
///   .snapshot() → CapabilityIssuanceRecord (non-mutating)
/// ```
#[derive(Debug)]
pub struct CapabilityIssuer {
    budget: CapabilityBudget,
    issued: AtomicU64,
}

impl CapabilityIssuer {
    /// Create a new issuer for the given budget. The issued counter starts at 0.
    pub fn new(budget: CapabilityBudget) -> Self {
        Self {
            budget,
            issued: AtomicU64::new(0),
        }
    }

    /// Attempt to claim one issuance slot.
    ///
    /// On success (`Ok(())`): the caller MUST materialise exactly one
    /// capability and record it durably before returning. The slot has been
    /// consumed regardless of what the caller does with it.
    ///
    /// On failure (`Err(IssuanceDenied)`): the ceiling is reached; no
    /// capability may be issued. The counter is unchanged.
    ///
    /// This uses a compare-and-swap loop to prevent TOCTOU races under
    /// concurrent issuance requests.
    pub fn try_issue(&self) -> Result<(), IssuanceDenied> {
        loop {
            let current = self.issued.load(Ordering::SeqCst);
            match check_issuance(current, self.budget.ceiling) {
                CapabilityIssueDecision::Allowed => {
                    // CAS: only increment if no other thread already did.
                    match self.issued.compare_exchange(
                        current,
                        current + 1,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => return Ok(()),
                        Err(_) => continue, // lost the race — retry
                    }
                }
                CapabilityIssueDecision::Denied { issued, ceiling } => {
                    return Err(IssuanceDenied {
                        authority_class: self.budget.authority_class,
                        issued,
                        ceiling,
                    });
                }
            }
        }
    }

    /// Non-mutating snapshot of current issuance state.
    pub fn snapshot(&self) -> CapabilityIssuanceRecord {
        let issued = self.issued.load(Ordering::SeqCst);
        CapabilityIssuanceRecord {
            schema: CAPABILITY_ISSUANCE_RECORD_SCHEMA.to_string(),
            authority_class: self.budget.authority_class.to_string(),
            issued,
            ceiling: self.budget.ceiling,
            remaining: self.budget.ceiling.saturating_sub(issued),
            exhausted: issued >= self.budget.ceiling,
        }
    }

    /// Current issued count (SeqCst load).
    pub fn issued(&self) -> u64 {
        self.issued.load(Ordering::SeqCst)
    }

    /// Configured ceiling (immutable).
    pub fn ceiling(&self) -> u64 {
        self.budget.ceiling
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget(ceiling: u64) -> CapabilityBudget {
        CapabilityBudget::new(ceiling, "TestClass")
    }

    #[test]
    fn single_issuance_below_ceiling() {
        let issuer = CapabilityIssuer::new(budget(3));
        assert!(issuer.try_issue().is_ok());
        assert_eq!(issuer.issued(), 1);
    }

    #[test]
    fn exhaustion_at_ceiling() {
        let issuer = CapabilityIssuer::new(budget(2));
        assert!(issuer.try_issue().is_ok());
        assert!(issuer.try_issue().is_ok());
        // ceiling reached
        let err = issuer.try_issue().unwrap_err();
        assert_eq!(err.issued, 2);
        assert_eq!(err.ceiling, 2);
        // counter unchanged after denial
        assert_eq!(issuer.issued(), 2);
    }

    #[test]
    fn zero_ceiling_denies_immediately() {
        let issuer = CapabilityIssuer::new(budget(0));
        let err = issuer.try_issue().unwrap_err();
        assert_eq!(err.issued, 0);
        assert_eq!(err.ceiling, 0);
        assert_eq!(issuer.issued(), 0);
    }

    #[test]
    fn snapshot_reflects_issued_and_remaining() {
        let issuer = CapabilityIssuer::new(budget(5));
        issuer.try_issue().unwrap();
        issuer.try_issue().unwrap();
        let snap = issuer.snapshot();
        assert_eq!(snap.issued, 2);
        assert_eq!(snap.ceiling, 5);
        assert_eq!(snap.remaining, 3);
        assert!(!snap.exhausted);
    }

    #[test]
    fn snapshot_marks_exhausted_when_ceiling_reached() {
        let issuer = CapabilityIssuer::new(budget(1));
        issuer.try_issue().unwrap();
        let snap = issuer.snapshot();
        assert!(snap.exhausted);
        assert_eq!(snap.remaining, 0);
    }

    #[test]
    fn monotonic_invariant_never_violated() {
        let issuer = CapabilityIssuer::new(budget(3));
        for _ in 0..10 {
            let _ = issuer.try_issue();
            let snap = issuer.snapshot();
            assert!(
                cynic_protocol::capability_budget::monotonic_invariant_holds(
                    snap.issued,
                    snap.ceiling
                ),
                "invariant violated: issued={} ceiling={}",
                snap.issued,
                snap.ceiling
            );
        }
    }

    #[test]
    fn concurrent_issuance_never_exceeds_ceiling() {
        use std::sync::Arc;
        use std::thread;

        let ceiling = 10u64;
        let issuer = Arc::new(CapabilityIssuer::new(budget(ceiling)));
        let threads: Vec<_> = (0..20)
            .map(|_| {
                let issuer = Arc::clone(&issuer);
                thread::spawn(move || issuer.try_issue())
            })
            .collect();

        let successes = threads
            .into_iter()
            .filter(|h| h.join().unwrap().is_ok())
            .count();

        assert_eq!(successes, ceiling as usize);
        assert_eq!(issuer.issued(), ceiling);
    }
}
