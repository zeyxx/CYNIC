use super::types::{
    NODE_SUBSTRATE_DECISION_SCHEMA, NODE_SUBSTRATE_OBSERVATION_SCHEMA,
    NODE_SUBSTRATE_PROVISIONING_SCHEMA, NodeNetworkAttachment, NodeSubstrateAction,
    NodeSubstrateDecision, NodeSubstrateDesiredState, NodeSubstrateExistence,
    NodeSubstrateObservation, NodeSubstratePower, NodeSubstrateProvisioningSpec,
    PROVIDER_CONTROL_DESIRED_STATE_SCHEMA, ProviderControlDesiredState,
};

pub fn validate_provider_control_desired_state(
    desired: &ProviderControlDesiredState,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if desired.schema != PROVIDER_CONTROL_DESIRED_STATE_SCHEMA {
        blockers.push("provider_control_desired_state_schema_mismatch".to_string());
    }
    if desired.provider.trim().is_empty() {
        blockers.push("provider_control_provider_missing".to_string());
    }
    let endpoint_valid = url::Url::parse(&desired.endpoint).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && matches!(url.path(), "" | "/")
    });
    if !endpoint_valid {
        blockers.push("provider_endpoint_not_credential_free_https_origin".to_string());
    }
    if desired.token_identity.is_empty()
        || desired.token_identity.len() > 255
        || !desired.token_identity.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'@' | b'!' | b'.' | b'_' | b'-')
        })
    {
        blockers.push("provider_token_identity_invalid".to_string());
    }
    if desired.credential_ref.trim().is_empty() {
        blockers.push("provider_credential_ref_missing".to_string());
    }
    if desired.ca_ref.as_deref().is_some_and(str::is_empty) {
        blockers.push("provider_ca_ref_invalid".to_string());
    }
    blockers
}

pub fn validate_node_substrate_provisioning(
    provisioning: &NodeSubstrateProvisioningSpec,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if provisioning.schema != NODE_SUBSTRATE_PROVISIONING_SCHEMA {
        blockers.push("node_substrate_provisioning_schema_mismatch".to_string());
    }
    for (value, blocker) in [
        (
            provisioning.image_ref.as_str(),
            "node_substrate_image_ref_missing",
        ),
        (
            provisioning.runtime_seed_ref.as_str(),
            "node_substrate_runtime_seed_ref_missing",
        ),
        (
            provisioning.network_profile_ref.as_str(),
            "node_substrate_network_profile_ref_missing",
        ),
    ] {
        if value.trim().is_empty() {
            blockers.push(blocker.to_string());
        }
    }
    for (value, blocker) in [
        (
            provisioning.image_digest.as_str(),
            "node_substrate_image_digest_invalid",
        ),
        (
            provisioning.runtime_seed_digest.as_str(),
            "node_substrate_runtime_seed_digest_invalid",
        ),
    ] {
        if !valid_sha256(value) {
            blockers.push(blocker.to_string());
        }
    }
    blockers
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

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
        NodeSubstrateExistence::Absent => match desired.provisioning.as_ref() {
            None => decision(
                desired,
                "blocked",
                None,
                false,
                vec!["node_substrate_provisioning_spec_missing".to_string()],
            ),
            Some(provisioning) => {
                let blockers = validate_node_substrate_provisioning(provisioning);
                if blockers.is_empty() {
                    decision(
                        desired,
                        "planned",
                        Some(NodeSubstrateAction::Provision),
                        true,
                        Vec::new(),
                    )
                } else {
                    decision(desired, "blocked", None, false, blockers)
                }
            }
        },
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
    let mut required_read_back = if mutates {
        vec![
            "provider_resource_observed".to_string(),
            "power_state_observed".to_string(),
            "network_attachment_observed".to_string(),
            "management_reachability_observed".to_string(),
        ]
    } else {
        Vec::new()
    };
    if action == Some(NodeSubstrateAction::Provision) {
        required_read_back.extend([
            "provisioned_image_digest_observed".to_string(),
            "runtime_seed_digest_observed".to_string(),
        ]);
    }
    NodeSubstrateDecision {
        schema: NODE_SUBSTRATE_DECISION_SCHEMA.to_string(),
        status: status.to_string(),
        node_id: desired.node_id.clone(),
        action,
        mutates,
        requires_explicit_apply: mutates,
        blockers,
        required_read_back,
    }
}
