use serde::Serialize;

pub const DEVELOPMENT_SURFACE_MANIFEST_SCHEMA: &str = "cynic.development_surface_manifest.v1";

#[derive(Debug, Clone, Serialize)]
pub struct DevelopmentSurfaceManifest {
    pub schema: &'static str,
    pub rule: &'static str,
    pub surfaces: Vec<DevelopmentSurface>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DevelopmentSurface {
    pub id: &'static str,
    pub layer: &'static str,
    pub owner: &'static str,
    pub responsibility: &'static str,
    pub mutation_scope: &'static str,
    pub canonical_commands: Vec<&'static str>,
    pub exit_evidence: Vec<&'static str>,
    pub anti_responsibilities: Vec<&'static str>,
}

pub fn development_surface_manifest() -> DevelopmentSurfaceManifest {
    DevelopmentSurfaceManifest {
        schema: DEVELOPMENT_SURFACE_MANIFEST_SCHEMA,
        rule: "Each development surface owns one kind of proof; no surface may become the global orchestrator.",
        surfaces: vec![
            DevelopmentSurface {
                id: "protocol_contracts",
                layer: "l0_language",
                owner: "cynic-protocol",
                responsibility: "Define shared schemas, runtime gates, planner rules, proof envelopes, and development surface taxonomy.",
                mutation_scope: "Rust protocol types and pure functions only",
                canonical_commands: vec![
                    "cargo test -p cynic-protocol",
                    "cargo check -p cynic-protocol",
                ],
                exit_evidence: vec!["protocol_tests_pass", "public_api_compiles"],
                anti_responsibilities: vec![
                    "read host filesystem",
                    "execute OS probes",
                    "own CLI-only behavior",
                ],
            },
            DevelopmentSurface {
                id: "host_substrate",
                layer: "l1_host",
                owner: "cynic-host",
                responsibility: "Observe and prepare local OS substrate: principal, privileges, packages, filesystem, and services.",
                mutation_scope: "Host-local bounded operations only after explicit apply",
                canonical_commands: vec![
                    "make local-host-proof",
                    "make local-host-apply-report",
                    "cynic bootstrap host supervisor --profile network-node",
                    "cargo test -p cynic-host",
                ],
                exit_evidence: vec!["host_identity_observed", "service_supervisor_observation"],
                anti_responsibilities: vec![
                    "decide fleet topology",
                    "own network mesh semantics",
                    "persist global organism memory",
                ],
            },
            DevelopmentSurface {
                id: "node_network_runtime",
                layer: "l2_network",
                owner: "cynic-node",
                responsibility: "Observe node/network runtime, generate DNS/provider-fallback artifacts, and emit network runtime proofs.",
                mutation_scope: "Network artifacts and bounded network actuator only",
                canonical_commands: vec![
                    "make local-network-proof",
                    "cargo test -p cynic-node --lib",
                ],
                exit_evidence: vec!["network_runtime_observed"],
                anti_responsibilities: vec![
                    "install host OS prerequisites",
                    "own protocol schemas",
                    "pretend provider fallback is canonical mesh",
                ],
            },
            DevelopmentSurface {
                id: "operator_cli",
                layer: "adapter",
                owner: "cynic-cli",
                responsibility: "Read local inputs, call lower crates, serialize reports, and provide ergonomic operator commands.",
                mutation_scope: "No domain rules; CLI I/O and command dispatch only",
                canonical_commands: vec!["make e2e", "make runtime-validate"],
                exit_evidence: vec!["local_e2e_report", "runtime_validation_report"],
                anti_responsibilities: vec![
                    "own planner rules",
                    "define runtime contracts",
                    "hide blocked runtime as success",
                ],
            },
            DevelopmentSurface {
                id: "kernel_memory",
                layer: "l5_control_plane",
                owner: "cynic-kernel",
                responsibility: "Persist runtime memory, observations, coordination, and proof history once lower proof producers exist.",
                mutation_scope: "Kernel storage/API state, not local operator heuristics",
                canonical_commands: vec!["cargo check -p cynic-kernel", "make run-kernel"],
                exit_evidence: vec!["kernel_health_observed", "runtime_proof_persisted"],
                anti_responsibilities: vec![
                    "replace host/node probes",
                    "compile on forbidden release hosts",
                    "make stale Markdown canonical",
                ],
            },
            DevelopmentSurface {
                id: "agent_exposure",
                layer: "l6_agent_interface",
                owner: "cynic-mcp",
                responsibility: "Expose only compiled and policy-backed organism capabilities to agents.",
                mutation_scope: "MCP/tool projections, not domain ownership",
                canonical_commands: vec![
                    "make check",
                    "cynic dev agent-exposure --json",
                    "cynic commands --json",
                ],
                exit_evidence: vec![
                    "agent_surface_projected",
                    "cynic.cli.agent_exposure_audit.v1",
                ],
                anti_responsibilities: vec![
                    "invent runtime state",
                    "duplicate CLI business logic",
                    "expose deleted or unbacked tools",
                    "invent a parallel validate stack",
                ],
            },
            DevelopmentSurface {
                id: "local_single_node_e2e",
                layer: "development_loop",
                owner: "Makefile",
                responsibility: "Run the single-machine development loop and report honest blocked runtime state.",
                mutation_scope: "Read-only by default; no host mutation",
                canonical_commands: vec!["make check", "make e2e"],
                exit_evidence: vec!["cynic.local_e2e.v1", "cynic.dev_check.v1"],
                anti_responsibilities: vec![
                    "prove fleet distribution",
                    "auto-apply OS remediation",
                    "mask blocked proofs",
                    "reintroduce fmt/build/clippy/test as a second agent gate",
                ],
            },
            DevelopmentSurface {
                id: "portable_lab_validation",
                layer: "development_loop",
                owner: "Makefile",
                responsibility: "Validate N-case portability against a synthetic lab registry distinct from the real fleet.",
                mutation_scope: "Read-only and artifact generation under temp dirs",
                canonical_commands: vec!["make runtime-validate", "make lab-smoke"],
                exit_evidence: vec!["cynic.runtime_validation.v1"],
                anti_responsibilities: vec![
                    "prove real fleet health",
                    "replace single-node E2E",
                    "depend on machine-local hostnames",
                ],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn development_surfaces_have_unique_ids_and_owners() {
        let manifest = development_surface_manifest();
        let mut ids = BTreeSet::new();
        for surface in &manifest.surfaces {
            assert!(ids.insert(surface.id), "duplicate surface {}", surface.id);
            assert!(!surface.owner.is_empty(), "surface owner required");
            assert!(
                !surface.canonical_commands.is_empty(),
                "surface commands required"
            );
        }
    }

    #[test]
    fn development_surfaces_keep_cli_as_adapter() {
        let manifest = development_surface_manifest();
        let cli = manifest
            .surfaces
            .iter()
            .find(|surface| surface.owner == "cynic-cli")
            .expect("cli surface exists");
        assert!(cli.anti_responsibilities.contains(&"own planner rules"));
    }
}
