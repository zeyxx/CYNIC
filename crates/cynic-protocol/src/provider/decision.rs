use super::types::{
    NODE_SUBSTRATE_DECISION_SCHEMA, NODE_SUBSTRATE_OBSERVATION_SCHEMA, NodeNetworkAttachment,
    NodeSubstrateAction, NodeSubstrateDecision, NodeSubstrateDesiredState, NodeSubstrateExistence,
    NodeSubstrateObservation, NodeSubstratePower,
};

pub fn reconcile_node_substrate(
    desired: &NodeSubstrateDesiredState,
    observed: &NodeSubstrateObservation,
) -> NodeSubstrateDecision {
    let mut blockers = Vec::new();
    if desired.node_id.trim().is_empty() || desired.node_id != observed.node_id {
        blockers.push("node_substrate_identity_mismatch".to_string());
    }
    if desired.provider.as_deref().is_none_or(str::is_empty)
        || desired.resource_id.as_deref().is_none_or(str::is_empty)
    {
        blockers.push("provider_resource_ref_missing".to_string());
    }
    if desired.provider != observed.provider || desired.resource_id != observed.resource_id {
        blockers.push("provider_resource_observation_mismatch".to_string());
    }
    if observed.schema != NODE_SUBSTRATE_OBSERVATION_SCHEMA
        || observed.status != "observed"
        || observed
            .observation_authority
            .as_deref()
            .is_none_or(str::is_empty)
        || observed.observed_at_unix_ms.is_none()
        || observed.evidence_refs.is_empty()
    {
        blockers.push("node_substrate_observation_unproven".to_string());
    }
    if !blockers.is_empty() {
        return decision(
            desired,
            "blocked",
            Some(NodeSubstrateAction::ObserveProvider),
            false,
            blockers,
        );
    }

    match observed.existence {
        NodeSubstrateExistence::Unknown => decision(
            desired,
            "blocked",
            Some(NodeSubstrateAction::ObserveProvider),
            false,
            vec!["node_substrate_existence_unknown".to_string()],
        ),
        NodeSubstrateExistence::Absent => decision(
            desired,
            "blocked",
            Some(NodeSubstrateAction::RequestEnrollment),
            false,
            vec!["node_substrate_absent".to_string()],
        ),
        NodeSubstrateExistence::Present => reconcile_present_node(desired, observed),
    }
}

fn reconcile_present_node(
    desired: &NodeSubstrateDesiredState,
    observed: &NodeSubstrateObservation,
) -> NodeSubstrateDecision {
    if observed.power == NodeSubstratePower::Unknown {
        return decision(
            desired,
            "blocked",
            Some(NodeSubstrateAction::ObserveProvider),
            false,
            vec!["node_substrate_power_unknown".to_string()],
        );
    }
    if desired.desired_power != observed.power {
        let action = match desired.desired_power {
            NodeSubstratePower::Running => NodeSubstrateAction::Start,
            NodeSubstratePower::Stopped => NodeSubstrateAction::Stop,
            NodeSubstratePower::Unknown => {
                return decision(
                    desired,
                    "blocked",
                    None,
                    false,
                    vec!["desired_power_unknown".to_string()],
                );
            }
        };
        return decision(desired, "planned", Some(action), true, Vec::new());
    }
    match observed.network_attachment {
        NodeNetworkAttachment::Unknown => decision(
            desired,
            "blocked",
            Some(NodeSubstrateAction::ObserveProvider),
            false,
            vec!["node_network_attachment_unknown".to_string()],
        ),
        NodeNetworkAttachment::Detached => decision(
            desired,
            "planned",
            Some(NodeSubstrateAction::AttachNetwork),
            true,
            Vec::new(),
        ),
        NodeNetworkAttachment::Attached if observed.management_reachable != Some(true) => decision(
            desired,
            "blocked",
            Some(NodeSubstrateAction::ObserveManagement),
            false,
            vec!["node_management_reachability_unproven".to_string()],
        ),
        NodeNetworkAttachment::Attached => decision(desired, "converged", None, false, Vec::new()),
    }
}

fn decision(
    desired: &NodeSubstrateDesiredState,
    status: &str,
    action: Option<NodeSubstrateAction>,
    mutates: bool,
    blockers: Vec<String>,
) -> NodeSubstrateDecision {
    NodeSubstrateDecision {
        schema: NODE_SUBSTRATE_DECISION_SCHEMA.to_string(),
        status: status.to_string(),
        node_id: desired.node_id.clone(),
        action,
        mutates,
        requires_explicit_apply: mutates,
        blockers,
        required_read_back: if mutates {
            vec![
                "provider_resource_observed".to_string(),
                "power_state_observed".to_string(),
                "network_attachment_observed".to_string(),
                "management_reachability_observed".to_string(),
            ]
        } else {
            Vec::new()
        },
    }
}
