/// The node-owned execution context selected before any execution surface is
/// activated. This type is containment only: a mode never grants authority that
/// the receiving surface does not independently admit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeExecutionMode {
    /// Temporary, closed bootstrap. It can materialize the admitted source,
    /// run the canonical bootstrap build, and retain its completion evidence.
    BootstrapSeed,
    /// Steady-state node execution. Individual surfaces retain their own
    /// admission, capability, claim, and proof requirements.
    Runtime,
}

/// A node execution surface whose availability is additionally bounded by the
/// selected `NodeExecutionMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeExecutionSurface {
    SourceMaterialization,
    BootstrapBuild,
    CompletionEvidence,
    McpServing,
    WorkloadDelivery,
    BuilderHttp,
    ReleasePublication,
    FleetMutation,
    RuntimeActivation,
    ArbitraryArgv,
}

/// Returns whether the selected mode permits this surface to proceed to its
/// own admission boundary. `Runtime` is not a grant; it simply adds no
/// bootstrap-containment denial.
pub fn node_execution_mode_allows(mode: NodeExecutionMode, surface: NodeExecutionSurface) -> bool {
    match mode {
        NodeExecutionMode::BootstrapSeed => matches!(
            surface,
            NodeExecutionSurface::SourceMaterialization
                | NodeExecutionSurface::BootstrapBuild
                | NodeExecutionSurface::CompletionEvidence
        ),
        NodeExecutionMode::Runtime => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_seed_mode_allows_only_source_build_and_completion_surfaces() {
        for surface in [
            NodeExecutionSurface::SourceMaterialization,
            NodeExecutionSurface::BootstrapBuild,
            NodeExecutionSurface::CompletionEvidence,
        ] {
            assert!(node_execution_mode_allows(
                NodeExecutionMode::BootstrapSeed,
                surface
            ));
        }
    }

    #[test]
    fn bootstrap_seed_mode_denies_runtime_ingress_publication_and_arbitrary_execution() {
        for surface in [
            NodeExecutionSurface::McpServing,
            NodeExecutionSurface::WorkloadDelivery,
            NodeExecutionSurface::BuilderHttp,
            NodeExecutionSurface::ReleasePublication,
            NodeExecutionSurface::FleetMutation,
            NodeExecutionSurface::RuntimeActivation,
            NodeExecutionSurface::ArbitraryArgv,
        ] {
            assert!(!node_execution_mode_allows(
                NodeExecutionMode::BootstrapSeed,
                surface
            ));
        }
    }
}
