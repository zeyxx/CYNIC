//! Canonical hub-mode federation membership contract.
//!
//! Desired state, observation, reconciliation decision, and residual unknowns
//! for a hub-topology node fleet. The hub is the `kernel_primary` authority;
//! it is the single admission authority and therefore an explicit SPOF. That
//! risk is declared here (`HubAuthority` / `single_point_of_failure`) rather
//! than hidden. Quorum admission may later be added as an alternative
//! authority kind without breaking this schema (versioned, additive).
//!
//! Per AGENTS.md invariant #5: declarative desired state plus observed proof.
//! This module owns only pure types, predicates, and the reconcile rule.
//! Storage, transport, and CLI projection belong to other boundaries.

use crate::agent_citizenship::AgentBudgets;
use serde::{Deserialize, Serialize};

pub const FEDERATION_MEMBERSHIP_SCHEMA: &str = "cynic.federation_membership.v1";
pub const FEDERATION_MEMBER_SCHEMA: &str = "cynic.federation_member.v1";
pub const FEDERATION_DESIRED_SCHEMA: &str = "cynic.federation_desired.v1";

/// Typed fleet role. Canonical table lives in [`crate::federation_authz`];
/// this enum is its key. Registry role strings in `infra/registry.json`
/// must deserialize into one of these variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetRole {
    Hypervisor,
    CortexHost,
    KernelPrimary,
    StoragePrimary,
    BuildRunner,
    ForgePrimary,
}

/// Every fleet role. Exhaustive list used by consumers that must cover all
/// roles (authz table checks, projections); adding a variant is a contract
/// change that breaks array-literal users of this constant.
pub const ALL_FLEET_ROLES: [FleetRole; 6] = [
    FleetRole::Hypervisor,
    FleetRole::CortexHost,
    FleetRole::KernelPrimary,
    FleetRole::StoragePrimary,
    FleetRole::BuildRunner,
    FleetRole::ForgePrimary,
];

/// Membership lifecycle status.
///
/// Two orthogonal dimensions, deliberately kept apart (see
/// `protocols/machine-enrollment-federation.md`, layers L1/L2 vs L3):
///
/// - **liveness** (`Active` / `Suspect` / `Lapsed` / `Unknown`) — derived
///   from observed heartbeats against [`MembershipPolicy`] thresholds.
///   Refutable: a fresh acknowledgment returns the member to `Active`.
/// - **authority** (`Suspended` / `Expelled`) — set only by an explicit
///   authority action (operator hold, desired-state removal, grant revoke),
///   never by a timer. Sticky under liveness observations.
///
/// `Unknown` is data: it means the hub has no fresh read-back for the
/// member, not that the member is healthy or dead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MemberStatus {
    Active,
    /// Heartbeat silent past the suspect threshold; refutable by the member.
    Suspect {
        reason: String,
    },
    /// Silence continued past the lapsed threshold. Identity (L1) and grant
    /// (L2) are untouched; only coordination trust is withdrawn.
    Lapsed {
        reason: String,
    },
    Suspended {
        reason: String,
    },
    Expelled {
        reason: String,
    },
    Unknown {
        reason: String,
    },
}

impl MemberStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, MemberStatus::Active)
    }

    /// Whether a fresh acknowledgment may clear this status back to `Active`.
    /// Authority states are never cleared by liveness evidence.
    pub fn is_refutable_by_acknowledgment(&self) -> bool {
        matches!(
            self,
            MemberStatus::Active
                | MemberStatus::Suspect { .. }
                | MemberStatus::Lapsed { .. }
                | MemberStatus::Unknown { .. }
        )
    }
}

/// Observed liveness input of the membership transition rule. Pure data: the
/// caller (node-local tick, hub projection) measures the transport and feeds
/// the result here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessObservation {
    /// Fresh heartbeat read-back at or after `last_seen`.
    Acknowledged,
    /// No heartbeat observed for `elapsed_ms` (monotonic measurement).
    SilentFor { elapsed_ms: u64 },
}

/// Default liveness thresholds (D3, 2026-08-25): k8s-style grace band, SWIM
/// style refutable suspect stage. Calibration belongs to registry policy
/// data, never to WireGuard tunnel watchdogs (L4 owns those separately).
pub const DEFAULT_PROBE_PERIOD_MS: u64 = 10_000;
pub const DEFAULT_SUSPECT_AFTER_MS: u64 = 40_000;
pub const DEFAULT_LAPSED_AFTER_MS: u64 = 180_000;
pub const MEMBERSHIP_POLICY_SCHEMA: &str = "cynic.membership_policy.v1";

/// Injectable liveness policy for membership reconciliation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipPolicy {
    pub schema: String,
    pub probe_period_ms: u64,
    pub suspect_after_ms: u64,
    pub lapsed_after_ms: u64,
}

impl Default for MembershipPolicy {
    fn default() -> Self {
        MembershipPolicy {
            schema: MEMBERSHIP_POLICY_SCHEMA.to_string(),
            probe_period_ms: DEFAULT_PROBE_PERIOD_MS,
            suspect_after_ms: DEFAULT_SUSPECT_AFTER_MS,
            lapsed_after_ms: DEFAULT_LAPSED_AFTER_MS,
        }
    }
}

/// Structural validation result of a [`MembershipPolicy`]: the blocker list
/// is empty exactly when the policy is usable.
pub type MembershipPolicyState = Vec<String>;

impl MembershipPolicy {
    /// Pure structural validation. Ordering law: probe < suspect < lapsed,
    /// every field strictly positive.
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != MEMBERSHIP_POLICY_SCHEMA {
            blockers.push("membership_policy_schema_invalid".to_string());
        }
        if self.probe_period_ms == 0 || self.suspect_after_ms == 0 || self.lapsed_after_ms == 0 {
            blockers.push("membership_policy_zero_threshold".to_string());
        }
        if self.probe_period_ms >= self.suspect_after_ms {
            blockers.push("membership_policy_probe_not_finer_than_suspect".to_string());
        }
        if self.suspect_after_ms >= self.lapsed_after_ms {
            blockers.push("membership_policy_suspect_not_below_lapsed".to_string());
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

/// Hub-granted membership lease. Mirrors claim lease semantics:
/// monotonic fencing token, bounded window, heartbeat-renewed expiry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberLease {
    pub granted_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub fencing_token: u64,
}

impl MemberLease {
    pub fn is_live_at(&self, now_unix_ms: u64) -> bool {
        now_unix_ms < self.expires_at_unix_ms
    }

    pub fn is_heartbeat_stale_at(&self, now_unix_ms: u64) -> bool {
        let ttl = self
            .expires_at_unix_ms
            .saturating_sub(self.granted_at_unix_ms);
        now_unix_ms >= self.granted_at_unix_ms.saturating_add(ttl * 8 / 10)
    }
}

/// One fleet member as the hub's durable record of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederationMember {
    pub schema: String,
    pub node_id: String,
    pub roles: Vec<FleetRole>,
    /// Public-key fingerprint of the enrolled machine identity.
    pub identity_fingerprint: String,
    pub endpoint: Option<String>,
    pub status: MemberStatus,
    pub lease: MemberLease,
    /// Hub authority id that admitted this member (must be the hub).
    pub admitted_by: String,
    /// Budget delegated to this member by the hub. Child budgets must stay
    /// within what the hub itself holds (monotonic, never amplified).
    pub budget: AgentBudgets,
    pub consumed: AgentBudgets,
}

impl FederationMember {
    pub fn remaining_budget(&self) -> AgentBudgets {
        AgentBudgets {
            actions: self.budget.actions.saturating_sub(self.consumed.actions),
            wall_seconds: self
                .budget
                .wall_seconds
                .saturating_sub(self.consumed.wall_seconds),
        }
    }

    /// Whether this member may act at `now`, given fresh read-back.
    /// Suspended/expelled/unknown members never pass, whatever the lease says.
    pub fn can_act_at(&self, now_unix_ms: u64) -> bool {
        self.status.is_active() && self.lease.is_live_at(now_unix_ms)
    }
}

/// Declared desired membership. Authority input of the reconcile step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederationDesiredState {
    pub schema: String,
    /// Id of the hub authority owning this desired state.
    pub hub_authority_id: String,
    pub members: Vec<DesiredMember>,
}

/// A member as intended by the operator/hub, comparable to observed records
/// without touching the substrate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesiredMember {
    pub node_id: String,
    pub roles: Vec<FleetRole>,
    pub identity_fingerprint: String,
    pub endpoint: Option<String>,
    pub budget: AgentBudgets,
}

/// Observed membership snapshot fed back from the durable store / probes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederationObservation {
    pub schema: String,
    pub observed_at_unix_ms: u64,
    pub members: Vec<FederationMember>,
}

/// Pure reconcile output: one bounded action per divergence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum MembershipAction {
    Admit { node_id: String },
    Renew { node_id: String },
    Update { node_id: String },
    Suspend { node_id: String, reason: String },
    Expel { node_id: String, reason: String },
    Keep { node_id: String },
}

/// Reconcile desired membership against observed membership.
///
/// Rules (pure, ordered):
/// - desired but unobserved → `Admit`
/// - desired and observed active → renew if lease near stale or budgets
///   changed (`Renew` / `Update`), else `Keep`
/// - desired but observed suspended/expelled → `Keep` the stricter observed
///   status; reactivation is an explicit operator action, not auto-healing
/// - observed but not desired → `Expel` (membership revoked by desired state)
/// - observed unknown freshness past lease expiry → `Suspend`
pub fn reconcile_federation_membership(
    desired: &FederationDesiredState,
    observed: &FederationObservation,
) -> Vec<MembershipAction> {
    let mut actions = Vec::new();
    let now = observed.observed_at_unix_ms;

    for want in &desired.members {
        match observed
            .members
            .iter()
            .find(|m| &m.node_id == &want.node_id)
        {
            None => actions.push(MembershipAction::Admit {
                node_id: want.node_id.clone(),
            }),
            Some(have) => match &have.status {
                MemberStatus::Active => {
                    if have.roles != want.roles
                        || have.identity_fingerprint != want.identity_fingerprint
                        || have.budget != want.budget
                    {
                        actions.push(MembershipAction::Update {
                            node_id: want.node_id.clone(),
                        });
                    } else if have.lease.is_heartbeat_stale_at(now) {
                        actions.push(MembershipAction::Renew {
                            node_id: want.node_id.clone(),
                        });
                    } else {
                        actions.push(MembershipAction::Keep {
                            node_id: want.node_id.clone(),
                        });
                    }
                }
                // Refutable grace band: keep the lease alive so the member
                // can still refute; escalation to Lapsed is observation work,
                // never an authority action.
                MemberStatus::Suspect { .. } => {
                    actions.push(MembershipAction::Renew {
                        node_id: want.node_id.clone(),
                    });
                }
                // Coordination trust withdrawn; identity/grant untouched.
                // Only explicit authority input reactivates or expels.
                MemberStatus::Lapsed { .. } => {
                    actions.push(MembershipAction::Keep {
                        node_id: want.node_id.clone(),
                    });
                }
                MemberStatus::Suspended { .. } | MemberStatus::Expelled { .. } => {
                    actions.push(MembershipAction::Keep {
                        node_id: want.node_id.clone(),
                    });
                }
                MemberStatus::Unknown { reason } => {
                    if !have.lease.is_live_at(now) {
                        actions.push(MembershipAction::Suspend {
                            node_id: want.node_id.clone(),
                            reason: format!("unknown_freshness_past_lease: {}", reason),
                        });
                    } else {
                        actions.push(MembershipAction::Renew {
                            node_id: want.node_id.clone(),
                        });
                    }
                }
            },
        }
    }

    for have in &observed.members {
        if !desired.members.iter().any(|w| w.node_id == have.node_id) {
            actions.push(MembershipAction::Expel {
                node_id: have.node_id.clone(),
                reason: "not_in_desired_state".to_string(),
            });
        }
    }

    actions
}

// ── Liveness transitions ─────────────────────────────────────────────────

/// Pure liveness transition rule (L3, D4 node-local).
///
/// Laws:
/// - a fresh acknowledgment refutes `Suspect`/`Lapsed`/`Unknown` back to
///   `Active`; authority states (`Suspended`/`Expelled`) are never cleared
///   by liveness evidence;
/// - silence escalates strictly through the injectable thresholds —
///   suspect band first (refutable), then lapsed;
/// - **no timer alone ever produces an authority action**: reaching `Lapsed`
///   is a coordination-trust state; `Suspend`/`Expel` remain explicit
///   decisions of the reconcile step fed by desired state or operator input.
pub fn membership_liveness_transition(
    current: &MemberStatus,
    observation: &LivenessObservation,
    policy: &MembershipPolicy,
) -> MemberStatus {
    if !policy.validate().is_empty() {
        // An invalid policy must never silently degrade the fleet view;
        // the caller surfaces validation blockers. Liveness is frozen.
        return current.clone();
    }
    match observation {
        LivenessObservation::Acknowledged => {
            if current.is_refutable_by_acknowledgment() {
                MemberStatus::Active
            } else {
                current.clone()
            }
        }
        LivenessObservation::SilentFor { elapsed_ms } => {
            let suspect_at = policy.suspect_after_ms;
            let lapsed_at = policy.lapsed_after_ms;
            let next = match current {
                MemberStatus::Suspended { .. }
                | MemberStatus::Expelled { .. }
                | MemberStatus::Active
                | MemberStatus::Unknown { .. }
                | MemberStatus::Suspect { .. }
                | MemberStatus::Lapsed { .. } => {
                    if *elapsed_ms >= lapsed_at {
                        MemberStatus::Lapsed {
                            reason: format!("silence_past_lapsed_threshold_{}ms", lapsed_at),
                        }
                    } else if *elapsed_ms >= suspect_at {
                        MemberStatus::Suspect {
                            reason: format!("silence_past_suspect_threshold_{}ms", suspect_at),
                        }
                    } else {
                        current.clone()
                    }
                }
            };
            // Authority stickiness: timers never clear or add authority state.
            match (current, &next) {
                (MemberStatus::Suspended { .. }, _) | (MemberStatus::Expelled { .. }, _) => {
                    current.clone()
                }
                _ => next,
            }
        }
    }
}

// ── Predicates ───────────────────────────────────────────────────────────

/// Monotonic delegation: a member's granted budget must fit within the hub's
/// own remaining budget. Budgets flow downward, never amplify.
pub fn budget_within_hub(child_budget: &AgentBudgets, hub_remaining: &AgentBudgets) -> bool {
    child_budget.actions <= hub_remaining.actions
        && child_budget.wall_seconds <= hub_remaining.wall_seconds
}

/// Role compatibility: exactly one hub may exist per federation. Any desired
/// member carrying `KernelPrimary` beyond the hub authority id is invalid.
pub fn roles_admissible(desired: &FederationDesiredState, member: &DesiredMember) -> bool {
    let claims_hub = member.roles.contains(&FleetRole::KernelPrimary);
    if claims_hub && member.node_id != desired.hub_authority_id {
        return false;
    }
    !member.roles.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budgets(actions: u64, wall_seconds: u64) -> AgentBudgets {
        AgentBudgets {
            actions,
            wall_seconds,
        }
    }

    fn lease(expires_in_ms: u64) -> MemberLease {
        MemberLease {
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 1000 + expires_in_ms,
            fencing_token: 1,
        }
    }

    fn desired_member(node_id: &str, roles: Vec<FleetRole>) -> DesiredMember {
        DesiredMember {
            node_id: node_id.to_string(),
            roles,
            identity_fingerprint: format!("sha256:{}", "0".repeat(64)),
            endpoint: None,
            budget: budgets(10, 600),
        }
    }

    fn desired(hub: &str, members: Vec<DesiredMember>) -> FederationDesiredState {
        FederationDesiredState {
            schema: FEDERATION_DESIRED_SCHEMA.to_string(),
            hub_authority_id: hub.to_string(),
            members,
        }
    }

    fn observed_member(node_id: &str, status: MemberStatus, l: MemberLease) -> FederationMember {
        FederationMember {
            schema: FEDERATION_MEMBER_SCHEMA.to_string(),
            node_id: node_id.to_string(),
            roles: vec![FleetRole::BuildRunner],
            identity_fingerprint: format!("sha256:{}", "0".repeat(64)),
            endpoint: None,
            status,
            lease: l,
            admitted_by: "kernel-alpha".to_string(),
            budget: budgets(10, 600),
            consumed: AgentBudgets::default(),
        }
    }

    #[test]
    fn admit_desired_but_unobserved() {
        let d = desired(
            "kernel-alpha",
            vec![desired_member("node-a", vec![FleetRole::BuildRunner])],
        );
        let o = FederationObservation {
            schema: FEDERATION_MEMBERSHIP_SCHEMA.to_string(),
            observed_at_unix_ms: 2000,
            members: vec![],
        };
        assert_eq!(
            reconcile_federation_membership(&d, &o),
            vec![MembershipAction::Admit {
                node_id: "node-a".to_string()
            }]
        );
    }

    #[test]
    fn keep_converged_active_member() {
        let d = desired(
            "kernel-alpha",
            vec![desired_member("node-a", vec![FleetRole::BuildRunner])],
        );
        let o = FederationObservation {
            schema: FEDERATION_MEMBERSHIP_SCHEMA.to_string(),
            observed_at_unix_ms: 2000,
            members: vec![observed_member("node-a", MemberStatus::Active, lease(5000))],
        };
        assert_eq!(
            reconcile_federation_membership(&d, &o),
            vec![MembershipAction::Keep {
                node_id: "node-a".to_string()
            }]
        );
    }

    #[test]
    fn update_when_roles_or_budget_diverge() {
        let mut d = desired(
            "kernel-alpha",
            vec![desired_member("node-a", vec![FleetRole::BuildRunner])],
        );
        d.members[0].budget = budgets(20, 600);
        let o = FederationObservation {
            schema: FEDERATION_MEMBERSHIP_SCHEMA.to_string(),
            observed_at_unix_ms: 2000,
            members: vec![observed_member("node-a", MemberStatus::Active, lease(5000))],
        };
        assert_eq!(
            reconcile_federation_membership(&d, &o),
            vec![MembershipAction::Update {
                node_id: "node-a".to_string()
            }]
        );
    }

    #[test]
    fn expel_observed_but_not_desired() {
        let d = desired("kernel-alpha", vec![]);
        let o = FederationObservation {
            schema: FEDERATION_MEMBERSHIP_SCHEMA.to_string(),
            observed_at_unix_ms: 2000,
            members: vec![observed_member("ghost", MemberStatus::Active, lease(5000))],
        };
        assert_eq!(
            reconcile_federation_membership(&d, &o),
            vec![MembershipAction::Expel {
                node_id: "ghost".to_string(),
                reason: "not_in_desired_state".to_string(),
            }]
        );
    }

    #[test]
    fn suspended_member_is_not_auto_healed() {
        let d = desired(
            "kernel-alpha",
            vec![desired_member("node-a", vec![FleetRole::BuildRunner])],
        );
        let o = FederationObservation {
            schema: FEDERATION_MEMBERSHIP_SCHEMA.to_string(),
            observed_at_unix_ms: 2000,
            members: vec![observed_member(
                "node-a",
                MemberStatus::Suspended {
                    reason: "operator_hold".to_string(),
                },
                lease(5000),
            )],
        };
        assert_eq!(
            reconcile_federation_membership(&d, &o),
            vec![MembershipAction::Keep {
                node_id: "node-a".to_string()
            }]
        );
    }

    #[test]
    fn unknown_freshness_past_lease_suspends() {
        let d = desired(
            "kernel-alpha",
            vec![desired_member("node-a", vec![FleetRole::BuildRunner])],
        );
        let o = FederationObservation {
            schema: FEDERATION_MEMBERSHIP_SCHEMA.to_string(),
            observed_at_unix_ms: 9000,
            members: vec![observed_member(
                "node-a",
                MemberStatus::Unknown {
                    reason: "no_readback".to_string(),
                },
                lease(5000),
            )],
        };
        assert!(matches!(
            reconcile_federation_membership(&d, &o).as_slice(),
            [MembershipAction::Suspend { .. }]
        ));
    }

    #[test]
    fn member_cannot_act_when_suspended_even_with_live_lease() {
        let m = observed_member(
            "node-a",
            MemberStatus::Suspended {
                reason: "hold".to_string(),
            },
            lease(5000),
        );
        assert!(!m.can_act_at(2000));
        assert!(observed_member("node-b", MemberStatus::Active, lease(5000)).can_act_at(2000));
        assert!(!observed_member("node-c", MemberStatus::Active, lease(5000)).can_act_at(9999));
    }

    #[test]
    fn budget_delegation_is_monotonic() {
        let hub_remaining = budgets(10, 600);
        assert!(budget_within_hub(&budgets(10, 600), &hub_remaining));
        assert!(!budget_within_hub(&budgets(11, 600), &hub_remaining));
        assert!(!budget_within_hub(&budgets(10, 601), &hub_remaining));
    }

    #[test]
    fn second_kernel_primary_role_is_inadmissible() {
        let d = desired("kernel-alpha", vec![]);
        assert!(!roles_admissible(
            &d,
            &desired_member("impostor", vec![FleetRole::KernelPrimary])
        ));
        assert!(roles_admissible(
            &d,
            &desired_member("kernel-alpha", vec![FleetRole::KernelPrimary])
        ));
        assert!(!roles_admissible(&d, &desired_member("node-x", vec![])));
    }

    // ── L3 liveness law (D3/D4, step 1 pure layer) ──────────────────────

    fn policy() -> MembershipPolicy {
        MembershipPolicy::default()
    }

    #[test]
    fn membership_policy_defaults_validate_clean() {
        assert_eq!(policy().validate(), Vec::<String>::new());
    }

    #[test]
    fn membership_policy_ordering_law_is_enforced() {
        let mut p = policy();
        p.probe_period_ms = p.suspect_after_ms;
        assert!(
            p.validate()
                .contains(&"membership_policy_probe_not_finer_than_suspect".to_string())
        );
        let mut q = policy();
        q.suspect_after_ms = q.lapsed_after_ms;
        assert!(
            q.validate()
                .contains(&"membership_policy_suspect_not_below_lapsed".to_string())
        );
    }

    #[test]
    fn liveness_transition_table_driven_law() {
        let cases: Vec<(MemberStatus, LivenessObservation, MemberStatus)> = vec![
            (
                MemberStatus::Active,
                LivenessObservation::Acknowledged,
                MemberStatus::Active,
            ),
            // below suspect threshold: unchanged
            (
                MemberStatus::Active,
                LivenessObservation::SilentFor { elapsed_ms: 39_999 },
                MemberStatus::Active,
            ),
            // at suspect threshold: refutable grace band entered
            (
                MemberStatus::Active,
                LivenessObservation::SilentFor { elapsed_ms: 40_000 },
                MemberStatus::Suspect {
                    reason: "silence_past_suspect_threshold_40000ms".to_string(),
                },
            ),
            // within suspect band: stays suspect
            (
                MemberStatus::Suspect {
                    reason: "x".to_string(),
                },
                LivenessObservation::SilentFor {
                    elapsed_ms: 179_999,
                },
                MemberStatus::Suspect {
                    reason: "silence_past_suspect_threshold_40000ms".to_string(),
                },
            ),
            // at lapsed threshold: coordination trust withdrawn
            (
                MemberStatus::Suspect {
                    reason: "x".to_string(),
                },
                LivenessObservation::SilentFor {
                    elapsed_ms: 180_000,
                },
                MemberStatus::Lapsed {
                    reason: "silence_past_lapsed_threshold_180000ms".to_string(),
                },
            ),
            // refutation: ack clears lapsed back to active
            (
                MemberStatus::Lapsed {
                    reason: "x".to_string(),
                },
                LivenessObservation::Acknowledged,
                MemberStatus::Active,
            ),
            // unknown is also refutable by a fresh ack
            (
                MemberStatus::Unknown {
                    reason: "no_readback".to_string(),
                },
                LivenessObservation::Acknowledged,
                MemberStatus::Active,
            ),
            // authority stickiness: ack never clears an operator hold
            (
                MemberStatus::Suspended {
                    reason: "operator_hold".to_string(),
                },
                LivenessObservation::Acknowledged,
                MemberStatus::Suspended {
                    reason: "operator_hold".to_string(),
                },
            ),
            // authority stickiness: even endless silence never expels by timer
            (
                MemberStatus::Expelled {
                    reason: "not_in_desired_state".to_string(),
                },
                LivenessObservation::SilentFor {
                    elapsed_ms: 10_000_000,
                },
                MemberStatus::Expelled {
                    reason: "not_in_desired_state".to_string(),
                },
            ),
        ];
        for (idx, (current, observation, expected)) in cases.into_iter().enumerate() {
            let got = membership_liveness_transition(&current, &observation, &policy());
            assert_eq!(got, expected, "case {idx}: {current:?} + {observation:?}");
        }
    }

    #[test]
    fn invalid_policy_freezes_liveness_instead_of_guessing() {
        let mut p = policy();
        p.schema = "bogus".to_string();
        let got = membership_liveness_transition(
            &MemberStatus::Active,
            &LivenessObservation::SilentFor {
                elapsed_ms: 999_999,
            },
            &p,
        );
        assert_eq!(
            got,
            MemberStatus::Active,
            "invalid policy must freeze, not fabricate"
        );
    }

    #[test]
    fn reconcile_suspect_renews_and_lapsed_keeps_never_timer_expels() {
        let d = desired(
            "kernel-alpha",
            vec![
                desired_member("node-a", vec![FleetRole::BuildRunner]),
                desired_member("node-b", vec![FleetRole::StoragePrimary]),
            ],
        );
        let o = FederationObservation {
            schema: FEDERATION_MEMBERSHIP_SCHEMA.to_string(),
            observed_at_unix_ms: 2000,
            members: vec![
                observed_member(
                    "node-a",
                    MemberStatus::Suspect {
                        reason: "silence".to_string(),
                    },
                    lease(5000),
                ),
                observed_member(
                    "node-b",
                    MemberStatus::Lapsed {
                        reason: "long_silence".to_string(),
                    },
                    lease(5000),
                ),
            ],
        };
        let actions = reconcile_federation_membership(&d, &o);
        assert_eq!(
            actions,
            vec![
                MembershipAction::Renew {
                    node_id: "node-a".to_string(),
                },
                MembershipAction::Keep {
                    node_id: "node-b".to_string(),
                },
            ]
        );
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, MembershipAction::Expel { .. })),
            "no timer-only expulsion"
        );
    }
}
