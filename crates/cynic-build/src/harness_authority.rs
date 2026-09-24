//! Canonical harness authority enum — the single source of truth for
//! authority levels across build harness and runtime harness.
//!
//! This module exists to eliminate the duplicate `HarnessAuthority` enums
//! that previously lived in `cynic-protocol::harness` (4 variants) and
//! `cynic-host::runtime_harness` (5 variants). Those enums had only
//! `ConformanceGate` in common, making authority comparison between build
//! and runtime surfaces impossible.
//!
//! Per AGENTS.md invariant #6: one canonical decision source per concern.
//! Authority is one concern; it has one type.
//!
//! # Mapping from legacy enums
//!
//! | Build harness (legacy)   | Runtime harness (legacy) | Canonical (this type) |
//! |--------------------------|--------------------------|-----------------------|
//! | `ReadOnlyProjection`     | `TestProjection`          | `TestProjection`      |
//! | `ConformanceGate`        | `ConformanceGate`        | `ConformanceGate`    |
//! | `StagingObservation`     | `ReleaseCandidate`       | `ReleaseCandidate`   |
//! | —                        | `FleetCanary`            | `FleetCanary`        |
//! | `ExplicitCanary`         | —                        | `FleetCanary`        |
//! | —                        | `RuntimeAuthority`       | `RuntimeAuthority`   |

use serde::{Deserialize, Serialize};

/// Canonical authority level for any harness (build or runtime).
///
/// Ordered from weakest to strongest. A stronger authority can always do
/// everything a weaker one can, but not the reverse.
///
/// Used by:
/// - `cynic-protocol::harness` (build scenarios)
/// - `cynic-host::runtime_harness` (runtime progression)
/// - `cynic-host::harness_progression` (progression decision)
/// - `cynic-cli` surfaces (projection only)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessAuthority {
    /// Read-only test projection. No mutation authority.
    /// Maps from: `harness::ReadOnlyProjection`, `runtime_harness::TestProjection`.
    TestProjection,

    /// Portable conformance gate. No real mutation; observes and qualifies.
    /// Common to both legacy enums.
    ConformanceGate,

    /// Release candidate. Staging-like, production-observation only.
    /// Maps from: `harness::StagingObservation`, `runtime_harness::ReleaseCandidate`.
    ReleaseCandidate,

    /// Bounded fleet canary. Real mutation with rollback, bounded blast radius.
    /// Maps from: `harness::ExplicitCanary`, `runtime_harness::FleetCanary`.
    FleetCanary,

    /// Declared runtime authority. Full mutation authority with SLO tracking.
    /// Maps from: `runtime_harness::RuntimeAuthority`.
    RuntimeAuthority,
}

impl HarnessAuthority {
    /// Weakest authority — read-only test projection.
    pub const WEAKEST: Self = Self::TestProjection;

    /// Strongest authority — declared runtime production authority.
    pub const STRONGEST: Self = Self::RuntimeAuthority;

    /// Whether this authority may mutate real (non-test) state.
    pub fn can_mutate_real_state(self) -> bool {
        matches!(self, Self::FleetCanary | Self::RuntimeAuthority)
    }

    /// Whether this authority is a test projection (no real mutation).
    pub fn is_test_projection(self) -> bool {
        matches!(self, Self::TestProjection)
    }

    /// Whether this authority is production runtime authority.
    pub fn is_runtime_authority(self) -> bool {
        matches!(self, Self::RuntimeAuthority)
    }

    /// Returns the minimum authority required for the given harness scope,
    /// per AGENTS.md § Runtime Harness progression rules.
    pub fn minimum_for_scope(scope: HarnessScope) -> Self {
        match scope {
            HarnessScope::Observe => Self::TestProjection,
            HarnessScope::Conformance => Self::ConformanceGate,
            HarnessScope::Staging => Self::ReleaseCandidate,
            HarnessScope::Canary => Self::FleetCanary,
            HarnessScope::Production => Self::RuntimeAuthority,
        }
    }
}

/// Harness scope — what tier of mutation is being attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessScope {
    Observe,
    Conformance,
    Staging,
    Canary,
    Production,
}

/// Pure predicate: does `held` authority satisfy `required`?
///
/// This is the single canonical check. Surfaces must not re-derive it.
pub fn authority_satisfies(held: HarnessAuthority, required: HarnessAuthority) -> bool {
    held >= required
}

/// Pure predicate: does `held` authority contain `other`?
///
/// Equivalent to `authority_satisfies`. Provided for symmetry with
/// `AgentScope::contains` where containment is the semantic frame.
pub fn authority_contains(held: HarnessAuthority, other: HarnessAuthority) -> bool {
    held >= other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_weak_to_strong() {
        assert!(HarnessAuthority::TestProjection < HarnessAuthority::ConformanceGate);
        assert!(HarnessAuthority::ConformanceGate < HarnessAuthority::ReleaseCandidate);
        assert!(HarnessAuthority::ReleaseCandidate < HarnessAuthority::FleetCanary);
        assert!(HarnessAuthority::FleetCanary < HarnessAuthority::RuntimeAuthority);
    }

    #[test]
    fn stronger_contains_weaker() {
        assert!(authority_contains(
            HarnessAuthority::RuntimeAuthority,
            HarnessAuthority::TestProjection
        ));
        assert!(authority_contains(
            HarnessAuthority::FleetCanary,
            HarnessAuthority::ConformanceGate
        ));
        assert!(!authority_contains(
            HarnessAuthority::TestProjection,
            HarnessAuthority::RuntimeAuthority
        ));
    }

    #[test]
    fn equal_authority_contains_itself() {
        assert!(authority_contains(
            HarnessAuthority::ConformanceGate,
            HarnessAuthority::ConformanceGate
        ));
    }

    #[test]
    fn mutate_real_state_requires_canary_or_above() {
        assert!(!HarnessAuthority::TestProjection.can_mutate_real_state());
        assert!(!HarnessAuthority::ConformanceGate.can_mutate_real_state());
        assert!(!HarnessAuthority::ReleaseCandidate.can_mutate_real_state());
        assert!(HarnessAuthority::FleetCanary.can_mutate_real_state());
        assert!(HarnessAuthority::RuntimeAuthority.can_mutate_real_state());
    }

    #[test]
    fn minimum_for_mutation_class_matches_progression() {
        assert_eq!(
            HarnessAuthority::minimum_for_scope(HarnessScope::Observe),
            HarnessAuthority::TestProjection
        );
        assert_eq!(
            HarnessAuthority::minimum_for_scope(HarnessScope::Production),
            HarnessAuthority::RuntimeAuthority
        );
    }

    #[test]
    fn serde_round_trip() {
        let json = serde_json::to_string(&HarnessAuthority::FleetCanary).unwrap();
        let back: HarnessAuthority = serde_json::from_str(&json).unwrap();
        assert_eq!(back, HarnessAuthority::FleetCanary);
    }
}
