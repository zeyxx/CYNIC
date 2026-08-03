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
        let ssh = node.pointer("/transports/ssh");
        if ssh
            .and_then(|value| value.get("target"))
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            blockers.push("build_runner_source_transport_undeclared".to_string());
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
        for proof in ["cargo_check_observed", "binary_build_observed"] {
            if !proofs.iter().any(|candidate| candidate == proof) {
                blockers.push(format!("build_runner_required_proof_missing:{proof}"));
            }
        }
        blockers
    }

    pub(super) fn publication_blockers(&self) -> Vec<String> {
        if self.role != "build_runner" {
            return Vec::new();
        }
        let Some(desired) = self.root.get("release_publication").cloned() else {
            return vec!["release_publication_desired_state_missing".to_string()];
        };
        let Ok(desired) =
            serde_json::from_value::<cynic_protocol::ReleasePublicationDesiredState>(desired)
        else {
            return vec!["release_publication_desired_state_invalid".to_string()];
        };
        cynic_protocol::validate_release_publication_desired_state(&desired)
    }

    pub(super) fn runtime_binding_blockers(&self) -> Vec<String> {
        if self.role == "kernel_primary" {
            return Vec::new();
        }
        let kernels = self
            .root
            .get("nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|node| node.get("role").and_then(Value::as_str) == Some("kernel_primary"))
            .collect::<Vec<_>>();
        let [kernel] = kernels.as_slice() else {
            return vec![format!("kernel_runtime_node_cardinality:{}", kernels.len())];
        };
        let services = kernel
            .get("services")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|service| service.get("name").and_then(Value::as_str) == Some("cynic-kernel"))
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
        let desired = self.root.get("node_enrollment");
        if desired
            .and_then(|value| value.get("schema"))
            .and_then(Value::as_str)
            != Some("cynic.node_enrollment.desired_state.v1")
        {
            blockers.push("node_enrollment_desired_state_missing".to_string());
        }
        if desired
            .and_then(|value| value.get("authority_role"))
            .and_then(Value::as_str)
            != Some("kernel_primary")
        {
            blockers.push("node_enrollment_authority_role_missing".to_string());
        }
        if desired
            .and_then(|value| value.get("credential_delivery"))
            .and_then(Value::as_str)
            != Some("machine_enrollment_staging")
        {
            blockers.push("node_enrollment_credential_delivery_missing".to_string());
        }
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
            .filter(|proof| !proofs.iter().any(|candidate| candidate.as_str() == *proof))
            .map(|proof| format!("lifecycle_required_proof_missing:{proof}"))
            .collect()
    }
}
