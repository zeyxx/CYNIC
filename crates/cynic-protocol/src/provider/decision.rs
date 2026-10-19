use super::types::{
    LXC_PROVISIONING_SCHEMA, LxcProvisioningSpec, NODE_SUBSTRATE_DECISION_SCHEMA,
    NODE_SUBSTRATE_OBSERVATION_SCHEMA, NODE_SUBSTRATE_PROVISIONING_SCHEMA, NodeNetworkAttachment,
    NodeSubstrateAction, NodeSubstrateDecision, NodeSubstrateDesiredState, NodeSubstrateExistence,
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
    if let Some(lxc) = provisioning.lxc.as_ref() {
        // LXC container substrate: content-addressed fields are replaced by
        // concrete container parameters validated by their own predicate.
        return validate_lxc_provisioning_spec(lxc);
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

/// Validates concrete Proxmox LXC container provisioning parameters.
/// Pure predicate: zero I/O, returns blocker codes, never panics.
pub fn validate_lxc_provisioning_spec(lxc: &LxcProvisioningSpec) -> Vec<String> {
    let mut blockers = Vec::new();
    if lxc.schema != LXC_PROVISIONING_SCHEMA {
        blockers.push("lxc_provisioning_schema_mismatch".to_string());
    }
    if !valid_lxc_hostname(&lxc.hostname) {
        blockers.push("lxc_hostname_invalid".to_string());
    }
    if !valid_template_volid(&lxc.template_volid) {
        blockers.push("lxc_template_volid_invalid".to_string());
    }
    if lxc.storage_pool.trim().is_empty() {
        blockers.push("lxc_storage_pool_missing".to_string());
    }
    if lxc.cores == 0 {
        blockers.push("lxc_cores_zero".to_string());
    }
    if lxc.memory_mib < 512 {
        blockers.push("lxc_memory_below_512mib_minimum".to_string());
    }
    if lxc.disk_gib < 4 {
        blockers.push("lxc_disk_below_4gib_minimum".to_string());
    }
    if lxc.bridge.trim().is_empty() {
        blockers.push("lxc_bridge_missing".to_string());
    }
    if let Some(ip_cidr) = lxc.ip_cidr.as_deref() {
        if !valid_ipv4_cidr(ip_cidr) {
            blockers.push("lxc_ip_cidr_invalid".to_string());
        }
        let gateway_valid = lxc
            .gateway
            .as_deref()
            .is_some_and(|gateway| valid_ipv4(gateway));
        if !gateway_valid {
            blockers.push("lxc_gateway_missing_or_invalid_for_static_ip".to_string());
        }
    }
    if let Some(nameserver) = lxc.nameserver.as_deref() {
        if !valid_ipv4(nameserver) {
            blockers.push("lxc_nameserver_invalid".to_string());
        }
    }
    for feature in &lxc.features {
        let chars_valid = feature
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'=' | b'-' | b'_' | b','));
        let shape_valid = !feature.contains('=') || feature.split('=').count() == 2;
        if !chars_valid || !shape_valid {
            blockers.push("lxc_feature_invalid".to_string());
            break;
        }
    }
    blockers
}

/// RFC 1123 hostname: labels of alphanumeric/hyphen, 1-63 chars each,
/// total length 1-63, no leading/trailing hyphen per label.
fn valid_lxc_hostname(hostname: &str) -> bool {
    if hostname.is_empty() || hostname.len() > 63 {
        return false;
    }
    hostname.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

/// Storage volume id shape: "<storage>:vztmpl/<filename>" with safe chars.
fn valid_template_volid(volid: &str) -> bool {
    let Some((storage, rest)) = volid.split_once(':') else {
        return false;
    };
    if storage.trim().is_empty() || !rest.starts_with("vztmpl/") {
        return false;
    }
    let filename = &rest["vztmpl/".len()..];
    !filename.is_empty()
        && filename
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_ipv4(value: &str) -> bool {
    let mut octets = value.split('.');
    let mut count = 0;
    for octet in octets.by_ref() {
        count += 1;
        if count > 4 {
            return false;
        }
        let Ok(parsed) = octet.parse::<u8>() else {
            return false;
        };
        let _ = parsed;
    }
    count == 4 && value.split('.').count() == 4
}

fn valid_ipv4_cidr(value: &str) -> bool {
    let Some((address, prefix)) = value.split_once('/') else {
        return false;
    };
    if !valid_ipv4(address) {
        return false;
    }
    match prefix.parse::<u8>() {
        Ok(prefix) => prefix <= 32,
        Err(_) => false,
    }
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
