use cynic_node_protocol::{NodeExecutionMode, NodeExecutionSurface, node_execution_mode_allows};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuilderActivationInput {
    pub build_scheduler_configured: bool,
    pub role_is_build_runner: bool,
    pub enrollment_proven: bool,
    pub scheduler_capabilities_proven: bool,
    pub signing_key_present: bool,
    pub durable_grant_present: bool,
    pub runtime_registry_present: bool,
    pub execution_mode: NodeExecutionMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuilderActivationDecision {
    pub starts_scheduler: bool,
    pub starts_builder_http: bool,
    pub blockers: Vec<&'static str>,
}

pub fn decide_builder_activation(input: &BuilderActivationInput) -> BuilderActivationDecision {
    let mut blockers = Vec::new();
    if !input.build_scheduler_configured {
        blockers.push("build_scheduler_not_configured");
    }
    if !input.role_is_build_runner {
        blockers.push("builder_role_not_build_runner");
    }
    if !input.enrollment_proven {
        blockers.push("build_scheduler_enrollment_read_back_missing");
    }
    if !input.scheduler_capabilities_proven {
        blockers.push("build_scheduler_capability_read_back_missing");
    }
    if !input.signing_key_present {
        blockers.push("build_scheduler_node_signing_key_missing");
    }
    if !input.durable_grant_present {
        blockers.push("build_scheduler_durable_enrollment_grant_missing");
    }
    if !input.runtime_registry_present {
        blockers.push("build_scheduler_runtime_registry_missing");
    }
    if !node_execution_mode_allows(input.execution_mode, NodeExecutionSurface::BuilderHttp) {
        blockers.push("bootstrap_seed_builder_ingress_forbidden");
    }

    BuilderActivationDecision {
        starts_scheduler: blockers.is_empty(),
        starts_builder_http: blockers.is_empty(),
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_runner_role_without_durable_inputs_blocks_every_builder_surface() {
        let decision = decide_builder_activation(&BuilderActivationInput {
            build_scheduler_configured: true,
            role_is_build_runner: true,
            enrollment_proven: false,
            scheduler_capabilities_proven: false,
            signing_key_present: false,
            durable_grant_present: false,
            runtime_registry_present: false,
            execution_mode: NodeExecutionMode::Runtime,
        });

        assert!(!decision.starts_scheduler);
        assert!(!decision.starts_builder_http);
        assert_eq!(
            decision.blockers,
            vec![
                "build_scheduler_enrollment_read_back_missing",
                "build_scheduler_capability_read_back_missing",
                "build_scheduler_node_signing_key_missing",
                "build_scheduler_durable_enrollment_grant_missing",
                "build_scheduler_runtime_registry_missing",
            ]
        );
    }

    #[test]
    fn proven_build_runner_inputs_activate_scheduler_and_builder_http_together() {
        let decision = decide_builder_activation(&BuilderActivationInput {
            build_scheduler_configured: true,
            role_is_build_runner: true,
            enrollment_proven: true,
            scheduler_capabilities_proven: true,
            signing_key_present: true,
            durable_grant_present: true,
            runtime_registry_present: true,
            execution_mode: NodeExecutionMode::Runtime,
        });

        assert!(decision.starts_scheduler);
        assert!(decision.starts_builder_http);
        assert!(decision.blockers.is_empty());
    }

    #[test]
    fn bootstrap_seed_mode_cannot_activate_runtime_builder_surfaces() {
        let decision = decide_builder_activation(&BuilderActivationInput {
            build_scheduler_configured: true,
            role_is_build_runner: true,
            enrollment_proven: true,
            scheduler_capabilities_proven: true,
            signing_key_present: true,
            durable_grant_present: true,
            runtime_registry_present: true,
            execution_mode: NodeExecutionMode::BootstrapSeed,
        });

        assert!(!decision.starts_scheduler);
        assert!(!decision.starts_builder_http);
        assert_eq!(
            decision.blockers,
            vec!["bootstrap_seed_builder_ingress_forbidden"]
        );
    }
}
