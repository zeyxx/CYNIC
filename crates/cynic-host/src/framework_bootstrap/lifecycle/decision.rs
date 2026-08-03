use super::registry::LifecycleRegistry;
use super::types::{FrameworkLifecycleAudit, FrameworkLifecycleDimension};

pub(super) fn decide(
    registry: &LifecycleRegistry<'_>,
    root_blockers: Vec<String>,
) -> FrameworkLifecycleAudit {
    let required_proofs = registry.required_proofs();
    let specs = [
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
            "build_execution",
            "cynic-node::build_scheduler",
            registry.build_blockers(),
            vec![
                "build_grant_read_back".to_string(),
                "binary_build_observed".to_string(),
            ],
        ),
        (
            "artifact_publication",
            "cynic-node::release_publication",
            registry.publication_blockers(),
            vec!["signed_publication_read_back".to_string()],
        ),
        (
            "runtime_binding",
            "cynic-host::framework_bootstrap",
            registry.runtime_binding_blockers(),
            vec!["kernel_endpoint_observed".to_string()],
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
