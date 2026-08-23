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

/// Membership lifecycle status. `Unknown` is data: it means the hub has no
/// fresh read-back for the member, not that the member is healthy or dead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MemberStatus {
    Active,
    Suspended { reason: String },
    Expelled { reason: String },
    Unknown { reason: String },
}

impl MemberStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, MemberStatus::Active)
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
}
