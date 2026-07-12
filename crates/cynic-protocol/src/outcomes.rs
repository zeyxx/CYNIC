use std::collections::BTreeSet;

use serde::Serialize;

pub const OUTCOME_SPEC_SCHEMA: &str = "cynic.outcome_spec.v1";
pub const KPI_SCORE_SCHEMA: &str = "cynic.kpi_score.v1";

#[derive(Debug, Clone, Serialize)]
pub struct OutcomeSpec {
    pub schema: &'static str,
    pub rule: &'static str,
    pub outcomes: Vec<Outcome>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    pub id: &'static str,
    pub layer: &'static str,
    pub objective: &'static str,
    pub success_kpis: Vec<SuccessKpi>,
    pub required_evidence: Vec<&'static str>,
    pub validation_commands: Vec<&'static str>,
    pub non_goals: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuccessKpi {
    pub id: &'static str,
    pub target: &'static str,
    pub measurement: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct KpiScoreReport {
    pub schema: &'static str,
    pub scope: &'static str,
    pub rule: &'static str,
    pub outcome_count: usize,
    pub ready_count: usize,
    pub partial_count: usize,
    pub blocked_count: usize,
    pub outcomes: Vec<OutcomeScore>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutcomeScore {
    pub id: &'static str,
    pub layer: &'static str,
    pub status: &'static str,
    pub score_percent: u8,
    pub kpi_count: usize,
    pub evidence: CoverageScore,
    pub validation_commands: CoverageScore,
    pub missing_evidence: Vec<&'static str>,
    pub missing_validation_commands: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageScore {
    pub required: usize,
    pub available: usize,
}

pub fn outcome_spec() -> OutcomeSpec {
    OutcomeSpec {
        schema: OUTCOME_SPEC_SCHEMA,
        rule: "A target outcome is valid only when it has measurable KPIs, runtime evidence, and executable validation commands.",
        outcomes: vec![
            Outcome {
                id: "local_single_node_life",
                layer: "single_node_runtime",
                objective: "Run the organism on one machine as a truthful minimal runtime without requiring fleet distribution.",
                success_kpis: vec![
                    kpi(
                        "e2e_command_exists",
                        "make e2e exits 0",
                        "local E2E emits cynic.local_e2e.v1",
                    ),
                    kpi(
                        "blocked_is_evidence",
                        "blocked proofs are represented as JSON, not shell failure",
                        "host_proof_status/network_proof_status fields exist",
                    ),
                    kpi(
                        "first_blocker_identified",
                        "first blocked runtime layer is explicit",
                        "first_blocked_layer.id is present when plan_status is planned_with_blockers",
                    ),
                ],
                required_evidence: vec!["cynic.local_e2e.v1"],
                validation_commands: vec!["make e2e"],
                non_goals: vec![
                    "prove multi-node distribution",
                    "auto-remediate host OS",
                    "mark blocked runtime as healthy",
                ],
            },
            Outcome {
                id: "portable_n_case_registry",
                layer: "portability",
                objective: "Prove runtime projections are registry-driven and not coupled to one hostname, CIDR, or DNS zone.",
                success_kpis: vec![
                    kpi(
                        "lab_registry_executes",
                        "lab registry produces runtime suite and plan",
                        "make runtime-validate reports lab_execution_plan status ok",
                    ),
                    kpi(
                        "dns_zone_not_hardcoded",
                        "lab DNS bundle contains labmesh.example and excludes synapse.cynic",
                        "make lab-smoke exits 0",
                    ),
                    kpi(
                        "proofs_unlock_layers",
                        "typed proof bundle opens L2 and L3 in the planner",
                        "runtime-validate proof consumption steps are ok",
                    ),
                ],
                required_evidence: vec![
                    "cynic.runtime_validation.v1",
                    "cynic.runtime_execution_plan.v1",
                ],
                validation_commands: vec!["make runtime-validate", "make lab-smoke"],
                non_goals: vec![
                    "prove real fleet health",
                    "depend on current machine hostname",
                    "use Markdown as proof storage",
                ],
            },
            Outcome {
                id: "hexagonal_crate_boundaries",
                layer: "architecture",
                objective: "Keep protocol, host, node, CLI, kernel, and MCP responsibilities separated by compilation boundaries.",
                success_kpis: vec![
                    kpi(
                        "protocol_owns_contracts",
                        "runtime planner and development/outcome specs live in cynic-protocol",
                        "cargo test -p cynic-protocol exits 0",
                    ),
                    kpi(
                        "cli_is_adapter",
                        "cynic-cli reads inputs and serializes outputs but does not own planner rules",
                        "development surface manifest marks own planner rules as CLI anti-responsibility",
                    ),
                    kpi(
                        "proof_producers_are_lower_layers",
                        "host and node crates emit typed runtime proof bundles",
                        "make local-host-proof and make local-network-proof emit cynic.runtime_proof_bundle.v1",
                    ),
                ],
                required_evidence: vec![
                    "cynic.development_surface_manifest.v1",
                    "cynic.runtime_proof_bundle.v1",
                ],
                validation_commands: vec![
                    "cynic dev surfaces --json",
                    "cargo test -p cynic-protocol",
                    "make local-host-proof",
                    "make local-network-proof",
                ],
                non_goals: vec![
                    "centralize all behavior in protocol",
                    "let CLI become business layer",
                    "duplicate contracts across agents",
                ],
            },
            Outcome {
                id: "runtime_proof_chain",
                layer: "runtime_evidence",
                objective: "Build a bottom-up proof chain where each layer opens only after prior runtime evidence exists.",
                success_kpis: vec![
                    kpi(
                        "host_opens_network",
                        "host_identity_observed opens l2_network_projection",
                        "protocol planner test covers host proof transition",
                    ),
                    kpi(
                        "network_opens_storage",
                        "network_runtime_observed opens l3_storage_projection",
                        "protocol planner test covers network proof transition",
                    ),
                    kpi(
                        "apply_stays_closed",
                        "apply commands are represented but can_apply remains false until a mutating executor exists",
                        "runtime plan JSON contains can_apply=false",
                    ),
                ],
                required_evidence: vec!["cynic.runtime_proof_bundle.v1"],
                validation_commands: vec![
                    "cargo test -p cynic-protocol runtime_planner_opens_layers_from_observed_gates",
                    "make runtime-validate",
                ],
                non_goals: vec![
                    "skip lower gates",
                    "execute apply during planning",
                    "treat synthetic lab proof as real fleet proof",
                ],
            },
            Outcome {
                id: "production_runtime_stability",
                layer: "production_operations",
                objective: "Measure whether deployed organism services stay alive, stable, responsive, and aligned with registry intent over time.",
                success_kpis: vec![
                    kpi(
                        "service_uptime_slo",
                        "critical services meet their declared uptime SLO over the measured window",
                        "service supervisor observations expose uptime ratio per service",
                    ),
                    kpi(
                        "restart_budget",
                        "unexpected restarts stay below the declared error budget",
                        "systemd or supervisor evidence exposes restart_count per service",
                    ),
                    kpi(
                        "health_latency_budget",
                        "health endpoints respond below the declared latency threshold",
                        "runtime health probes expose p50/p95 latency per service",
                    ),
                    kpi(
                        "fresh_runtime_proofs",
                        "runtime proof bundles are recent enough to be trusted",
                        "proof evidence includes observed_at and max_age policy can be evaluated",
                    ),
                    kpi(
                        "registry_runtime_convergence",
                        "running services match the registry projection for the selected environment",
                        "lifecycle/runtime validation compares observed units to infra registry intent",
                    ),
                ],
                required_evidence: vec![
                    "cynic.runtime_proof_bundle.v1",
                    "cynic.runtime_execution_plan.v1",
                    "service_supervisor_observation",
                ],
                validation_commands: vec![
                    "cynic runtime plan --json --proof <path>",
                    "make runtime-validate",
                    "make e2e",
                ],
                non_goals: vec![
                    "declare prod healthy from a local build",
                    "hide flapping services behind successful deploy commands",
                    "treat one health check as uptime history",
                ],
            },
            Outcome {
                id: "sovereign_os_agentic_orchestration",
                layer: "sovereign_os_control_plane",
                objective: "Converge toward a sovereign OS-like control plane where agents, services, hosts, and desired runtime state are reconciled through portable primitives.",
                success_kpis: vec![
                    kpi(
                        "declarative_desired_state",
                        "runtime intent is declared in registries and protocol contracts, not embedded in ad-hoc commands",
                        "runtime plan is derived from registry inputs and typed proof bundles",
                    ),
                    kpi(
                        "reconciliation_loop",
                        "the system can compare desired state, observed state, and next safe action",
                        "runtime execution plan exposes blocked/open layers and can_apply state",
                    ),
                    kpi(
                        "agent_workload_boundary",
                        "agent sessions operate as bounded workloads with explicit surfaces and evidence",
                        "development surfaces and outcomes expose ownership, responsibilities, anti_responsibilities, and validation commands",
                    ),
                    kpi(
                        "host_portability",
                        "host execution primitives are not coupled to one machine, provider, hypervisor, or DNS zone",
                        "single-node and lab registries both produce plans without code changes",
                    ),
                    kpi(
                        "kubernetes_like_without_kubernetes_lockin",
                        "CYNIC has Kubernetes-like semantics for desired state, observation, scheduling boundaries, and reconciliation without requiring Kubernetes as the substrate",
                        "orchestration semantics are represented in cynic-protocol before any adapter-specific backend",
                    ),
                ],
                required_evidence: vec![
                    "cynic.runtime_execution_plan.v1",
                    "cynic.runtime_proof_bundle.v1",
                    "cynic.development_surface_manifest.v1",
                    "cynic.outcome_spec.v1",
                ],
                validation_commands: vec![
                    "cynic runtime validate --json",
                    "cynic runtime plan --json --proof <path>",
                    "cynic dev surfaces --json",
                    "cynic dev outcomes --json",
                ],
                non_goals: vec![
                    "make Kubernetes the only valid runtime substrate",
                    "turn agents into unbounded repo-mutating daemons",
                    "hide OS reality behind provider-specific abstractions",
                    "centralize all reconciliation logic in the CLI",
                ],
            },
            Outcome {
                id: "agentic_session_convergence",
                layer: "agentic_development_loop",
                objective: "Make each agentic development session converge on one bounded increment with explicit evidence and no hidden memory.",
                success_kpis: vec![
                    kpi(
                        "scope_is_explicit",
                        "session work is tied to a declared development surface or runtime layer",
                        "cynic dev surfaces --json exposes the owning surface before implementation",
                    ),
                    kpi(
                        "result_is_interrogable",
                        "session output can be inspected through a compiled command",
                        "cynic dev outcomes --json emits cynic.outcome_spec.v1",
                    ),
                    kpi(
                        "next_step_is_deterministic",
                        "the next increment follows from failed or blocked validation, not narrative backlog",
                        "validation_commands and required_evidence are non-empty for every outcome",
                    ),
                ],
                required_evidence: vec![
                    "cynic.development_surface_manifest.v1",
                    "cynic.outcome_spec.v1",
                ],
                validation_commands: vec![
                    "cynic dev surfaces --json",
                    "cynic dev outcomes --json",
                    "cargo test -p cynic-protocol outcomes_are_measurable_and_unique",
                ],
                non_goals: vec![
                    "store session truth in chat history",
                    "create a new planning document for every hesitation",
                    "let agents infer ownership from stale prose",
                ],
            },
            Outcome {
                id: "agentic_fragmentation_control",
                layer: "agentic_information_architecture",
                objective: "Reduce LLM chaos by keeping canonical low-level rules in typed protocol surfaces instead of duplicated narrative contracts.",
                success_kpis: vec![
                    kpi(
                        "few_canonical_surfaces",
                        "development surfaces remain enumerable and owned",
                        "cynic dev surfaces --json returns stable ids, owners, responsibilities, and anti_responsibilities",
                    ),
                    kpi(
                        "outcomes_reference_evidence",
                        "every target outcome names runtime evidence schemas",
                        "cynic dev outcomes --json has required_evidence for every outcome",
                    ),
                    kpi(
                        "contracts_compile",
                        "agent-facing specs are Rust-compiled protocol data",
                        "cargo test -p cynic-protocol exits 0",
                    ),
                ],
                required_evidence: vec![
                    "cynic.development_surface_manifest.v1",
                    "cynic.outcome_spec.v1",
                ],
                validation_commands: vec![
                    "cynic dev surfaces --json",
                    "cynic dev outcomes --json",
                    "cargo test -p cynic-protocol",
                ],
                non_goals: vec![
                    "centralize implementation responsibility in cynic-protocol",
                    "recreate the deleted axioms bureaucracy",
                    "optimize for exhaustive prose before executable feedback",
                ],
            },
        ],
    }
}

fn kpi(id: &'static str, target: &'static str, measurement: &'static str) -> SuccessKpi {
    SuccessKpi {
        id,
        target,
        measurement,
    }
}

pub fn score_outcomes(
    spec: &OutcomeSpec,
    available_evidence: &[&str],
    available_commands: &[&str],
) -> KpiScoreReport {
    let evidence_index = available_evidence.iter().copied().collect::<BTreeSet<_>>();
    let mut outcomes = Vec::new();
    let mut ready_count = 0;
    let mut partial_count = 0;
    let mut blocked_count = 0;

    for outcome in &spec.outcomes {
        let missing_evidence = outcome
            .required_evidence
            .iter()
            .copied()
            .filter(|evidence| !evidence_index.contains(evidence))
            .collect::<Vec<_>>();
        let missing_validation_commands = outcome
            .validation_commands
            .iter()
            .copied()
            .filter(|command| {
                !available_commands
                    .iter()
                    .any(|available| command_matches(command, available))
            })
            .collect::<Vec<_>>();
        let evidence_available = outcome.required_evidence.len() - missing_evidence.len();
        let commands_available =
            outcome.validation_commands.len() - missing_validation_commands.len();
        let required = outcome.required_evidence.len() + outcome.validation_commands.len();
        let available = evidence_available + commands_available;
        let score_percent = if required == 0 {
            0
        } else {
            ((available * 100) / required) as u8
        };
        let status = if available == required {
            ready_count += 1;
            "ready_for_validation"
        } else if available == 0 {
            blocked_count += 1;
            "blocked"
        } else {
            partial_count += 1;
            "partial"
        };
        outcomes.push(OutcomeScore {
            id: outcome.id,
            layer: outcome.layer,
            status,
            score_percent,
            kpi_count: outcome.success_kpis.len(),
            evidence: CoverageScore {
                required: outcome.required_evidence.len(),
                available: evidence_available,
            },
            validation_commands: CoverageScore {
                required: outcome.validation_commands.len(),
                available: commands_available,
            },
            missing_evidence,
            missing_validation_commands,
        });
    }

    KpiScoreReport {
        schema: KPI_SCORE_SCHEMA,
        scope: "contract_and_surface_readiness",
        rule: "This score proves whether outcome KPIs have declared evidence and executable surfaces; it does not replace runtime uptime history.",
        outcome_count: outcomes.len(),
        ready_count,
        partial_count,
        blocked_count,
        outcomes,
    }
}

fn command_matches(required: &str, available: &str) -> bool {
    let required_tokens = normalized_command_tokens(required);
    let available_tokens = normalized_command_tokens(available);
    required_tokens
        .iter()
        .all(|required| available_tokens.contains(required))
        || available_tokens
            .iter()
            .all(|available| required_tokens.contains(available))
}

fn normalized_command_tokens(command: &str) -> BTreeSet<String> {
    command
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|c| matches!(c, '[' | ']' | '<' | '>' | '|' | ','))
                .to_string()
        })
        .filter(|token| {
            !token.is_empty()
                && !token.starts_with('<')
                && !token.ends_with('>')
                && *token != "path"
                && *token != "name"
                && *token != "n"
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn outcomes_are_measurable_and_unique() {
        let spec = outcome_spec();
        let mut ids = BTreeSet::new();
        for outcome in &spec.outcomes {
            assert!(ids.insert(outcome.id), "duplicate outcome {}", outcome.id);
            assert!(!outcome.success_kpis.is_empty(), "kpis required");
            assert!(!outcome.required_evidence.is_empty(), "evidence required");
            assert!(
                !outcome.validation_commands.is_empty(),
                "validation commands required"
            );
        }
    }

    #[test]
    fn outcomes_cover_local_portability_architecture_and_proof_chain() {
        let ids = outcome_spec()
            .outcomes
            .into_iter()
            .map(|outcome| outcome.id)
            .collect::<BTreeSet<_>>();
        assert!(ids.contains("local_single_node_life"));
        assert!(ids.contains("portable_n_case_registry"));
        assert!(ids.contains("hexagonal_crate_boundaries"));
        assert!(ids.contains("runtime_proof_chain"));
        assert!(ids.contains("production_runtime_stability"));
        assert!(ids.contains("sovereign_os_agentic_orchestration"));
        assert!(ids.contains("agentic_session_convergence"));
        assert!(ids.contains("agentic_fragmentation_control"));
    }

    #[test]
    fn kpi_score_reports_missing_runtime_evidence_without_pretending_prod_is_proven() {
        let spec = outcome_spec();
        let report = score_outcomes(
            &spec,
            &[
                "cynic.outcome_spec.v1",
                "cynic.development_surface_manifest.v1",
                "cynic.runtime_execution_plan.v1",
                "cynic.runtime_proof_bundle.v1",
            ],
            &[
                "cynic dev outcomes --json",
                "cynic dev surfaces --json",
                "cynic runtime plan --json [--proof <path>]",
            ],
        );
        let prod = report
            .outcomes
            .iter()
            .find(|outcome| outcome.id == "production_runtime_stability")
            .expect("prod stability outcome exists");
        assert_ne!(prod.status, "ready_for_validation");
        assert!(
            prod.missing_evidence
                .contains(&"service_supervisor_observation")
        );
    }

    #[test]
    fn command_matching_accepts_manifest_commands_with_optional_arguments() {
        assert!(command_matches(
            "cynic runtime plan --json --proof <path>",
            "cynic runtime plan --json [--registry <path>] [--profile <name>] [--proof <path>]",
        ));
        assert!(command_matches(
            "cargo test -p cynic-protocol runtime_planner_opens_layers_from_observed_gates",
            "cargo test -p cynic-protocol",
        ));
    }
}
