use cynic_protocol::{
    NODE_SUBSTRATE_OBSERVATION_SCHEMA, NodeNetworkAttachment, NodeSubstrateDecision,
    NodeSubstrateExistence, NodeSubstrateObservation, NodeSubstratePower, reconcile_node_substrate,
};
use serde::Serialize;

use crate::ports::{NodeSubstrateControlPort, NodeSubstrateObservationPort};
use crate::registry::resolve_node_substrate_desired_state;

#[derive(Debug, Clone, Serialize)]
pub struct NodeSubstrateAssessment {
    pub schema: &'static str,
    pub status: String,
    pub desired: cynic_protocol::NodeSubstrateDesiredState,
    pub observed: NodeSubstrateObservation,
    pub decision: NodeSubstrateDecision,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeSubstrateApplyReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub initial: NodeSubstrateAssessment,
    pub postflight: Option<NodeSubstrateObservation>,
    pub postflight_decision: Option<NodeSubstrateDecision>,
    pub blockers: Vec<String>,
    pub rollback_intent: String,
}

pub fn assess_node_substrate(
    registry_path: Option<&str>,
    node_id: &str,
    observation_path: Option<&str>,
) -> Result<NodeSubstrateAssessment, String> {
    let desired = resolve_node_substrate_desired_state(registry_path, node_id)?;
    let observed = match observation_path {
        Some(path) => {
            let content = std::fs::read(path)
                .map_err(|error| format!("failed to read substrate observation: {error}"))?;
            serde_json::from_slice(&content)
                .map_err(|error| format!("invalid substrate observation: {error}"))?
        }
        None => unknown_observation(&desired.node_id),
    };
    Ok(assess_observed_node_substrate(desired, observed))
}

/// Collects a provider read-back through an injected read-only port before
/// producing the canonical substrate decision. A failed observation remains
/// explicit evidence rather than becoming a CLI transport failure or a guessed
/// provider state.
pub fn assess_live_node_substrate(
    registry_path: Option<&str>,
    node_id: &str,
    observer: &dyn NodeSubstrateObservationPort,
) -> Result<NodeSubstrateAssessment, String> {
    let desired = resolve_node_substrate_desired_state(registry_path, node_id)?;
    let observed = observer
        .observe_node_substrate(&desired)
        .unwrap_or_else(|error| unavailable_observation(&desired, error));
    Ok(assess_observed_node_substrate(desired, observed))
}

/// Assembles read-back already collected through an injected provider port into
/// the canonical pure substrate decision. This boundary never mutates a
/// provider and is shared by offline evidence assessment and live observation.
pub fn assess_observed_node_substrate(
    desired: cynic_protocol::NodeSubstrateDesiredState,
    observed: NodeSubstrateObservation,
) -> NodeSubstrateAssessment {
    let decision = reconcile_node_substrate(&desired, &observed);
    NodeSubstrateAssessment {
        schema: "cynic.node_substrate.assessment.v1",
        status: decision.status.clone(),
        desired,
        observed,
        decision,
    }
}

pub fn apply_node_substrate(
    initial: NodeSubstrateAssessment,
    yes: bool,
    observer: &dyn NodeSubstrateObservationPort,
    controller: &dyn NodeSubstrateControlPort,
) -> NodeSubstrateApplyReport {
    let mut blockers = initial.decision.blockers.clone();
    if !yes {
        blockers.push("explicit_yes_required".to_string());
    }
    let action = initial.decision.action.clone();
    if initial.decision.status != "planned" || !initial.decision.mutates || action.is_none() {
        blockers.push("node_substrate_action_not_planned".to_string());
    }
    if !blockers.is_empty() {
        blockers.sort();
        blockers.dedup();
        return apply_report(initial, false, yes, None, None, blockers);
    }
    let Some(action) = action else {
        return apply_report(
            initial,
            false,
            yes,
            None,
            None,
            vec!["node_substrate_action_missing".to_string()],
        );
    };
    if let Err(error) = controller.apply_node_substrate_action(&initial.desired, &action) {
        return apply_report(
            initial,
            true,
            yes,
            None,
            None,
            vec![format!("node_substrate_apply_failed:{error}")],
        );
    }
    let postflight = match observer.observe_node_substrate(&initial.desired) {
        Ok(observation) => observation,
        Err(error) => {
            return apply_report(
                initial,
                true,
                yes,
                None,
                None,
                vec![format!("node_substrate_read_back_failed:{error}")],
            );
        }
    };
    let postflight_decision = reconcile_node_substrate(&initial.desired, &postflight);
    let provider_state_matches = postflight.power == initial.desired.desired_power
        && postflight.existence == NodeSubstrateExistence::Present
        && postflight.network_attachment == NodeNetworkAttachment::Attached;
    let blockers = if provider_state_matches {
        Vec::new()
    } else {
        vec!["node_substrate_provider_read_back_mismatch".to_string()]
    };
    apply_report(
        initial,
        true,
        yes,
        Some(postflight),
        Some(postflight_decision),
        blockers,
    )
}

fn apply_report(
    initial: NodeSubstrateAssessment,
    mutates: bool,
    requested_apply: bool,
    postflight: Option<NodeSubstrateObservation>,
    postflight_decision: Option<NodeSubstrateDecision>,
    blockers: Vec<String>,
) -> NodeSubstrateApplyReport {
    let status = if blockers.is_empty()
        && postflight_decision
            .as_ref()
            .is_some_and(|d| d.status == "converged")
    {
        "applied"
    } else if blockers.is_empty() && postflight.is_some() {
        "applied_pending_management_proof"
    } else {
        "blocked"
    };
    NodeSubstrateApplyReport {
        schema: "cynic.node_substrate.apply_report.v1",
        status,
        mutates,
        requested_apply,
        rollback_intent: format!(
            "restore provider power state observed before action for {}",
            initial.desired.node_id
        ),
        initial,
        postflight,
        postflight_decision,
        blockers,
    }
}

fn unknown_observation(node_id: &str) -> NodeSubstrateObservation {
    NodeSubstrateObservation {
        schema: NODE_SUBSTRATE_OBSERVATION_SCHEMA.to_string(),
        status: "unknown".to_string(),
        node_id: node_id.to_string(),
        provider: None,
        resource_id: None,
        existence: NodeSubstrateExistence::Unknown,
        power: NodeSubstratePower::Unknown,
        network_attachment: NodeNetworkAttachment::Unknown,
        observed_addresses: Vec::new(),
        management_reachable: None,
        observation_authority: None,
        observed_at_unix_ms: None,
        evidence_refs: Vec::new(),
        unknowns: vec!["provider_observer_not_injected".to_string()],
    }
}

fn unavailable_observation(
    desired: &cynic_protocol::NodeSubstrateDesiredState,
    error: String,
) -> NodeSubstrateObservation {
    let mut observed = unknown_observation(&desired.node_id);
    observed.provider = desired.provider.clone();
    observed.resource_id = desired.resource_id.clone();
    observed
        .unknowns
        .retain(|unknown| unknown != "provider_observer_not_injected");
    observed
        .unknowns
        .push(format!("provider_observation_failed:{error}"));
    observed
}

#[cfg(test)]
mod tests {
    use cynic_protocol::{
        NODE_SUBSTRATE_OBSERVATION_SCHEMA, NodeNetworkAttachment, NodeSubstrateDesiredState,
        NodeSubstrateExistence, NodeSubstrateObservation, NodeSubstratePower,
    };

    fn builder_desired() -> NodeSubstrateDesiredState {
        NodeSubstrateDesiredState {
            node_id: "cynic-builder".to_string(),
            provider: Some("proxmox".to_string()),
            resource_id: Some("lxc/105".to_string()),
            desired_power: NodeSubstratePower::Running,
            provisioning: None,
        }
    }

    fn observed_builder() -> NodeSubstrateObservation {
        NodeSubstrateObservation {
            schema: NODE_SUBSTRATE_OBSERVATION_SCHEMA.to_string(),
            status: "observed".to_string(),
            node_id: "cynic-builder".to_string(),
            provider: Some("proxmox".to_string()),
            resource_id: Some("lxc/105".to_string()),
            existence: NodeSubstrateExistence::Present,
            power: NodeSubstratePower::Running,
            network_attachment: NodeNetworkAttachment::Attached,
            observed_addresses: vec!["10.10.10.119/24".to_string()],
            // Convergence requires the full read-back ladder incl. management
            // reachability (`node_management_reachability_unproven` otherwise).
            management_reachable: Some(true),
            observation_authority: Some("proxmox:pvesh:proxmox-forge".to_string()),
            observed_at_unix_ms: Some(1),
            evidence_refs: vec![
                "proxmox:/nodes/pve/lxc/105/status/current".to_string(),
                "proxmox:/nodes/pve/lxc/105/config".to_string(),
            ],
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn proven_proxmox_builder_observation_assesses_without_mutation() {
        let report = super::assess_observed_node_substrate(builder_desired(), observed_builder());

        assert_eq!(report.status, "converged");
        assert_eq!(report.observed.node_id, "cynic-builder");
        assert!(!report.decision.mutates);
        assert!(report.decision.blockers.is_empty());
    }

    #[test]
    fn incomplete_proxmox_builder_observation_remains_blocked() {
        let mut observed = observed_builder();
        observed.observation_authority = None;
        observed.evidence_refs.clear();

        let report = super::assess_observed_node_substrate(builder_desired(), observed);

        assert_eq!(report.status, "blocked");
        assert!(!report.decision.mutates);
        assert!(
            report
                .decision
                .blockers
                .contains(&"node_substrate_observation_unproven".to_string())
        );
    }

    #[test]
    fn provider_probe_failure_is_retained_as_explicit_unknown() {
        let observed = super::unavailable_observation(
            &builder_desired(),
            "proxmox_pvesh_failed:connection_refused".to_string(),
        );
        let report = super::assess_observed_node_substrate(builder_desired(), observed);

        assert_eq!(report.status, "blocked");
        assert!(!report.decision.mutates);
        assert!(
            !report
                .observed
                .unknowns
                .contains(&"provider_observer_not_injected".to_string())
        );
        assert!(
            report
                .observed
                .unknowns
                .iter()
                .any(|unknown| unknown.starts_with("provider_observation_failed:"))
        );
    }

    #[test]
    fn missing_provider_adapter_remains_explicit_unknown() {
        let path = std::env::temp_dir().join(format!(
            "cynic-node-substrate-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        std::fs::write(
            &path,
            r#"{"nodes":[{"id":"builder","role":"build_runner"}]}"#,
        )
        .expect("registry");

        let report =
            super::assess_node_substrate(Some(path.to_str().expect("path")), "builder", None)
                .expect("assessment");

        assert_eq!(report.status, "blocked");
        assert!(
            report
                .observed
                .unknowns
                .contains(&"provider_observer_not_injected".to_string())
        );
        assert!(
            report
                .decision
                .blockers
                .contains(&"provider_resource_ref_missing".to_string())
        );
        std::fs::remove_file(path).expect("cleanup");
    }
}
