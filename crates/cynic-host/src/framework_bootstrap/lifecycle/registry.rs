use serde_json::Value;

pub(super) struct LifecycleRegistry<'a> {
    pub root: &'a Value,
    pub role: &'a str,
    pub node: Option<&'a Value>,
    pub role_profile: Option<&'a Value>,
}

impl<'a> LifecycleRegistry<'a> {
    pub(super) fn resolve(root: &'a Value, role: &'a str) -> (Self, Vec<String>) {
        let role_nodes = root
            .get("nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|node| node.get("role").and_then(Value::as_str) == Some(role))
            .collect::<Vec<_>>();
        let mut blockers = Vec::new();
        if role_nodes.len() != 1 {
            blockers.push(format!(
                "lifecycle_role_node_cardinality:{role}:{}",
                role_nodes.len()
            ));
        }
        let node = role_nodes.first().copied();
        let role_profile = root
            .pointer("/lifecycle/kernel_release/host_bootstrap/role_profiles")
            .and_then(|profiles| profiles.get(role));
        if role_profile.is_none() {
            blockers.push(format!("lifecycle_role_profile_missing:{role}"));
        }
        (
            Self {
                root,
                role,
                node,
                role_profile,
            },
            blockers,
        )
    }

    pub(super) fn node_id(&self) -> Option<String> {
        self.node
            .and_then(|node| node.get("id"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }

    pub(super) fn required_proofs(&self) -> Vec<String> {
        self.role_profile
            .and_then(|profile| profile.get("required_proof"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect()
    }

    pub(super) fn source_blockers(&self) -> Vec<String> {
        if self.role != "build_runner" {
            return Vec::new();
        }
        let mut blockers = Vec::new();
        let Some(node) = self.node else {
            return vec!["build_runner_node_missing".to_string()];
        };
        let repository_declared = node
            .pointer("/source_workspace_layout/repositories")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .any(|repository| {
                repository.get("id").and_then(Value::as_str) == Some("cynic")
                    && repository
                        .get("relative_path")
                        .and_then(Value::as_str)
                        .is_some_and(|path| !path.is_empty() && !path.starts_with('/'))
            });
        if !repository_declared {
            blockers.push("build_runner_source_workspace_undeclared".to_string());
        }
        blockers
    }

    pub(super) fn substrate_reconstruction_blockers(&self) -> Vec<String> {
        if self.role != "build_runner" {
            return Vec::new();
        }
        let targets = self
            .root
            .pointer("/fleet_desired/targets")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|target| target.get("role").and_then(Value::as_str) == Some(self.role))
            .collect::<Vec<_>>();
        let [target] = targets.as_slice() else {
            return vec![format!(
                "node_substrate_fleet_target_cardinality:{}",
                targets.len()
            )];
        };
        let Some(provisioning) = target.get("provisioning").cloned() else {
            return vec!["node_substrate_provisioning_spec_missing".to_string()];
        };
        let Ok(provisioning) =
            serde_json::from_value::<cynic_protocol::NodeSubstrateProvisioningSpec>(provisioning)
        else {
            return vec!["node_substrate_provisioning_spec_invalid".to_string()];
        };
        cynic_protocol::validate_node_substrate_provisioning(&provisioning)
    }

    pub(super) fn workload_delivery_blockers(&self) -> Vec<String> {
        if self.role != "build_runner" {
            return Vec::new();
        }
        let capabilities = self
            .root
            .pointer("/node_enrollment/authority_policy/allowed_capabilities")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        cynic_node_protocol::BUILD_SCHEDULER_RUNTIME_CAPABILITIES
            .iter()
            .filter(|capability| !capabilities.contains(**capability))
            .map(|capability| format!("build_scheduler_capability_missing:{capability}"))
            .collect()
    }

    pub(super) fn bootstrap_containment_blockers(&self) -> Vec<String> {
        if self.role != "build_runner" {
            return Vec::new();
        }
        let containment = self
            .node
            .and_then(|node| node.pointer("/build_lifecycle/bootstrap_containment"));
        let Some(containment) = containment else {
            return vec!["build_runner_bootstrap_containment_undeclared".to_string()];
        };
        let mut blockers = Vec::new();
        for (pointer, expected, blocker) in [
            (
                "/schema",
                "cynic.build_runner.bootstrap_containment.v1",
                "build_runner_bootstrap_containment_schema_mismatch",
            ),
            (
                "/authority_class",
                "MachineEnrollment",
                "build_runner_bootstrap_containment_authority_mismatch",
            ),
            (
                "/scope",
                "n_minus_one_seed_only",
                "build_runner_bootstrap_containment_scope_mismatch",
            ),
        ] {
            if containment.pointer(pointer).and_then(Value::as_str) != Some(expected) {
                blockers.push(blocker.to_string());
            }
        }
        let removal = containment
            .get("removal_requires")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        for proof in [
            "node_substrate_provisioning_read_back",
            "source_snapshot_transport_read_back",
            "durable_workload_delivery_read_back",
            "machine_enrollment_without_operator_transport",
            "canonical_build_completion_read_back",
        ] {
            if !removal.contains(proof) {
                blockers.push(format!(
                    "build_runner_bootstrap_removal_condition_missing:{proof}"
                ));
            }
        }
        blockers
    }

    pub(super) fn build_blockers(&self) -> Vec<String> {
        if self.role != "build_runner" {
            return Vec::new();
        }
        let domains = self
            .role_profile
            .and_then(|profile| profile.get("required_domains"))
            .and_then(Value::as_array);
        let has_build_domain = domains
            .into_iter()
            .flatten()
            .any(|domain| domain.as_str() == Some("build"));
        let proofs = self.required_proofs();
        let mut blockers = Vec::new();
        if !has_build_domain {
            blockers.push("build_runner_build_domain_missing".to_string());
        }
        for proof in [
            "cargo_check_observed",
            "binary_build_observed",
            "build_resource_preflight_read_back",
        ] {
            if !proofs.iter().any(|candidate| candidate == proof) {
                blockers.push(format!("build_runner_required_proof_missing:{proof}"));
            }
        }
        let resource_policy = self.node.and_then(|node| node.get("build_lifecycle"));
        if resource_policy
            .and_then(|policy| policy.get("schema"))
            .and_then(Value::as_str)
            != Some("cynic.build_runner.resource_policy.v1")
        {
            blockers.push("build_runner_resource_policy_missing".to_string());
        }
        if resource_policy
            .and_then(|policy| policy.get("minimum_available_bytes"))
            .and_then(Value::as_u64)
            .is_none_or(|bytes| bytes == 0)
        {
            blockers.push("build_runner_minimum_available_bytes_missing".to_string());
        }
        if resource_policy
            .and_then(|policy| policy.get("canonical_target_relative_path"))
            .and_then(Value::as_str)
            .is_none_or(|path| path.is_empty() || path.starts_with('/') || path.contains(".."))
        {
            blockers.push("build_runner_canonical_target_invalid".to_string());
        }
        let cache_candidates = resource_policy
            .and_then(|policy| policy.get("noncanonical_cache_candidates"))
            .and_then(Value::as_array);
        if cache_candidates.is_none_or(|candidates| candidates.is_empty()) {
            blockers.push("build_runner_cache_reconciliation_policy_missing".to_string());
        }
        let reclaim = resource_policy.and_then(|policy| policy.get("canonical_cache_reclaim"));
        if reclaim
            .and_then(|value| value.get("schema"))
            .and_then(Value::as_str)
            != Some("cynic.build_runner.cache_reclaim_policy.v1")
            || reclaim
                .and_then(|value| value.get("trigger"))
                .and_then(Value::as_str)
                != Some("available_bytes_below_minimum")
            || reclaim
                .and_then(|value| value.get("action"))
                .and_then(Value::as_str)
                != Some("rust_remove_bounded_profile")
            || reclaim
                .and_then(|value| value.get("target_profile"))
                .and_then(Value::as_str)
                != Some("debug")
            || !reclaim
                .and_then(|value| value.get("preserve_profiles"))
                .and_then(Value::as_array)
                .is_some_and(|profiles| {
                    profiles
                        .iter()
                        .any(|profile| profile.as_str() == Some("release"))
                })
        {
            blockers.push("build_runner_canonical_cache_reclaim_policy_invalid".to_string());
        }
        blockers
    }

    pub(super) fn publication_blockers(&self) -> Vec<String> {
        if !self.participates_in_runtime_release() {
            return Vec::new();
        }
        let Some(desired) = self.root.get("release_publication").cloned() else {
            return vec!["release_publication_desired_state_missing".to_string()];
        };
        let Ok(desired) =
            serde_json::from_value::<cynic_build::ReleasePublicationDesiredState>(desired)
        else {
            return vec!["release_publication_desired_state_invalid".to_string()];
        };
        cynic_build::validate_release_publication_desired_state(&desired)
    }

    pub(super) fn artifact_packaging_blockers(&self) -> Vec<String> {
        if !self.participates_in_runtime_release() {
            return Vec::new();
        }
        let artifacts = self
            .root
            .pointer("/lifecycle/kernel_release/runtime_artifacts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let Ok(expected) = cynic_protocol::runtime_release_artifact_names() else {
            return vec!["runtime_basket_desired_state_invalid".to_string()];
        };
        let expected = expected
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        if artifacts == expected {
            Vec::new()
        } else {
            vec!["release_artifact_set_incomplete".to_string()]
        }
    }

    pub(super) fn release_signing_blockers(&self) -> Vec<String> {
        if !self.participates_in_runtime_release() {
            return Vec::new();
        }
        let Some(desired) = self.release_publication_desired() else {
            return vec!["release_signing_desired_state_missing".to_string()];
        };
        let mut blockers = Vec::new();
        if desired.signer_identity.trim().is_empty() {
            blockers.push("release_signer_identity_missing".to_string());
        }
        if desired.signature_namespace != cynic_protocol::RUNTIME_RELEASE_SIGNATURE_NAMESPACE {
            blockers.push("release_signature_namespace_mismatch".to_string());
        }
        if !std::path::Path::new(&desired.signing_key_path).is_absolute() {
            blockers.push("release_signing_key_path_not_absolute".to_string());
        }
        blockers
    }

    pub(super) fn artifact_store_blockers(&self) -> Vec<String> {
        if !self.participates_in_runtime_release() {
            return Vec::new();
        }
        let Some(desired) = self.release_publication_desired() else {
            return vec!["artifact_store_desired_state_missing".to_string()];
        };
        let nodes = self
            .root
            .get("nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|node| {
                node.get("role").and_then(Value::as_str) == Some(desired.storage_role.as_str())
            })
            .collect::<Vec<_>>();
        let [node] = nodes.as_slice() else {
            return vec![format!("artifact_store_node_cardinality:{}", nodes.len())];
        };
        let services = node
            .get("services")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|service| {
                service.get("name").and_then(Value::as_str) == Some(desired.service_name.as_str())
            })
            .collect::<Vec<_>>();
        let [service] = services.as_slice() else {
            return vec![format!(
                "artifact_store_service_cardinality:{}",
                services.len()
            )];
        };
        let mut blockers = Vec::new();
        if service.get("scheme").and_then(Value::as_str) != Some("https") {
            blockers.push("artifact_store_https_endpoint_missing".to_string());
        }
        if service
            .get("endpoint_host")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            blockers.push("artifact_store_endpoint_host_missing".to_string());
        }
        if service
            .get("port")
            .and_then(Value::as_u64)
            .is_none_or(|port| port == 0 || port > u64::from(u16::MAX))
        {
            blockers.push("artifact_store_endpoint_port_invalid".to_string());
        }
        if service
            .get("base_path")
            .and_then(Value::as_str)
            .is_none_or(|path| !path.starts_with('/') || path.contains(".."))
        {
            blockers.push("artifact_store_base_path_invalid".to_string());
        }
        for (field, expected, blocker) in [
            (
                "protocol",
                desired.protocol.as_str(),
                "artifact_store_protocol_mismatch",
            ),
            (
                "authentication_kind",
                desired.authentication_kind.as_str(),
                "artifact_store_authentication_mismatch",
            ),
        ] {
            if service.get(field).and_then(Value::as_str) != Some(expected) {
                blockers.push(blocker.to_string());
            }
        }
        if service
            .pointer("/install_authority/channel")
            .and_then(Value::as_str)
            .is_none()
            || service
                .pointer("/install_authority/provenance")
                .and_then(Value::as_str)
                .is_none()
        {
            blockers.push("artifact_store_install_authority_missing".to_string());
        }
        if service.pointer("/health/kind").and_then(Value::as_str) != Some("https")
            || service
                .pointer("/health/path")
                .and_then(Value::as_str)
                .is_none_or(|path| !path.starts_with('/'))
        {
            blockers.push("artifact_store_health_contract_missing".to_string());
        }
        blockers
    }

    pub(super) fn runtime_binding_blockers(&self) -> Vec<String> {
        if self.role == "kernel_primary" {
            return Vec::new();
        }
        let desired = self
            .root
            .get("node_enrollment")
            .cloned()
            .and_then(|desired| {
                serde_json::from_value::<cynic_node_protocol::NodeEnrollmentDesiredState>(desired).ok()
            });
        let authority_role = desired
            .as_ref()
            .map_or("kernel_primary", |desired| desired.authority_role.as_str());
        let endpoint_service = desired
            .as_ref()
            .map_or("cynic-kernel", |desired| desired.endpoint_service.as_str());
        let kernels = self
            .root
            .get("nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|node| node.get("role").and_then(Value::as_str) == Some(authority_role))
            .collect::<Vec<_>>();
        let [kernel] = kernels.as_slice() else {
            return vec![format!("kernel_runtime_node_cardinality:{}", kernels.len())];
        };
        let services = kernel
            .get("services")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|service| service.get("name").and_then(Value::as_str) == Some(endpoint_service))
            .collect::<Vec<_>>();
        let [service] = services.as_slice() else {
            return vec![format!(
                "kernel_runtime_service_cardinality:{}",
                services.len()
            )];
        };
        let mut blockers = Vec::new();
        if !matches!(
            service.get("scheme").and_then(Value::as_str),
            Some("http" | "https")
        ) {
            blockers.push("kernel_runtime_endpoint_scheme_missing".to_string());
        }
        if service
            .get("endpoint_host")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            blockers.push("kernel_runtime_endpoint_host_missing".to_string());
        }
        if service
            .get("port")
            .and_then(Value::as_u64)
            .is_none_or(|port| port == 0 || port > u64::from(u16::MAX))
        {
            blockers.push("kernel_runtime_endpoint_port_invalid".to_string());
        }
        blockers
    }

    pub(super) fn provider_control_blockers(&self) -> Vec<String> {
        let Some(desired) = self.root.get("provider_control").cloned() else {
            return vec!["provider_control_desired_state_missing".to_string()];
        };
        let Ok(desired) =
            serde_json::from_value::<cynic_protocol::ProviderControlDesiredState>(desired)
        else {
            return vec!["provider_control_desired_state_invalid".to_string()];
        };
        cynic_protocol::validate_provider_control_desired_state(&desired)
    }

    pub(super) fn enrollment_blockers(&self) -> Vec<String> {
        if self.role == "kernel_primary" {
            return Vec::new();
        }
        let mut blockers = Vec::new();
        let authority = self
            .node
            .and_then(|node| node.pointer("/transports/ssh/enrollment_authority"));
        if authority
            .and_then(|value| value.get("class"))
            .and_then(Value::as_str)
            != Some("machine_enrollment")
        {
            blockers.push("machine_enrollment_transport_authority_missing".to_string());
        }
        let Some(desired) = self.root.get("node_enrollment").cloned() else {
            blockers.push("node_enrollment_desired_state_missing".to_string());
            return blockers;
        };
        let Ok(desired) =
            serde_json::from_value::<cynic_node_protocol::NodeEnrollmentDesiredState>(desired)
        else {
            blockers.push("node_enrollment_desired_state_invalid".to_string());
            return blockers;
        };
        blockers.extend(cynic_node_protocol::validate_node_enrollment_desired_state(
            &desired,
        ));
        blockers
    }

    pub(super) fn activation_blockers(&self) -> Vec<String> {
        let services = self
            .node
            .and_then(|node| node.get("services"))
            .and_then(Value::as_array);
        let service = services
            .into_iter()
            .flatten()
            .find(|service| service.get("name").and_then(Value::as_str) == Some("cynic-node"));
        let Some(service) = service else {
            return vec!["node_runtime_service_missing".to_string()];
        };
        let mut blockers = Vec::new();
        if service
            .pointer("/supervisor/provider")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            blockers.push("node_runtime_supervisor_missing".to_string());
        }
        if service.get("health").is_none() {
            blockers.push("node_runtime_health_contract_missing".to_string());
        }
        blockers
    }

    pub(super) fn read_back_blockers(&self) -> Vec<String> {
        let proofs = self.required_proofs();
        let required = if self.role == "build_runner" {
            [
                "runtime_marker_read_back",
                "node_control_config_read_back",
                "node_service_active_read_back",
                "bootstrap_completion_read_back",
            ]
            .as_slice()
        } else {
            ["runtime_marker_read_back", "node_service_active_read_back"].as_slice()
        };
        required
            .iter()
            .filter(|proof| !proofs.iter().any(|candidate| candidate.as_str() == **proof))
            .map(|proof| format!("lifecycle_required_proof_missing:{proof}"))
            .collect()
    }

    fn release_publication_desired(&self) -> Option<cynic_build::ReleasePublicationDesiredState> {
        self.root
            .get("release_publication")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
    }

    fn participates_in_runtime_release(&self) -> bool {
        let runtime_artifacts = self
            .root
            .pointer("/lifecycle/kernel_release/runtime_artifacts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        self.node
            .and_then(|node| node.get("services"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|service| service.get("artifact").and_then(Value::as_str))
            .any(|artifact| runtime_artifacts.contains(artifact))
    }
}
