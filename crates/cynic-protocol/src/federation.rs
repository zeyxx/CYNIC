//! Federation reconciliation — composes the four autonomy models into one pipeline.
//!
//! CYNIC runs multiple autonomy models concurrently. Without reconciliation, they
//! contradict each other (node_control says "converged" while local_autonomy says
//! "degraded"). This module is the single composition point: it decides which
//! model owns the decision at each moment, following the fractal coherence
//! pattern — the same observe→reconcile→act shape at every scale, with a
//! different decision owner depending on the node's operating mode.
//!
//! Composition pipeline:
//!
//! ```text
//! NodeLocalObservation (OS, kernel, mesh, enrollment)
//!     ↓
//! decide_autonomy() → AutonomyMode
//!     ↓
//! match mode {
//!     Coordinated  → NodeControlPlan (kernel observes, directs)
//!     Autonomous   → elect_for_role() (self-coordinate via emergence)
//!     Degraded     → Retain identity + health only
//! }
//!     ↓ (if elected for a promoted role)
//! decide_runtime_takeover() → promote or stay standby
//!     ↓
//! FederationReconciliation → the authoritative decision for this node
//! ```
//!
//! This is structural-invariants rule #6: one canonical decision per concern.
//! Vertical-specific enums (NodeControlDecision, EmergenceDecision, etc.)
//! still exist for their own domain, but the authoritative composition lives here.

use serde::Serialize;

use crate::emergence::EmergenceDecision;
use crate::local_autonomy::{AutonomyDecision, AutonomyMode};
use crate::node_control::NodeControlPlan;
use crate::runtime_takeover::RuntimeTakeoverDecision;

/// The authoritative reconciliation output for a federation node.
///
/// This is the single source of truth for "what should this node do now".
/// Domain-specific decisions (NodeControlPlan, EmergenceDecision, etc.)
/// are preserved as evidence fields — they are inputs to this decision,
/// not competing authorities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FederationReconciliation {
    /// Kernel is alive and directing. Node follows the kernel's plan.
    /// The kernel's NodeControlPlan is the authority.
    Coordinated {
        node_control_plan: NodeControlPlan,
        autonomy: AutonomyDecision,
    },

    /// Kernel is unreachable. Node self-coordinates via emergence.
    /// The EmergenceDecision is the authority for role assignment.
    Autonomous {
        assumed_roles: Vec<EmergenceDecision>,
        autonomy: AutonomyDecision,
    },

    /// Node cannot observe enough to decide. Degraded to minimum capabilities.
    Degraded {
        autonomy: AutonomyDecision,
        retained_capabilities: Vec<String>,
    },

    /// Node has been elected for a promoted role (e.g. standby → active kernel).
    /// Runtime takeover decision is the authority.
    Promoted {
        takeover: RuntimeTakeoverDecision,
        autonomy: AutonomyDecision,
    },
}

impl FederationReconciliation {
    /// The operating mode this reconciliation represents.
    pub fn mode(&self) -> AutonomyMode {
        match self {
            FederationReconciliation::Coordinated { .. } => AutonomyMode::Coordinated,
            FederationReconciliation::Autonomous { .. } => AutonomyMode::Autonomous,
            FederationReconciliation::Degraded { .. } => AutonomyMode::Degraded,
            FederationReconciliation::Promoted { .. } => AutonomyMode::Autonomous,
        }
    }

    /// Human-readable label for the reconciliation status.
    pub fn label(&self) -> String {
        match self {
            FederationReconciliation::Coordinated { .. } => "coordinated",
            FederationReconciliation::Autonomous { .. } => "autonomous",
            FederationReconciliation::Degraded { .. } => "degraded",
            FederationReconciliation::Promoted { .. } => "promoted",
        }
        .to_string()
    }

    /// What capabilities this node retains in this mode.
    pub fn retained_capabilities(&self) -> Vec<String> {
        match self {
            FederationReconciliation::Coordinated { autonomy, .. } => {
                let caps = &autonomy.retained_capabilities;
                let mut v = Vec::new();
                if caps.identity { v.push("identity"); }
                if caps.health { v.push("health"); }
                if caps.observe { v.push("observe"); }
                if caps.judge { v.push("judge"); }
                if caps.build { v.push("build"); }
                if caps.release { v.push("release"); }
                v.into_iter().map(String::from).collect()
            }
            FederationReconciliation::Autonomous { autonomy, .. } => {
                let caps = &autonomy.retained_capabilities;
                let mut v = Vec::new();
                if caps.identity { v.push("identity"); }
                if caps.health { v.push("health"); }
                if caps.observe { v.push("observe"); }
                if caps.judge { v.push("judge"); }
                v.into_iter().map(String::from).collect()
            }
            FederationReconciliation::Degraded { retained_capabilities, .. } => {
                retained_capabilities.clone()
            }
            FederationReconciliation::Promoted { takeover, .. } => {
                takeover.retained_capabilities.iter().map(|s| s.to_string()).collect()
            }
        }
    }

    /// Residual unknowns — what remains unobserved.
    pub fn residual_unknowns(&self) -> Vec<String> {
        match self {
            FederationReconciliation::Coordinated { autonomy, .. } => autonomy.blockers.clone(),
            FederationReconciliation::Autonomous { autonomy, .. } => autonomy.blockers.clone(),
            FederationReconciliation::Degraded { autonomy, .. } => autonomy.blockers.clone(),
            FederationReconciliation::Promoted { takeover, .. } => {
                takeover.blockers.iter().map(|s| s.to_string()).collect()
            }
        }
    }
}

/// Compose the four autonomy models into one authoritative decision.
///
/// This is the single entry point for "what should this node do now".
/// Each model is used at the appropriate moment:
///
/// 1. `decide_autonomy()` → which mode are we in?
/// 2. If Coordinated → use the kernel's NodeControlPlan
/// 3. If Autonomous → use emergence to self-coordinate
/// 4. If Autonomous + elected for a promoted role → use runtime_takeover
/// 5. If Degraded → retain minimum capabilities
pub fn reconcile_federation(
    autonomy: &AutonomyDecision,
    node_control_plan: Option<&NodeControlPlan>,
    emergence_decisions: &[EmergenceDecision],
    takeover: Option<&RuntimeTakeoverDecision>,
) -> FederationReconciliation {
    match autonomy.mode {
        AutonomyMode::Coordinated => {
            if let Some(plan) = node_control_plan {
                FederationReconciliation::Coordinated {
                    node_control_plan: plan.clone(),
                    autonomy: autonomy.clone(),
                }
            } else {
                // Should not happen: Coordinated mode requires a kernel plan.
                // Fall back to Autonomous (kernel plan missing = kernel unreachable).
                FederationReconciliation::Autonomous {
                    assumed_roles: emergence_decisions.to_vec(),
                    autonomy: autonomy.clone(),
                }
            }
        }
        AutonomyMode::Autonomous => {
            if let Some(takeover_decision) = takeover {
                if takeover_decision.promote {
                    FederationReconciliation::Promoted {
                        takeover: takeover_decision.clone(),
                        autonomy: autonomy.clone(),
                    }
                } else {
                    FederationReconciliation::Autonomous {
                        assumed_roles: emergence_decisions.to_vec(),
                        autonomy: autonomy.clone(),
                    }
                }
            } else {
                FederationReconciliation::Autonomous {
                    assumed_roles: emergence_decisions.to_vec(),
                    autonomy: autonomy.clone(),
                }
            }
        }
        AutonomyMode::Degraded => {
            let retained = vec!["identity".to_string(), "health".to_string()];
            FederationReconciliation::Degraded {
                autonomy: autonomy.clone(),
                retained_capabilities: retained,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_autonomy::RetainedCapabilities;
    use crate::runtime_takeover::StandbyMode;

    fn autonomy_decision(mode: AutonomyMode) -> AutonomyDecision {
        AutonomyDecision {
            schema: "cynic.local_autonomy_decision.v1".to_string(),
            mode,
            retained_capabilities: RetainedCapabilities::for_mode(mode),
            blockers: vec![],
            evidence_refs: vec![],
        }
    }

    fn dummy_node_control_plan() -> NodeControlPlan {
        NodeControlPlan {
            schema: "cynic.node_control_plan.v2".to_string(),
            status: "converged".to_string(),
            node_id: "node-a".to_string(),
            role: "build_runner".to_string(),
            desired_generation: 1,
            observed_generation: Some(1),
            connection: crate::node_control::NodeControlConnectionState::Connected,
            decision: crate::node_control::NodeControlDecision::Converged,
            execution: None,
            evidence_refs: vec![],
            unknowns: vec![],
            epoch: None,
        }
    }

    fn dummy_emergence_decision() -> EmergenceDecision {
        EmergenceDecision {
            schema: "cynic.emergence_decision.v1".to_string(),
            local_node_id: "node-a".to_string(),
            assumed_role: Some("build_runner".to_string()),
            role_priority: Some(crate::emergence::RolePriority::Primary),
            candidates: vec!["node-a".to_string()],
            reasoning: "elected node-a for build_runner".to_string(),
            residual_unknowns: vec![],
        }
    }

    fn dummy_takeover_decision(promote: bool) -> RuntimeTakeoverDecision {
        RuntimeTakeoverDecision {
            schema: "cynic.runtime_takeover.v1",
            status: if promote { "promotable" } else { "blocked" },
            promote,
            mode: if promote { StandbyMode::Active } else { StandbyMode::Resident },
            fencing_token: 2,
            retained_capabilities: if promote {
                vec!["identity", "health", "observe"]
            } else {
                vec!["identity", "health"]
            },
            blocked_mutations: vec!["fleet_mutation"],
            blockers: if promote { vec![] } else { vec!["primary_still_live"] },
        }
    }

    #[test]
    fn coordinated_when_kernel_alive() {
        let autonomy = autonomy_decision(AutonomyMode::Coordinated);
        let plan = dummy_node_control_plan();
        let reconciliation = reconcile_federation(&autonomy, Some(&plan), &[], None);

        assert_eq!(reconciliation.mode(), AutonomyMode::Coordinated);
        assert_eq!(reconciliation.label(), "coordinated");
        assert!(reconciliation.residual_unknowns().is_empty());
    }

    #[test]
    fn autonomous_when_kernel_dead() {
        let autonomy = autonomy_decision(AutonomyMode::Autonomous);
        let emergence = vec![dummy_emergence_decision()];
        let reconciliation = reconcile_federation(&autonomy, None, &emergence, None);

        assert_eq!(reconciliation.mode(), AutonomyMode::Autonomous);
        assert_eq!(reconciliation.label(), "autonomous");
    }

    #[test]
    fn promoted_when_elected_and_takeover_approves() {
        let autonomy = autonomy_decision(AutonomyMode::Autonomous);
        let emergence = vec![dummy_emergence_decision()];
        let takeover = dummy_takeover_decision(true);
        let reconciliation = reconcile_federation(&autonomy, None, &emergence, Some(&takeover));

        assert_eq!(reconciliation.mode(), AutonomyMode::Autonomous);
        assert_eq!(reconciliation.label(), "promoted");
        assert!(reconciliation.retained_capabilities().contains(&"observe".to_string()));
    }

    #[test]
    fn not_promoted_when_takeover_blocks() {
        let autonomy = autonomy_decision(AutonomyMode::Autonomous);
        let emergence = vec![dummy_emergence_decision()];
        let takeover = dummy_takeover_decision(false);
        let reconciliation = reconcile_federation(&autonomy, None, &emergence, Some(&takeover));

        assert_eq!(reconciliation.label(), "autonomous");
        assert_eq!(reconciliation.retained_capabilities().len(), 4); // identity, health, observe, judge
    }

    #[test]
    fn degraded_when_enrollment_unproved() {
        let autonomy = autonomy_decision(AutonomyMode::Degraded);
        let reconciliation = reconcile_federation(&autonomy, None, &[], None);

        assert_eq!(reconciliation.mode(), AutonomyMode::Degraded);
        assert_eq!(reconciliation.label(), "degraded");
        assert_eq!(reconciliation.retained_capabilities(), vec!["identity", "health"]);
    }

    #[test]
    fn coordinated_without_plan_falls_back_to_autonomous() {
        let autonomy = autonomy_decision(AutonomyMode::Coordinated);
        let emergence = vec![dummy_emergence_decision()];
        let reconciliation = reconcile_federation(&autonomy, None, &emergence, None);

        // Kernel plan missing = kernel unreachable → Autonomous
        assert_eq!(reconciliation.label(), "autonomous");
    }

    #[test]
    fn residual_unknowns_propagate_from_blockers() {
        let mut autonomy = autonomy_decision(AutonomyMode::Degraded);
        autonomy.blockers = vec!["enrollment_unproved".to_string()];
        let reconciliation = reconcile_federation(&autonomy, None, &[], None);

        assert_eq!(reconciliation.residual_unknowns(), vec!["enrollment_unproved"]);
    }

    #[test]
    fn schema_is_stable() {
        let reconciliation = FederationReconciliation::Degraded {
            autonomy: autonomy_decision(AutonomyMode::Degraded),
            retained_capabilities: vec!["identity".to_string()],
        };
        let json = serde_json::to_value(&reconciliation).unwrap();
        assert_eq!(json["status"], "degraded");
    }
}
