//! Bootstrap withdrawal law (step 5) — when the seed transport may be cut.
//!
//! Consumes the existing `route_convergence` contract (RuntimeFenced is the
//! sole normal-delivery path) and crosses it with the enrollment/membership
//! layers from `protocols/machine-enrollment-federation.md`:
//!
//! - L1 identity proved (enrollment read-back),
//! - L2 grant live with comfortable remaining life,
//! - L3 membership view stable (not Suspect/Lapsed/authority-held),
//! - L4 service-plane transport fresh (the plane that replaces the seed).
//!
//! Law: withdrawal is a *plan*, never an action. The removal conditions of
//! `bootstrap_containment.removal_requires` (registry) must each be cited by
//! an evidence reference; anything less stays `Retain`. Retention of the
//! bootstrap after convergence is debt, not safety — but it is *declared*
//! debt (`Retain{reason}`), never silent.
//!
//! Pure module: no I/O, no execution. Adapters execute; this only decides.

use serde::{Deserialize, Serialize};

pub const BOOTSTRAP_WITHDRAWAL_SCHEMA: &str = "cynic.bootstrap_withdrawal.v1";

use super::route_convergence::BuildRunnerRouteState;
use cynic_protocol::MemberStatus;

/// Evidence inputs crossing all four layers. Each field is a proof reference
/// plus its observation instant; freshness windows are consumer-owned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithdrawalEvidence {
    /// L2: remaining grant life in seconds (from the node's own read-back).
    pub grant_remaining_life_s: u64,
    /// L2: minimum comfortable life required before cutting the seed path.
    pub grant_min_remaining_life_s: u64,
    /// L3: current membership status of the node (its own view or hub's).
    pub membership_status: MemberStatus,
    /// L4: age of the freshest healthy service-plane evidence, ms.
    pub service_plane_age_ms: Option<u64>,
    /// L4: maximum age at which service-plane evidence still counts.
    pub service_plane_max_age_ms: u64,
}

/// The decision itself: retain or withdraw, always with reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum BootstrapWithdrawalDecision {
    /// Every condition holds; the bootstrap transport may be retired by the
    /// operator/adapter owning it. This authorizes nothing by itself.
    Withdraw { evidence_refs: Vec<String> },
    /// A condition is missing or regressed. The bootstrap stays until the
    /// named gaps close. Retention beyond convergence is declared debt.
    Retain {
        reason: String,
        missing_conditions: Vec<String>,
    },
}

impl BootstrapWithdrawalDecision {
    pub fn may_withdraw(&self) -> bool {
        matches!(self, BootstrapWithdrawalDecision::Withdraw { .. })
    }
}

/// Pure withdrawal rule.
///
/// Conditions (all required, fail-closed on any unknown):
/// 1. route state reached `RuntimeFenced` — workload delivery no longer
///    consults the seed path (route_convergence contract);
/// 2. L2 grant outlives the seed comfortably (remaining > minimum);
/// 3. L3 membership `Active` — a member whose coordination trust is withdrawn
///    must not also lose its rescue path;
/// 4. L4 service-plane evidence exists, healthy, and fresh.
pub fn reconcile_bootstrap_withdrawal(
    route_state: &BuildRunnerRouteState,
    evidence: &WithdrawalEvidence,
) -> BootstrapWithdrawalDecision {
    let mut missing = Vec::new();
    let mut refs = Vec::new();

    match route_state {
        BuildRunnerRouteState::RuntimeFenced => {
            refs.push("route_convergence:runtime_fenced".to_string());
        }
        other => missing.push(format!("route_not_runtime_fenced:{other:?}")),
    }

    if evidence.grant_remaining_life_s > evidence.grant_min_remaining_life_s {
        refs.push("l2_grant_life_comfortable".to_string());
    } else {
        missing.push("l2_grant_life_below_minimum".to_string());
    }

    match &evidence.membership_status {
        MemberStatus::Active => refs.push("l3_membership_active".to_string()),
        status => missing.push(format!("l3_membership_not_active:{}", status_name(status))),
    }

    match evidence.service_plane_age_ms {
        Some(age) if age <= evidence.service_plane_max_age_ms => {
            refs.push("l4_service_plane_fresh".to_string());
        }
        Some(age) => missing.push(format!("l4_service_plane_stale_age_ms:{age}")),
        None => missing.push("l4_service_plane_no_healthy_evidence".to_string()),
    }

    if missing.is_empty() {
        BootstrapWithdrawalDecision::Withdraw {
            evidence_refs: refs,
        }
    } else {
        BootstrapWithdrawalDecision::Retain {
            reason: "withdrawal_conditions_unmet".to_string(),
            missing_conditions: missing,
        }
    }
}

fn status_name(status: &MemberStatus) -> &'static str {
    match status {
        MemberStatus::Active => "active",
        MemberStatus::Suspect { .. } => "suspect",
        MemberStatus::Lapsed { .. } => "lapsed",
        MemberStatus::Suspended { .. } => "suspended",
        MemberStatus::Expelled { .. } => "expelled",
        MemberStatus::Unknown { .. } => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence() -> WithdrawalEvidence {
        WithdrawalEvidence {
            grant_remaining_life_s: 7 * 24 * 3600,
            grant_min_remaining_life_s: 24 * 3600,
            membership_status: MemberStatus::Active,
            service_plane_age_ms: Some(5_000),
            service_plane_max_age_ms: 30_000,
        }
    }

    #[test]
    fn all_conditions_met_yields_withdraw_with_full_evidence_chain() {
        let d = reconcile_bootstrap_withdrawal(&BuildRunnerRouteState::RuntimeFenced, &evidence());
        match &d {
            BootstrapWithdrawalDecision::Withdraw { evidence_refs } => {
                assert_eq!(evidence_refs.len(), 4);
                assert!(evidence_refs.contains(&"route_convergence:runtime_fenced".to_string()));
                assert!(evidence_refs.contains(&"l4_service_plane_fresh".to_string()));
            }
            other => panic!("expected withdraw, got {other:?}"),
        }
        assert!(d.may_withdraw());
    }

    #[test]
    fn every_non_fenced_route_state_retains_the_bootstrap() {
        for state in [
            BuildRunnerRouteState::Unroutable,
            BuildRunnerRouteState::ProviderObserved,
            BuildRunnerRouteState::SshTrustVerified,
            BuildRunnerRouteState::SeedAdmitted,
            BuildRunnerRouteState::SeedProved,
            BuildRunnerRouteState::BuilderRuntimeObserved,
            BuildRunnerRouteState::Blocked,
        ] {
            let d = reconcile_bootstrap_withdrawal(&state, &evidence());
            assert!(!d.may_withdraw(), "{state:?} must not authorize withdrawal");
            match &d {
                BootstrapWithdrawalDecision::Retain {
                    missing_conditions, ..
                } => assert!(
                    missing_conditions
                        .iter()
                        .any(|c| c.starts_with("route_not_runtime_fenced")),
                    "{state:?}: {missing_conditions:?}"
                ),
                other => panic!("{state:?}: unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn uncomfortable_grant_life_retains_even_when_fenced() {
        let mut ev = evidence();
        ev.grant_remaining_life_s = 600; // below the 24h minimum
        let d = reconcile_bootstrap_withdrawal(&BuildRunnerRouteState::RuntimeFenced, &ev);
        assert!(!d.may_withdraw());
        match d {
            BootstrapWithdrawalDecision::Retain {
                missing_conditions, ..
            } => assert!(missing_conditions.contains(&"l2_grant_life_below_minimum".to_string())),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn non_active_membership_blocks_withdrawal_rescue_path_law() {
        for status in [
            MemberStatus::Suspect {
                reason: "s".to_string(),
            },
            MemberStatus::Lapsed {
                reason: "l".to_string(),
            },
            MemberStatus::Suspended {
                reason: "h".to_string(),
            },
            MemberStatus::Unknown {
                reason: "u".to_string(),
            },
            MemberStatus::Expelled {
                reason: "e".to_string(),
            },
        ] {
            let mut ev = evidence();
            ev.membership_status = status;
            let d = reconcile_bootstrap_withdrawal(&BuildRunnerRouteState::RuntimeFenced, &ev);
            assert!(!d.may_withdraw(), "{ev:?} must retain");
        }
    }

    #[test]
    fn stale_or_absent_service_plane_evidence_fails_closed() {
        let mut ev = evidence();
        ev.service_plane_age_ms = Some(60_000); // > 30s max
        assert!(
            !reconcile_bootstrap_withdrawal(&BuildRunnerRouteState::RuntimeFenced, &ev)
                .may_withdraw()
        );
        ev.service_plane_age_ms = None;
        let d = reconcile_bootstrap_withdrawal(&BuildRunnerRouteState::RuntimeFenced, &ev);
        match d {
            BootstrapWithdrawalDecision::Retain {
                missing_conditions, ..
            } => assert!(
                missing_conditions.contains(&"l4_service_plane_no_healthy_evidence".to_string())
            ),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn boundary_exact_minimum_life_is_not_comfortable() {
        let mut ev = evidence();
        ev.grant_remaining_life_s = ev.grant_min_remaining_life_s;
        // Law uses strict `>`: exactly-at-minimum still retains (conservative).
        let d = reconcile_bootstrap_withdrawal(&BuildRunnerRouteState::RuntimeFenced, &ev);
        assert!(!d.may_withdraw());
    }

    #[test]
    fn withdrawal_decision_is_never_an_execution_authority() {
        // The decision carries evidence refs only; it has no argv, no
        // capability, no mutation flag. Structural guarantee by construction:
        // assert the Withdraw variant's shape stays evidence-only.
        let d = reconcile_bootstrap_withdrawal(&BuildRunnerRouteState::RuntimeFenced, &evidence());
        let json = serde_json::to_string(&d).expect("serialize");
        assert!(
            !json.contains("\"argv\"") && !json.contains("capability"),
            "withdrawal must stay a plan, not an action: {json}"
        );
    }
}
