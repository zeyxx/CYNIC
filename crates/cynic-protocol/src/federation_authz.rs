//! Canonical federation authorization — one decision source per concern.
//!
//! Per AGENTS.md invariant #6, role-to-capability policy must not be
//! recalculated independently in enrollment, admission, storage, or CLI
//! surfaces. This module owns the single table and the single pure decision
//! function; every other surface consumes it.
//!
//! Decision shape is honest about residual unknowns: `Deny` means the policy
//! explicitly forbids, `Unknown` means the capability or principal is not
//! modelled — never silently mapped to either verdict.

use serde::{Deserialize, Serialize};

use crate::federation_membership::InfraRole;

/// Capabilities the fleet can grant. Exhaustive by design: adding a
/// capability is a contract change, visible in this table first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Observe fleet topology and member health (read-only).
    FleetObserve,
    /// Admit, renew, suspend, or expel members (hub-only).
    MembershipAdmit,
    /// Issue signed node control plans.
    NodeControl,
    /// Mutate build workloads on build runners.
    BuildExecute,
    /// Own storage ACL and credential sync decisions.
    StorageAcl,
    /// Administer forge repositories and webhooks.
    ForgeAdmin,
    /// Supervise host OS substrate (services, packages, bootstrap).
    HostSupervision,
    /// Run inference backends and proxy inference traffic.
    InferenceServe,
}

/// The canonical role → capabilities table. One row per role; no other
/// surface may maintain a parallel table.
pub fn role_capabilities(role: InfraRole) -> &'static [Capability] {
    match role {
        // The hub coordinates but does not absorb substrate roles.
        InfraRole::ControlPlane => &[
            Capability::FleetObserve,
            Capability::MembershipAdmit,
            Capability::NodeControl,
        ],
        InfraRole::Hypervisor => &[Capability::FleetObserve, Capability::HostSupervision],
        InfraRole::PrimaryHost => &[
            Capability::FleetObserve,
            Capability::HostSupervision,
            Capability::InferenceServe,
        ],
        InfraRole::StorageNode => &[Capability::FleetObserve, Capability::StorageAcl],
        InfraRole::BuildNode => &[Capability::FleetObserve, Capability::BuildExecute],
        InfraRole::StoreNode => &[Capability::FleetObserve, Capability::ForgeAdmin],
        InfraRole::MeshNode => &[Capability::FleetObserve],
    }
}

/// Principal whose authorization is requested. Capabilities are the L4
/// declared capabilities of the node (see `infra/fleet-taxonomy.json`);
/// a principal with no modelled capability is `Unknown`. The role is no
/// longer the authorization key — only the declared capabilities are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthzPrincipal<'a> {
    pub node_id: &'a str,
    pub capabilities: &'a [Capability],
}

/// Explicit authorization decision. `Unknown` is data, never a silent deny.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum AuthzDecision {
    Allow,
    Deny { reason: String },
    Unknown { reason: String },
}

impl AuthzDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, AuthzDecision::Allow)
    }
}

/// Pure decision: may this principal exercise this capability?
///
/// - required capability present in principal's declared capabilities → `Allow`
/// - capabilities present but required not among them → `Deny` (explicitly out of scope)
/// - no declared capability at all → `Unknown` (principal outside the contract)
pub fn authorize(principal: &AuthzPrincipal, required: Capability) -> AuthzDecision {
    if principal.capabilities.is_empty() {
        return AuthzDecision::Unknown {
            reason: format!("principal {} holds no modelled capability", principal.node_id),
        };
    }
    if principal.capabilities.contains(&required) {
        return AuthzDecision::Allow;
    }
    AuthzDecision::Deny {
        reason: format!(
            "capabilities {:?} do not grant capability {:?}",
            principal.capabilities, required
        ),
    }
}

/// Whether an action that mutates the substrate additionally requires the
/// explicit bounded-execution gate (`--yes` / `--apply`). Authorization is
/// necessary but never sufficient for mutation.
pub fn requires_mutation_gate(decision: &AuthzDecision, mutates: bool) -> bool {
    mutates && !matches!(decision, AuthzDecision::Unknown { .. })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal<'a>(node_id: &'a str, capabilities: &'a [Capability]) -> AuthzPrincipal<'a> {
        AuthzPrincipal { node_id, capabilities }
    }

    #[test]
    fn hub_admits_and_controls_but_does_not_absorb_substrate_roles() {
        let hub = principal(
            "kernel-alpha",
            &[Capability::FleetObserve, Capability::MembershipAdmit, Capability::NodeControl],
        );
        assert!(authorize(&hub, Capability::MembershipAdmit).is_allowed());
        assert!(authorize(&hub, Capability::NodeControl).is_allowed());
        assert!(!authorize(&hub, Capability::BuildExecute).is_allowed());
        assert!(!authorize(&hub, Capability::StorageAcl).is_allowed());
    }

    #[test]
    fn build_runner_executes_but_cannot_admin_forge() {
        let runner = principal(
            "builder-1",
            &[Capability::FleetObserve, Capability::BuildExecute],
        );
        assert!(authorize(&runner, Capability::BuildExecute).is_allowed());
        assert!(matches!(
            authorize(&runner, Capability::ForgeAdmin),
            AuthzDecision::Deny { .. }
        ));
    }

    #[test]
    fn unmodelled_principal_is_unknown_not_denied() {
        let ghost = principal("ghost", &[]);
        assert!(matches!(
            authorize(&ghost, Capability::FleetObserve),
            AuthzDecision::Unknown { .. }
        ));
    }

    #[test]
    fn observe_is_common_to_all_roles() {
        for role in crate::federation_membership::ALL_INFRA_ROLES {
            let caps = role_capabilities(role);
            let p = principal("any", caps);
            assert!(authorize(&p, Capability::FleetObserve).is_allowed());
        }
    }

    #[test]
    fn mutation_gate_applies_to_allow_and_deny_but_not_unknown() {
        let allow = AuthzDecision::Allow;
        let deny = AuthzDecision::Deny {
            reason: "r".to_string(),
        };
        let unknown = AuthzDecision::Unknown {
            reason: "r".to_string(),
        };
        assert!(requires_mutation_gate(&allow, true));
        assert!(requires_mutation_gate(&deny, true));
        assert!(!requires_mutation_gate(&unknown, true));
        assert!(!requires_mutation_gate(&allow, false));
    }
}
