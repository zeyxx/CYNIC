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

use crate::federation_membership::FleetRole;

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
pub fn role_capabilities(role: FleetRole) -> &'static [Capability] {
    match role {
        // The hub coordinates but does not absorb substrate roles.
        FleetRole::KernelPrimary => &[
            Capability::FleetObserve,
            Capability::MembershipAdmit,
            Capability::NodeControl,
        ],
        FleetRole::Hypervisor => &[Capability::FleetObserve, Capability::HostSupervision],
        FleetRole::CortexHost => &[
            Capability::FleetObserve,
            Capability::HostSupervision,
            Capability::InferenceServe,
        ],
        FleetRole::StoragePrimary => &[Capability::FleetObserve, Capability::StorageAcl],
        FleetRole::BuildRunner => &[Capability::FleetObserve, Capability::BuildExecute],
        FleetRole::ForgePrimary => &[Capability::FleetObserve, Capability::ForgeAdmin],
    }
}

/// Principal whose authorization is requested. Roles are the typed keys of
/// the membership contract; a principal with no modelled role is `Unknown`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthzPrincipal<'a> {
    pub node_id: &'a str,
    pub roles: &'a [FleetRole],
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
/// - any held role granting the capability → `Allow`
/// - roles present but none grants it → `Deny` (explicitly out of scope)
/// - no modelled role at all → `Unknown` (principal outside the contract)
pub fn authorize(principal: &AuthzPrincipal, required: Capability) -> AuthzDecision {
    if principal.roles.is_empty() {
        return AuthzDecision::Unknown {
            reason: format!("principal {} holds no modelled role", principal.node_id),
        };
    }
    if principal
        .roles
        .iter()
        .any(|r| role_capabilities(*r).contains(&required))
    {
        return AuthzDecision::Allow;
    }
    AuthzDecision::Deny {
        reason: format!(
            "roles {:?} do not grant capability {:?}",
            principal.roles, required
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

    fn principal<'a>(node_id: &'a str, roles: &'a [FleetRole]) -> AuthzPrincipal<'a> {
        AuthzPrincipal { node_id, roles }
    }

    #[test]
    fn hub_admits_and_controls_but_does_not_absorb_substrate_roles() {
        let hub = principal("kernel-alpha", &[FleetRole::KernelPrimary]);
        assert!(authorize(&hub, Capability::MembershipAdmit).is_allowed());
        assert!(authorize(&hub, Capability::NodeControl).is_allowed());
        assert!(!authorize(&hub, Capability::BuildExecute).is_allowed());
        assert!(!authorize(&hub, Capability::StorageAcl).is_allowed());
    }

    #[test]
    fn build_runner_executes_but_cannot_admin_forge() {
        let runner = principal("builder-1", &[FleetRole::BuildRunner]);
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
        for role in crate::federation_membership::ALL_FLEET_ROLES {
            let roles = [role];
            let p = principal("any", &roles);
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
