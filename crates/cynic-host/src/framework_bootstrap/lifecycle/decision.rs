use super::registry::LifecycleRegistry;
use super::types::{FrameworkLifecycleAudit, FrameworkLifecycleDimension};

pub(super) fn decide(
    registry: &LifecycleRegistry<'_>,
    root_blockers: Vec<String>,
) -> FrameworkLifecycleAudit {
    let required_proofs = registry.required_proofs();
    let specs = [
        (
            "substrate_reconstruction",
            "cynic-protocol::provider",
            registry.substrate_reconstruction_blockers(),
            vec![
                "node_substrate_provisioning_read_back".to_string(),
                "provisioned_image_digest_observed".to_string(),
                "runtime_seed_digest_observed".to_string(),
            ],
        ),
        (
            "source_materialization",
            "cynic-host::source_materialization",
            registry.source_blockers(),
            vec![
                "source_commit_read_back".to_string(),
                "source_worktree_clean".to_string(),
            ],
        ),
        (
            "source_snapshot_recovery",
            "cynic-node::runtime_artifact_transport",
            Vec::new(),
            vec![
                "source_snapshot_transport_read_back".to_string(),
                "source_snapshot_digest_read_back".to_string(),
            ],
        ),
        (
            "workload_delivery_recovery",
            "cynic-node::build_scheduler",
            registry.workload_delivery_blockers(),
            vec![
                "durable_workload_delivery_read_back".to_string(),
                "delivery_lease_fence_read_back".to_string(),
                "scheduler_recovery_read_back".to_string(),
            ],
        ),
        (
            "build_execution",
            "cynic-node::build_scheduler",
            registry.build_blockers(),
            vec![
                "build_grant_read_back".to_string(),
                "binary_build_observed".to_string(),
            ],
        ),
        (
            "artifact_packaging",
            "cynic-node::local_release_basket_packaging",
            registry.artifact_packaging_blockers(),
            vec!["release_basket_digest_read_back".to_string()],
        ),
        (
            "release_signing",
            "cynic-node::release_basket_signing",
            registry.release_signing_blockers(),
            vec!["detached_signature_read_back".to_string()],
        ),
        (
            "artifact_store",
            "storage_primary::runtime_artifact_store",
            registry.artifact_store_blockers(),
            vec!["artifact_store_health_read_back".to_string()],
        ),
        (
            "artifact_publication",
            "cynic-node::release_publication",
            registry.publication_blockers(),
            vec!["signed_publication_read_back".to_string()],
        ),
        (
            "runtime_binding",
            "cynic-host::framework_bootstrap::runtime_binding",
            registry.runtime_binding_blockers(),
            vec!["kernel_endpoint_observed".to_string()],
        ),
        (
            "provider_control_identity",
            "cynic-host::framework_bootstrap::provider_enrollment",
            registry.provider_control_blockers(),
            vec![
                "provider_config_owner_group_mode_read_back".to_string(),
                "provider_token_presence_without_value".to_string(),
                "provider_configuration_binding_read_back".to_string(),
                "provider_ca_binding_read_back".to_string(),
                "provider_api_authentication_read_back".to_string(),
            ],
        ),
        (
            "machine_enrollment",
            "cynic-protocol::node_enrollment",
            registry.enrollment_blockers(),
            vec![
                "enrollment_authority_read_back".to_string(),
                "one_time_token_consumed".to_string(),
                "signed_enrollment_grant_read_back".to_string(),
            ],
        ),
        (
            "activation",
            "cynic-host::service_reconcile",
            registry.activation_blockers(),
            vec!["node_service_active_read_back".to_string()],
        ),
        (
            "terminal_read_back",
            "cynic-host::framework_bootstrap",
            registry.read_back_blockers(),
            required_proofs,
        ),
        (
            "bootstrap_containment_retirement",
            "cynic-host::framework_bootstrap",
            registry.bootstrap_containment_blockers(),
            vec![
                "bootstrap_containment_scope_read_back".to_string(),
                "bootstrap_removal_conditions_read_back".to_string(),
            ],
        ),
    ];
    let dimensions = specs
        .into_iter()
        .map(
            |(name, owner, blockers, proofs)| FrameworkLifecycleDimension {
                name,
                desired_status: if blockers.is_empty() {
                    "declared"
                } else {
                    "incomplete"
                },
                observation_status: "unknown",
                convergence_status: if blockers.is_empty() {
                    "observation_required"
                } else {
                    "blocked_desired_state"
                },
                owner,
                required_proofs: proofs,
                blockers,
            },
        )
        .collect::<Vec<_>>();
    let mut blockers = root_blockers;
    blockers.extend(
        dimensions
            .iter()
            .flat_map(|dimension| dimension.blockers.iter().cloned()),
    );
    blockers.sort();
    blockers.dedup();
    let next_actions = blockers
        .iter()
        .map(|blocker| match blocker.as_str() {
            "kernel_runtime_endpoint_scheme_missing"
            | "kernel_runtime_endpoint_host_missing"
            | "kernel_runtime_endpoint_port_invalid" => {
                "declare_and_observe_kernel_runtime_endpoint"
            }
            "node_enrollment_desired_state_missing"
            | "node_enrollment_authority_role_missing"
            | "node_enrollment_credential_delivery_missing" => {
                "declare_canonical_node_enrollment_desired_state"
            }
            "provider_control_desired_state_missing"
            | "provider_control_desired_state_invalid"
            | "provider_control_desired_state_schema_mismatch"
            | "provider_control_provider_missing"
            | "provider_endpoint_not_credential_free_https_origin"
            | "provider_token_identity_invalid"
            | "provider_credential_ref_missing"
            | "provider_ca_ref_invalid" => "declare_canonical_provider_control_desired_state",
            "artifact_store_desired_state_missing"
            | "artifact_store_https_endpoint_missing"
            | "artifact_store_endpoint_host_missing"
            | "artifact_store_endpoint_port_invalid"
            | "artifact_store_health_contract_missing" => {
                "declare_and_observe_runtime_artifact_store"
            }
            "node_substrate_provisioning_spec_missing"
            | "node_substrate_provisioning_spec_invalid" => {
                "declare_content_addressed_substrate_reconstruction"
            }
            blocker if blocker.starts_with("build_scheduler_capability_missing:") => {
                "complete_node_owned_scheduler_capability_contract"
            }
            blocker if blocker.starts_with("build_runner_bootstrap_") => {
                "bound_and_retire_n_minus_one_bootstrap_containment"
            }
            blocker if blocker.starts_with("lifecycle_required_proof_missing:") => {
                "extend_role_terminal_read_back_contract"
            }
            _ => "repair_dimension_desired_state",
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut next_actions = next_actions;
    next_actions.sort();
    next_actions.dedup();
    FrameworkLifecycleAudit {
        schema: "cynic.framework_bootstrap.lifecycle_audit.v1",
        status: if blockers.is_empty() {
            "observation_required"
        } else {
            "blocked"
        },
        mutates: false,
        role: registry.role.to_string(),
        node_id: registry.node_id(),
        dimensions,
        blockers,
        next_actions,
        residual_unknowns: vec![
            "runtime observation was not supplied by this declarative audit".to_string(),
            "no lifecycle dimension is converged until substrate read-back is retained".to_string(),
        ],
    }
}
