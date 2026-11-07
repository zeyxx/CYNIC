//! Canonical authority enum — the single source of truth for
//! authority levels across build and runtime surfaces.
//!
//! Per AGENTS.md invariant #6: one canonical decision source per concern.
//! Authority is one concern; it has one type (`Authority`).
//!
//! The environment stage paired with this authority is the canonical
//! [`DeploymentTier`](crate::deployment_tier::DeploymentTier)
//! (`LocalLab` / `Conformance` / `Staging` / `Canary` / `Production`),
//! not a parallel scope enum. The legacy `Observe` spelling maps to
//! `LocalLab` through `DeploymentTier::parse_flexible`.
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

use crate::deployment_tier::DeploymentTier;

/// Canonical authority level for any build or runtime surface.
///
/// Ordered from weakest to strongest. A stronger authority can always do
/// everything a weaker one can, but not the reverse.
///
/// Used by:
/// - `cynic-protocol::harness` (build scenarios)
/// - `cynic-host::runtime_harness` (runtime progression)
/// - `cynic-registry::harness_progression` (progression decision)
/// - `cynic-cli` surfaces (projection only)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Authority {
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

impl Authority {
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

    /// Returns the minimum authority required for the given deployment tier,
    /// per AGENTS.md § Runtime Harness progression rules.
    pub fn minimum_for_scope(tier: DeploymentTier) -> Self {
        match tier {
            DeploymentTier::LocalLab => Self::TestProjection,
            DeploymentTier::Conformance => Self::ConformanceGate,
            DeploymentTier::Staging => Self::ReleaseCandidate,
            DeploymentTier::Canary => Self::FleetCanary,
            DeploymentTier::Production => Self::RuntimeAuthority,
        }
    }
}

/// Pure predicate: does `held` authority satisfy `required`?
///
/// This is the single canonical check. Surfaces must not re-derive it.
pub fn authority_satisfies(held: Authority, required: Authority) -> bool {
    held >= required
}

/// Pure predicate: does `held` authority contain `other`?
///
/// Equivalent to `authority_satisfies`. Provided for symmetry with
/// `AgentScope::contains` where containment is the semantic frame.
pub fn authority_contains(held: Authority, other: Authority) -> bool {
    held >= other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_weak_to_strong() {
        assert!(Authority::TestProjection < Authority::ConformanceGate);
        assert!(Authority::ConformanceGate < Authority::ReleaseCandidate);
        assert!(Authority::ReleaseCandidate < Authority::FleetCanary);
        assert!(Authority::FleetCanary < Authority::RuntimeAuthority);
    }

    #[test]
    fn stronger_contains_weaker() {
        assert!(authority_contains(
            Authority::RuntimeAuthority,
            Authority::TestProjection
        ));
        assert!(authority_contains(
            Authority::FleetCanary,
            Authority::ConformanceGate
        ));
        assert!(!authority_contains(
            Authority::TestProjection,
            Authority::RuntimeAuthority
        ));
    }

    #[test]
    fn equal_authority_contains_itself() {
        assert!(authority_contains(
            Authority::ConformanceGate,
            Authority::ConformanceGate
        ));
    }

    #[test]
    fn mutate_real_state_requires_canary_or_above() {
        assert!(!Authority::TestProjection.can_mutate_real_state());
        assert!(!Authority::ConformanceGate.can_mutate_real_state());
        assert!(!Authority::ReleaseCandidate.can_mutate_real_state());
        assert!(Authority::FleetCanary.can_mutate_real_state());
        assert!(Authority::RuntimeAuthority.can_mutate_real_state());
    }

    #[test]
    fn minimum_for_mutation_class_matches_progression() {
        assert_eq!(
            Authority::minimum_for_scope(DeploymentTier::LocalLab),
            Authority::TestProjection
        );
        assert_eq!(
            Authority::minimum_for_scope(DeploymentTier::Production),
            Authority::RuntimeAuthority
        );
    }

    #[test]
    fn legacy_observe_spelling_maps_to_local_lab() {
        // `Observe` was the legacy first scope variant; its canonical
        // replacement is `DeploymentTier::LocalLab` via parse_flexible.
        assert_eq!(
            DeploymentTier::parse_flexible("observe"),
            Some(DeploymentTier::LocalLab)
        );
        assert_eq!(
            Authority::minimum_for_scope(
                DeploymentTier::parse_flexible("observe").expect("observe parses")
            ),
            Authority::TestProjection
        );
    }

    #[test]
    fn serde_round_trip() {
        let json = serde_json::to_string(&Authority::FleetCanary).unwrap();
        let back: Authority = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Authority::FleetCanary);
    }
}
