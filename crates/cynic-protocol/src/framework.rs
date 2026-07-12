use serde::Serialize;

pub const FRAMEWORK_SPEC_SCHEMA: &str = "cynic.framework_spec.v1";

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkSpec {
    pub schema: &'static str,
    pub status: &'static str,
    pub identity: FrameworkIdentity,
    pub operating_model: Vec<FrameworkPrinciple>,
    pub canonical_layers: Vec<FrameworkLayer>,
    pub primitive_families: Vec<FrameworkPrimitiveFamily>,
    pub target_use_cases: Vec<FrameworkUseCase>,
    pub agnosticity_rules: Vec<FrameworkRule>,
    pub modularity_rules: Vec<FrameworkRule>,
    pub non_goals: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkIdentity {
    pub name: &'static str,
    pub purpose: &'static str,
    pub short_definition: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkPrinciple {
    pub id: &'static str,
    pub rule: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkLayer {
    pub id: &'static str,
    pub responsibility: &'static str,
    pub owned_by: Vec<&'static str>,
    pub evidence: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkPrimitiveFamily {
    pub id: &'static str,
    pub role: &'static str,
    pub examples: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkUseCase {
    pub id: &'static str,
    pub description: &'static str,
    pub required_primitives: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkRule {
    pub id: &'static str,
    pub rule: &'static str,
}

pub fn framework_spec() -> FrameworkSpec {
    FrameworkSpec {
        schema: FRAMEWORK_SPEC_SCHEMA,
        status: "normative",
        identity: FrameworkIdentity {
            name: "CYNIC",
            purpose: "Build a sovereign, agentic, OS-like runtime framework from portable low-level primitives.",
            short_definition: "CYNIC is a Rust-first organism runtime: registries declare desired state, host/node/kernel crates observe real state, protocol crates score evidence, and adapters expose bounded commands to humans and agents.",
        },
        operating_model: vec![
            FrameworkPrinciple {
                id: "bottom_up_runtime_truth",
                rule: "Runtime claims start from host, service, network, storage, and kernel read-back; prose cannot promote a layer to healthy.",
            },
            FrameworkPrinciple {
                id: "desired_observed_reconcile",
                rule: "Every orchestration loop must compare desired state, observed state, and the next safe action.",
            },
            FrameworkPrinciple {
                id: "agent_as_bounded_workload",
                rule: "An agent session is a workload with declared surface, evidence, and validation; it is not an unlimited authority.",
            },
            FrameworkPrinciple {
                id: "kubernetes_like_not_kubernetes_bound",
                rule: "CYNIC may use Kubernetes-like semantics, but the domain model must also run on one host, systemd, VMs, LXC, or another substrate.",
            },
        ],
        canonical_layers: vec![
            FrameworkLayer {
                id: "l0_protocol",
                responsibility: "Schemas, pure planning rules, proof envelopes, KPI scoring, and framework contracts.",
                owned_by: vec!["cynic-protocol"],
                evidence: vec![
                    "cynic.framework_spec.v1",
                    "cynic.outcome_spec.v1",
                    "cynic.kpi_score.v1",
                ],
            },
            FrameworkLayer {
                id: "l1_host",
                responsibility: "Local OS substrate observation and bounded preparation.",
                owned_by: vec!["cynic-host"],
                evidence: vec!["host_identity_observed", "service_supervisor_observation"],
            },
            FrameworkLayer {
                id: "l2_node_network",
                responsibility: "Node, mesh, DNS, provider fallback, and network runtime proof.",
                owned_by: vec!["cynic-node"],
                evidence: vec!["network_runtime_observed"],
            },
            FrameworkLayer {
                id: "l3_kernel_memory",
                responsibility: "Persist observations, coordination, runtime proof history, and task memory.",
                owned_by: vec!["cynic-kernel"],
                evidence: vec!["kernel_health_observed", "runtime_proof_persisted"],
            },
            FrameworkLayer {
                id: "l4_agent_exposure",
                responsibility: "Expose backed capabilities to agents without duplicating domain authority.",
                owned_by: vec!["cynic-mcp", "cynic-cli"],
                evidence: vec!["agent_surface_projected", "cynic.cli.surface_manifest.v1"],
            },
        ],
        primitive_families: vec![
            FrameworkPrimitiveFamily {
                id: "registry",
                role: "Declare desired placement, roles, services, and environment-specific topology.",
                examples: vec![
                    "infra/registry.json",
                    "infra/registries/single-node.json",
                    "infra/registries/lab.json",
                ],
            },
            FrameworkPrimitiveFamily {
                id: "proof_bundle",
                role: "Carry observed runtime evidence between layers without relying on chat or stale files.",
                examples: vec![
                    "cynic.runtime_proof_bundle.v1",
                    "host_identity_observed",
                    "network_runtime_observed",
                ],
            },
            FrameworkPrimitiveFamily {
                id: "planner",
                role: "Turn desired state and proof bundles into safe read-only, dry-run, and apply boundaries.",
                examples: vec![
                    "cynic.runtime_execution_plan.v1",
                    "can_apply=false until executor exists",
                ],
            },
            FrameworkPrimitiveFamily {
                id: "adapter",
                role: "Bind portable ports to concrete substrates without leaking provider logic into domain rules.",
                examples: vec![
                    "systemd",
                    "local shell",
                    "MCP",
                    "future Kubernetes adapter",
                    "future VM/LXC adapter",
                ],
            },
            FrameworkPrimitiveFamily {
                id: "agentic_surface",
                role: "Give human and LLM agents bounded, inspectable, validation-backed commands.",
                examples: vec![
                    "cynic dev surfaces --json",
                    "cynic dev outcomes --json",
                    "cynic dev kpi-score --json",
                ],
            },
        ],
        target_use_cases: vec![
            FrameworkUseCase {
                id: "single_machine_bootstrap",
                description: "Run the organism on one machine for development and proof iteration.",
                required_primitives: vec!["registry", "proof_bundle", "planner", "adapter"],
            },
            FrameworkUseCase {
                id: "portable_lab_fleet",
                description: "Validate N-case topology without binding the framework to one hostname, DNS zone, or provider.",
                required_primitives: vec!["registry", "planner", "adapter"],
            },
            FrameworkUseCase {
                id: "production_service_supervision",
                description: "Measure uptime, restart budgets, health latency, proof freshness, and registry/runtime convergence.",
                required_primitives: vec!["proof_bundle", "planner", "adapter"],
            },
            FrameworkUseCase {
                id: "agentic_development_runtime",
                description: "Let agents develop safely through explicit surfaces, claims, proofs, validation commands, and next-step scoring.",
                required_primitives: vec!["agentic_surface", "proof_bundle", "planner"],
            },
            FrameworkUseCase {
                id: "kubernetes_like_orchestration",
                description: "Provide desired-state reconciliation semantics that can later target Kubernetes, VMs, LXC, systemd, or custom hosts.",
                required_primitives: vec!["registry", "planner", "adapter"],
            },
        ],
        agnosticity_rules: vec![
            FrameworkRule {
                id: "no_provider_in_domain",
                rule: "Domain types must not require a specific cloud, hypervisor, LLM, browser, DNS zone, or host identity.",
            },
            FrameworkRule {
                id: "n_case_by_registry",
                rule: "A new host, fleet, provider, or deployment model must be represented as registry data or adapter config before code branching.",
            },
            FrameworkRule {
                id: "substrate_behind_ports",
                rule: "Systemd, Kubernetes, LXC, SSH, REST, MCP, and shell execution are adapters behind portable ports.",
            },
        ],
        modularity_rules: vec![
            FrameworkRule {
                id: "protocol_is_pure",
                rule: "cynic-protocol owns schemas and pure rules; it must not read the host or mutate runtime state.",
            },
            FrameworkRule {
                id: "cli_is_adapter",
                rule: "cynic-cli wires inputs and outputs; it must not own orchestration semantics.",
            },
            FrameworkRule {
                id: "proof_producers_are_low_layer",
                rule: "Host and node crates produce runtime proofs; higher layers consume and persist them.",
            },
            FrameworkRule {
                id: "one_boundary_one_reason",
                rule: "Add a crate, module, adapter, or contract only when an existing boundary cannot absorb the responsibility cleanly.",
            },
        ],
        non_goals: vec![
            "be a UX portal first",
            "recreate deleted Python organs as durable authorities",
            "require Kubernetes before single-node execution works",
            "treat LLM session memory as canonical state",
            "centralize all responsibilities in one orchestrator crate",
        ],
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn framework_spec_has_unique_layers_and_use_cases() {
        let spec = framework_spec();
        let mut layers = BTreeSet::new();
        let mut use_cases = BTreeSet::new();
        for layer in &spec.canonical_layers {
            assert!(layers.insert(layer.id), "duplicate layer {}", layer.id);
            assert!(!layer.owned_by.is_empty(), "layer owner required");
            assert!(!layer.evidence.is_empty(), "layer evidence required");
        }
        for use_case in &spec.target_use_cases {
            assert!(
                use_cases.insert(use_case.id),
                "duplicate use case {}",
                use_case.id
            );
            assert!(
                !use_case.required_primitives.is_empty(),
                "use case primitives required"
            );
        }
    }

    #[test]
    fn framework_spec_preserves_substrate_agnosticity() {
        let spec = framework_spec();
        assert!(
            spec.agnosticity_rules
                .iter()
                .any(|rule| rule.id == "substrate_behind_ports")
        );
        assert!(
            spec.non_goals
                .contains(&"require Kubernetes before single-node execution works")
        );
    }
}
