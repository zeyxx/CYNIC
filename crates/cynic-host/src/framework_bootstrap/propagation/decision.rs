use super::types::{FrameworkPropagationDecision, FrameworkPropagationObservation};

pub(super) fn decide_framework_propagation(
    expected_basket_digest: &str,
    node_id: &str,
    enrollment_authority_class: &str,
    enrollment_principal: &str,
    observation: &FrameworkPropagationObservation,
    mut blockers: Vec<String>,
) -> FrameworkPropagationDecision {
    if !blockers.is_empty() {
        return blocked(blockers);
    }
    if !observation.ssh_authenticated {
        blockers.push("ssh_authentication_unproven".to_string());
        return blocked(blockers);
    }
    if enrollment_authority_class != "machine_enrollment" {
        blockers.push("machine_enrollment_authority_not_declared".to_string());
    }
    if observation.remote_principal.as_deref() != Some(enrollment_principal) {
        blockers.push("machine_enrollment_principal_mismatch".to_string());
    }
    if !observation.unknowns.is_empty() {
        blockers.extend(observation.unknowns.iter().cloned());
    }
    if !blockers.is_empty() {
        return blocked(blockers);
    }

    match (
        observation.runtime_marker.as_ref(),
        observation.node_observation.as_ref(),
    ) {
        (None, None) => FrameworkPropagationDecision {
            schema: "cynic.framework_bootstrap.propagation_decision.v1",
            status: "enroll_with_machine_authority",
            next_action: "stage_and_enroll_content_addressed_seed",
            authority_class: Some("MachineEnrollment"),
            blockers,
        },
        (None, Some(node)) if valid_node_observation(node, node_id) => {
            FrameworkPropagationDecision {
                schema: "cynic.framework_bootstrap.propagation_decision.v1",
                status: "enroll_with_machine_authority",
                next_action: "resume_seed_enrollment_with_machine_authority",
                authority_class: Some("MachineEnrollment"),
                blockers,
            }
        }
        (Some(marker), Some(node))
            if valid_runtime_marker(marker, expected_basket_digest)
                && valid_node_observation(node, node_id) =>
        {
            FrameworkPropagationDecision {
                schema: "cynic.framework_bootstrap.propagation_decision.v1",
                status: "converged",
                next_action: "none",
                authority_class: None,
                blockers,
            }
        }
        (Some(marker), _) if !valid_runtime_marker(marker, expected_basket_digest) => {
            blockers.push("existing_runtime_requires_signed_release_reconciliation".to_string());
            blocked(blockers)
        }
        _ => {
            blockers.push("partial_bootstrap_requires_operator_recovery".to_string());
            blocked(blockers)
        }
    }
}

fn valid_runtime_marker(marker: &serde_json::Value, expected_basket_digest: &str) -> bool {
    marker["schema"].as_str() == Some("cynic.runtime_seed.release_marker.v3")
        && marker["release_epoch"]
            .as_u64()
            .is_some_and(|epoch| epoch > 0)
        && marker["basket_digest"].as_str() == Some(expected_basket_digest)
}

fn valid_node_observation(observation: &serde_json::Value, node_id: &str) -> bool {
    observation["schema"].as_str() == Some("cynic.runtime.node_observation.v1")
        && observation["status"].as_str() == Some("observed")
        && observation["node_id"].as_str() == Some(node_id)
}

fn blocked(blockers: Vec<String>) -> FrameworkPropagationDecision {
    FrameworkPropagationDecision {
        schema: "cynic.framework_bootstrap.propagation_decision.v1",
        status: "blocked",
        next_action: "resolve_blockers_then_reobserve",
        authority_class: None,
        blockers,
    }
}
