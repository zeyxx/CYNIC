//! Coordination-loss continuity law for external agent sessions.
//!
//! A missing, incompatible, or unstable coordination runtime must notify the
//! operator without turning an agent into a powerless relay. Read-only
//! capabilities remain available; mutating capabilities stay suspended until
//! the existing bounded human-scoped fallback is independently admitted.

use serde::{Deserialize, Serialize};

use crate::{AgentCapabilityGrant, CoordinationRuntimeAvailability};

pub const AGENT_COORDINATION_CONTINUITY_SCHEMA: &str = "cynic.agent_coordination_continuity.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCoordinationContinuityStatus {
    RuntimeFenced,
    DegradedOperatorNotification,
    BlockedUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCoordinationContinuity {
    pub schema: String,
    pub status: AgentCoordinationContinuityStatus,
    pub runtime_availability: CoordinationRuntimeAvailability,
    pub operator_notification_required: bool,
    pub retained_capabilities: Vec<AgentCapabilityGrant>,
    pub suspended_capabilities: Vec<AgentCapabilityGrant>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

/// Reconcile coordination availability without amplifying authority.
///
/// This function never admits a mutation. A degraded agent keeps only its
/// non-mutating capabilities, while the bounded source fallback remains owned
/// by `decide_coordination_mode` in `coordination_lease`.
pub fn reconcile_agent_coordination_continuity(
    runtime_availability: CoordinationRuntimeAvailability,
    capabilities: &[AgentCapabilityGrant],
) -> AgentCoordinationContinuity {
    let mut retained_capabilities = capabilities
        .iter()
        .filter(|capability| !capability.mutates)
        .cloned()
        .collect::<Vec<_>>();
    let mut suspended_capabilities = capabilities
        .iter()
        .filter(|capability| capability.mutates)
        .cloned()
        .collect::<Vec<_>>();
    sort_capabilities(&mut retained_capabilities);
    sort_capabilities(&mut suspended_capabilities);

    let (status, operator_notification_required, blockers, unknowns) = match runtime_availability {
        CoordinationRuntimeAvailability::ActiveStable => {
            retained_capabilities.extend(suspended_capabilities);
            sort_capabilities(&mut retained_capabilities);
            suspended_capabilities = Vec::new();
            (
                AgentCoordinationContinuityStatus::RuntimeFenced,
                false,
                Vec::new(),
                Vec::new(),
            )
        }
        CoordinationRuntimeAvailability::Unavailable
        | CoordinationRuntimeAvailability::Incompatible
        | CoordinationRuntimeAvailability::Unstable => (
            AgentCoordinationContinuityStatus::DegradedOperatorNotification,
            true,
            vec![
                format!("coordination_runtime_{runtime_availability:?}_mutation_suspended")
                    .to_lowercase(),
            ],
            Vec::new(),
        ),
        CoordinationRuntimeAvailability::Unknown => (
            AgentCoordinationContinuityStatus::BlockedUnknown,
            true,
            vec!["coordination_runtime_unknown_mutation_suspended".to_string()],
            vec!["coordination_runtime_availability_unproven".to_string()],
        ),
    };

    AgentCoordinationContinuity {
        schema: AGENT_COORDINATION_CONTINUITY_SCHEMA.to_string(),
        status,
        runtime_availability,
        operator_notification_required,
        retained_capabilities,
        suspended_capabilities,
        blockers,
        unknowns,
    }
}

fn sort_capabilities(capabilities: &mut [AgentCapabilityGrant]) {
    capabilities.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then(left.mutates.cmp(&right.mutates))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities() -> Vec<AgentCapabilityGrant> {
        vec![
            AgentCapabilityGrant {
                id: "source.apply".to_string(),
                mutates: true,
            },
            AgentCapabilityGrant {
                id: "runtime.observe".to_string(),
                mutates: false,
            },
        ]
    }

    #[test]
    fn unavailable_coordination_notifies_operator_but_preserves_read_only_capability() {
        let report = reconcile_agent_coordination_continuity(
            CoordinationRuntimeAvailability::Incompatible,
            &capabilities(),
        );

        assert_eq!(
            report.status,
            AgentCoordinationContinuityStatus::DegradedOperatorNotification
        );
        assert!(report.operator_notification_required);
        assert_eq!(report.retained_capabilities.len(), 1);
        assert_eq!(report.retained_capabilities[0].id, "runtime.observe");
        assert_eq!(report.suspended_capabilities.len(), 1);
        assert_eq!(report.suspended_capabilities[0].id, "source.apply");
    }

    #[test]
    fn stable_coordination_keeps_the_original_capability_set_fenced() {
        let report = reconcile_agent_coordination_continuity(
            CoordinationRuntimeAvailability::ActiveStable,
            &capabilities(),
        );

        assert_eq!(
            report.status,
            AgentCoordinationContinuityStatus::RuntimeFenced
        );
        assert!(!report.operator_notification_required);
        assert_eq!(
            report
                .retained_capabilities
                .iter()
                .map(|capability| capability.id.as_str())
                .collect::<Vec<_>>(),
            vec!["runtime.observe", "source.apply"]
        );
        assert!(report.suspended_capabilities.is_empty());
    }

    #[test]
    fn unknown_coordination_never_restores_mutation_authority() {
        let report = reconcile_agent_coordination_continuity(
            CoordinationRuntimeAvailability::Unknown,
            &capabilities(),
        );

        assert_eq!(
            report.status,
            AgentCoordinationContinuityStatus::BlockedUnknown
        );
        assert!(report.operator_notification_required);
        assert_eq!(report.retained_capabilities[0].id, "runtime.observe");
        assert_eq!(report.suspended_capabilities[0].id, "source.apply");
        assert_eq!(
            report.unknowns,
            vec!["coordination_runtime_availability_unproven".to_string()]
        );
    }
}
