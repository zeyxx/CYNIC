use super::types::{
    FLEET_INFERENCE_DEPLOYMENT_SCHEMA, FLEET_INFERENCE_PLAN_SCHEMA, FleetInferenceDeployment,
    FleetInferenceDnsAction, FleetInferenceDnsRecord, FleetInferenceNodeObservation,
    FleetInferencePlan, FleetInferenceTargetAction, FleetInferenceTargetPlan,
};

/// Deterministically reconcile declarative deployment intent and node read-back.
/// This produces no transport call and must remain safe to run repeatedly.
pub fn reconcile_fleet_inference(
    desired: &FleetInferenceDeployment,
    observations: &[FleetInferenceNodeObservation],
) -> FleetInferencePlan {
    let mut blockers = validate_desired(desired);
    let mut residual_unknowns = Vec::new();
    let mut eligible = observations
        .iter()
        .filter(|node| matches_all_tags(node, &desired.selector_tags))
        .collect::<Vec<_>>();
    eligible.sort_by(|left, right| left.node_id.cmp(&right.node_id));

    let mut targets = Vec::new();
    for node in eligible {
        let mut target_blockers = Vec::new();
        if !node.observed {
            target_blockers.push("fleet_inference_node_unobserved".to_string());
        }
        if !node.supported_runtimes.contains(&desired.runtime) {
            target_blockers.push("fleet_inference_runtime_unsupported".to_string());
        }
        let address = node.address.clone().unwrap_or_default();
        if address.trim().is_empty() {
            target_blockers.push("fleet_inference_address_unobserved".to_string());
        }
        if node.evidence_refs.is_empty() {
            target_blockers.push("fleet_inference_evidence_missing".to_string());
        }
        residual_unknowns.extend(node.unknowns.iter().cloned());
        if target_blockers.is_empty() {
            let action = if node.active_runtime == Some(desired.runtime)
                && node.active_model_ref.as_deref() == Some(desired.model_ref.as_str())
                && node.health_ok == Some(true)
            {
                FleetInferenceTargetAction::Keep
            } else if node.active_runtime.is_some() {
                FleetInferenceTargetAction::RollingReplace
            } else {
                FleetInferenceTargetAction::Start
            };
            targets.push(target_plan(
                desired,
                node.node_id.clone(),
                address,
                action,
                Vec::new(),
            ));
        } else {
            residual_unknowns.extend(
                target_blockers
                    .into_iter()
                    .map(|blocker| format!("{}:{blocker}", node.node_id)),
            );
        }
    }
    let actionable = targets
        .iter()
        .filter(|target| target.blockers.is_empty())
        .count();
    if actionable < usize::from(desired.replicas) {
        blockers.push("fleet_inference_insufficient_qualified_replicas".to_string());
    }
    targets.truncate(usize::from(desired.replicas));
    let all_kept = !targets.is_empty()
        && targets.iter().all(|target| {
            target.blockers.is_empty() && target.action == FleetInferenceTargetAction::Keep
        })
        && targets.len() == usize::from(desired.replicas);
    let dns_action = if !blockers.is_empty() {
        FleetInferenceDnsAction::Blocked
    } else if all_kept {
        FleetInferenceDnsAction::Verify
    } else {
        FleetInferenceDnsAction::PublishAfterReadBack
    };
    let dns_records = if dns_action == FleetInferenceDnsAction::Verify {
        targets
            .iter()
            .map(|target| FleetInferenceDnsRecord {
                host: format!("{}.{}", desired.service_name, desired.dns_zone),
                port: desired.port,
                node_id: target.node_id.clone(),
                address: target.address.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    normalize(&mut blockers);
    normalize(&mut residual_unknowns);
    FleetInferencePlan {
        schema: FLEET_INFERENCE_PLAN_SCHEMA.to_string(),
        deployment_id: desired.deployment_id.clone(),
        status: if blockers.is_empty() {
            "ready"
        } else {
            "blocked"
        }
        .to_string(),
        targets,
        dns_action,
        dns_records,
        dns_read_back_required: vec![
            "runtime_health_read_back".to_string(),
            "runtime_model_identity_read_back".to_string(),
            "synapse_dns_resolution_read_back".to_string(),
        ],
        blockers,
        residual_unknowns,
    }
}

fn target_plan(
    desired: &FleetInferenceDeployment,
    node_id: String,
    address: String,
    action: FleetInferenceTargetAction,
    blockers: Vec<String>,
) -> FleetInferenceTargetPlan {
    let bounded_action = match action {
        FleetInferenceTargetAction::Keep => "observe_runtime_and_dns",
        FleetInferenceTargetAction::Start => "node_owned_runtime_start_with_read_back",
        FleetInferenceTargetAction::RollingReplace => {
            "node_owned_rolling_replacement_then_dns_cutover"
        }
    }
    .to_string();
    FleetInferenceTargetPlan {
        node_id,
        address,
        action,
        runtime: desired.runtime,
        model_ref: desired.model_ref.clone(),
        bounded_action,
        required_read_back: vec![
            "runtime_health_read_back".to_string(),
            "runtime_model_identity_read_back".to_string(),
        ],
        blockers,
    }
}

fn validate_desired(desired: &FleetInferenceDeployment) -> Vec<String> {
    let mut blockers = Vec::new();
    if desired.schema != FLEET_INFERENCE_DEPLOYMENT_SCHEMA {
        blockers.push("fleet_inference_deployment_schema_mismatch".to_string());
    }
    if desired.deployment_id.trim().is_empty() {
        blockers.push("fleet_inference_deployment_id_missing".to_string());
    }
    if desired.selector_tags.is_empty()
        || desired
            .selector_tags
            .iter()
            .any(|tag| tag.trim().is_empty())
    {
        blockers.push("fleet_inference_selector_tags_invalid".to_string());
    }
    if desired.replicas == 0 {
        blockers.push("fleet_inference_replicas_zero".to_string());
    }
    if desired.model_ref.trim().is_empty() {
        blockers.push("fleet_inference_model_ref_missing".to_string());
    } else if !is_immutable_model_ref(&desired.model_ref) {
        blockers.push("fleet_inference_model_ref_unpinned".to_string());
    }
    if desired.service_name.trim().is_empty() || desired.dns_zone.trim().is_empty() {
        blockers.push("fleet_inference_dns_name_missing".to_string());
    }
    if desired.port == 0 {
        blockers.push("fleet_inference_port_invalid".to_string());
    }
    blockers
}

fn is_immutable_model_ref(model_ref: &str) -> bool {
    model_ref.contains("@sha256:")
        || model_ref.starts_with("sha256:")
        || model_ref.starts_with("file+sha256:")
}

fn matches_all_tags(node: &FleetInferenceNodeObservation, tags: &[String]) -> bool {
    tags.iter()
        .all(|tag| node.tags.iter().any(|node_tag| node_tag == tag))
}

fn normalize(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet_inference::{FLEET_INFERENCE_OBSERVATION_SCHEMA, FleetInferenceRuntime};

    fn desired() -> FleetInferenceDeployment {
        FleetInferenceDeployment {
            schema: FLEET_INFERENCE_DEPLOYMENT_SCHEMA.to_string(),
            deployment_id: "coder".to_string(),
            selector_tags: vec!["inference".to_string(), "gpu".to_string()],
            replicas: 1,
            runtime: FleetInferenceRuntime::Vllm,
            model_ref: "hf://org/model@sha256:abc".to_string(),
            service_name: "coder".to_string(),
            dns_zone: "synapse.cynic".to_string(),
            port: 8000,
        }
    }

    fn node(id: &str) -> FleetInferenceNodeObservation {
        FleetInferenceNodeObservation {
            schema: FLEET_INFERENCE_OBSERVATION_SCHEMA.to_string(),
            node_id: id.to_string(),
            observed: true,
            tags: vec!["inference".to_string(), "gpu".to_string()],
            address: Some("10.10.10.8".to_string()),
            supported_runtimes: vec![FleetInferenceRuntime::Vllm],
            active_runtime: None,
            active_model_ref: None,
            health_ok: None,
            observed_at_unix_ms: Some(1),
            evidence_refs: vec!["inventory://node".to_string()],
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn selects_tagged_node_and_requires_read_back_before_dns_publish() {
        let mut ignored = node("z-ignored");
        ignored.tags = vec!["inference".to_string()];
        let plan = reconcile_fleet_inference(&desired(), &[ignored, node("a-selected")]);
        assert_eq!(plan.status, "ready");
        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.targets[0].node_id, "a-selected");
        assert_eq!(plan.targets[0].action, FleetInferenceTargetAction::Start);
        assert_eq!(
            plan.dns_action,
            FleetInferenceDnsAction::PublishAfterReadBack
        );
        assert!(plan.dns_records.is_empty());
    }

    #[test]
    fn model_change_is_a_rolling_replacement_not_an_in_place_claim() {
        let mut observed = node("gpu-1");
        observed.active_runtime = Some(FleetInferenceRuntime::Vllm);
        observed.active_model_ref = Some("hf://org/old@sha256:def".to_string());
        observed.health_ok = Some(true);
        let plan = reconcile_fleet_inference(&desired(), &[observed]);
        assert_eq!(plan.status, "ready");
        assert_eq!(
            plan.targets[0].action,
            FleetInferenceTargetAction::RollingReplace
        );
        assert_eq!(
            plan.dns_action,
            FleetInferenceDnsAction::PublishAfterReadBack
        );
    }

    #[test]
    fn only_healthy_matching_runtime_can_keep_dns_records() {
        let mut observed = node("gpu-1");
        observed.active_runtime = Some(FleetInferenceRuntime::Vllm);
        observed.active_model_ref = Some("hf://org/model@sha256:abc".to_string());
        observed.health_ok = Some(true);
        let plan = reconcile_fleet_inference(&desired(), &[observed]);
        assert_eq!(plan.dns_action, FleetInferenceDnsAction::Verify);
        assert_eq!(plan.dns_records.len(), 1);
        assert_eq!(plan.dns_records[0].host, "coder.synapse.cynic");
    }

    #[test]
    fn blocks_unobserved_or_unsupported_nodes() {
        let mut observed = node("gpu-1");
        observed.observed = false;
        observed.supported_runtimes.clear();
        let plan = reconcile_fleet_inference(&desired(), &[observed]);
        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"fleet_inference_insufficient_qualified_replicas".to_string())
        );
        assert_eq!(plan.dns_action, FleetInferenceDnsAction::Blocked);
    }
}
