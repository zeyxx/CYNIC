use serde::{Deserialize, Serialize};

pub const AGENT_MATRIX_SCHEMA: &str = "cynic.agent_matrix.v1";
<<<<<<< HEAD
pub const AGENT_MATRIX_GENERATED_AT: &str = "2026-08-11";
=======
pub const AGENT_MATRIX_GENERATED_AT: &str = "2026-07-22";
>>>>>>> github/main

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Observer,
    Planner,
    Executor,
    Prover,
    HybridCorrector,
    Supervisor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentReadiness {
    AgentifiableNow,
    FormalizeNowActivateLater,
    NotYetAgentifiable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActivationState {
    Inactive,
    Declared,
    PilotEnabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSurfaceEntry {
    pub surface: String,
    pub readiness: AgentReadiness,
    pub allowed_agent_kinds: Vec<AgentKind>,
    pub activation_state: AgentActivationState,
    pub reason: String,
}

impl AgentSurfaceEntry {
    pub fn can_activate_now(&self) -> bool {
        self.readiness == AgentReadiness::AgentifiableNow
            && self.activation_state == AgentActivationState::PilotEnabled
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMatrixReport {
    pub schema: &'static str,
    pub generated_at: &'static str,
    pub surfaces: Vec<AgentSurfaceEntry>,
}

pub fn build_agent_matrix_report() -> AgentMatrixReport {
    AgentMatrixReport {
        schema: AGENT_MATRIX_SCHEMA,
        generated_at: AGENT_MATRIX_GENERATED_AT,
        surfaces: vec![
            entry(
                "principal",
                AgentReadiness::AgentifiableNow,
                vec![AgentKind::Observer, AgentKind::Prover],
                AgentActivationState::PilotEnabled,
                "observed state, next boundary, next command, and proof are explicit",
            ),
            entry(
                "privilege",
                AgentReadiness::AgentifiableNow,
                vec![AgentKind::Observer, AgentKind::Prover],
                AgentActivationState::PilotEnabled,
                "privilege boundary is already projected canonically",
            ),
            entry(
                "packages",
                AgentReadiness::AgentifiableNow,
                vec![AgentKind::Observer, AgentKind::Prover],
                AgentActivationState::PilotEnabled,
                "package gap and remediation boundary are explicit",
            ),
            entry(
                "services",
                AgentReadiness::AgentifiableNow,
                vec![AgentKind::Observer, AgentKind::Prover],
                AgentActivationState::PilotEnabled,
                "service activation state and proof are explicit",
            ),
            entry(
                "host_bootstrap_evidence",
                AgentReadiness::AgentifiableNow,
                vec![AgentKind::Observer, AgentKind::Prover],
                AgentActivationState::PilotEnabled,
                "composite bootstrap evidence is canonically projected",
            ),
            entry(
                "bootstrap_chain",
                AgentReadiness::AgentifiableNow,
                vec![AgentKind::Observer],
                AgentActivationState::PilotEnabled,
                "stage sequence is stable enough for observation",
            ),
            entry(
                "runtime_context",
                AgentReadiness::AgentifiableNow,
                vec![AgentKind::Observer],
                AgentActivationState::PilotEnabled,
                "top-level runtime context now exposes canonical lifecycle fields",
            ),
            entry(
                "conformance",
                AgentReadiness::AgentifiableNow,
                vec![AgentKind::Prover],
                AgentActivationState::PilotEnabled,
                "qualification gates and proof semantics are explicit",
            ),
            entry(
                "runtime_network_preflight",
                AgentReadiness::AgentifiableNow,
                vec![AgentKind::Observer, AgentKind::Prover],
                AgentActivationState::PilotEnabled,
                "network state and next connection boundary are explicit",
            ),
            entry(
                "runtime_resolver_plan",
                AgentReadiness::AgentifiableNow,
                vec![AgentKind::Planner],
                AgentActivationState::PilotEnabled,
                "resolver planning is already tied to canonical network preflight",
            ),
            entry(
                "runtime_execution_context",
                AgentReadiness::AgentifiableNow,
                vec![AgentKind::Observer, AgentKind::Planner],
                AgentActivationState::PilotEnabled,
                "execution boundary and next command are explicit",
            ),
            entry(
                "storage_capabilities",
                AgentReadiness::AgentifiableNow,
                vec![AgentKind::Observer, AgentKind::Prover],
                AgentActivationState::PilotEnabled,
                "status and next boundary are explicit enough for bounded automation",
            ),
            entry(
                "host_activation_plan",
                AgentReadiness::FormalizeNowActivateLater,
                vec![AgentKind::Planner],
                AgentActivationState::Declared,
                "canonical projection exists but compatibility fields still dominate consumers",
            ),
            entry(
                "host_apply_report",
                AgentReadiness::FormalizeNowActivateLater,
                vec![AgentKind::Planner],
                AgentActivationState::Declared,
                "still partially coupled to legacy blocked-domain routing",
            ),
            entry(
                "host_apply_execution",
                AgentReadiness::FormalizeNowActivateLater,
                vec![AgentKind::Executor],
                AgentActivationState::Declared,
                "execution is sensitive and should remain bounded until rollback and proof paths are fully hardened",
            ),
            entry(
                "bootstrap_host_e2e",
                AgentReadiness::FormalizeNowActivateLater,
                vec![AgentKind::HybridCorrector],
                AgentActivationState::Declared,
                "depends on several already-converged layers but is still a final composite surface",
            ),
            entry(
                "bootstrap_constellation",
                AgentReadiness::FormalizeNowActivateLater,
                vec![AgentKind::Supervisor],
                AgentActivationState::Declared,
                "distributed visibility is partial and host-local observation is uneven",
            ),
            entry(
                "runtime_validation",
                AgentReadiness::FormalizeNowActivateLater,
                vec![AgentKind::Supervisor, AgentKind::Prover],
                AgentActivationState::Declared,
                "step orchestration is explicit, but actions still span heterogeneous surfaces",
            ),
            entry(
                "runtime_report",
                AgentReadiness::FormalizeNowActivateLater,
                vec![AgentKind::Supervisor],
                AgentActivationState::Declared,
                "report is canonicalized but remains an index rather than a corrective engine",
            ),
            entry(
<<<<<<< HEAD
                "build_runner",
                AgentReadiness::FormalizeNowActivateLater,
                vec![AgentKind::Executor, AgentKind::Prover],
                AgentActivationState::Declared,
                "each build must carry a scheduler-issued workload id, admitted authority material, a lease, and an isolated node-state target; execution remains inactive until scheduler recovery proof is complete",
            ),
            entry(
=======
>>>>>>> github/main
                "update_lifecycle",
                AgentReadiness::NotYetAgentifiable,
                Vec::new(),
                AgentActivationState::Inactive,
                "too specialized and not yet stabilized in the core runtime loop",
            ),
            entry(
                "framework_bootstrap",
                AgentReadiness::NotYetAgentifiable,
                Vec::new(),
                AgentActivationState::Inactive,
                "specialized surface, not part of the stable corrective backbone",
            ),
            entry(
                "lab",
                AgentReadiness::NotYetAgentifiable,
                Vec::new(),
                AgentActivationState::Inactive,
                "must not blur proof harnesses with runtime authority",
            ),
            entry(
                "inference_surface",
                AgentReadiness::NotYetAgentifiable,
                Vec::new(),
                AgentActivationState::Inactive,
                "too interdependent across layers for safe autonomy today",
            ),
            entry(
                "recursive_multi_agent",
                AgentReadiness::FormalizeNowActivateLater,
                vec![AgentKind::Supervisor, AgentKind::Planner, AgentKind::Prover],
                AgentActivationState::Declared,
                "node-local recursion envelopes are formalized, but activation waits for aggregate budget accounting, cancellation, lease fencing, and durable recovery proof",
            ),
        ],
    }
}

fn entry(
    surface: &str,
    readiness: AgentReadiness,
    allowed_agent_kinds: Vec<AgentKind>,
    activation_state: AgentActivationState,
    reason: &str,
) -> AgentSurfaceEntry {
    AgentSurfaceEntry {
        surface: surface.to_string(),
        readiness,
        allowed_agent_kinds,
        activation_state,
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pilot_enabled_entries_can_activate_only_when_ready_now() {
        let report = build_agent_matrix_report();
        let principal = report
            .surfaces
            .iter()
            .find(|entry| entry.surface == "principal")
            .expect("principal entry");
        let runtime_validation = report
            .surfaces
            .iter()
            .find(|entry| entry.surface == "runtime_validation")
            .expect("runtime_validation entry");

        assert!(principal.can_activate_now());
        assert!(!runtime_validation.can_activate_now());
    }

    #[test]
    fn recursive_agents_are_declared_but_not_activated() {
        let report = build_agent_matrix_report();
        let recursion = report
            .surfaces
            .iter()
            .find(|entry| entry.surface == "recursive_multi_agent")
            .expect("recursive agent entry");

        assert_eq!(
            recursion.readiness,
            AgentReadiness::FormalizeNowActivateLater
        );
        assert_eq!(recursion.activation_state, AgentActivationState::Declared);
        assert!(!recursion.can_activate_now());
    }
<<<<<<< HEAD

    #[test]
    fn build_runner_is_declared_but_requires_scheduler_proof() {
        let report = build_agent_matrix_report();
        let build_runner = report
            .surfaces
            .iter()
            .find(|entry| entry.surface == "build_runner")
            .expect("build runner entry");

        assert_eq!(
            build_runner.readiness,
            AgentReadiness::FormalizeNowActivateLater
        );
        assert_eq!(
            build_runner.activation_state,
            AgentActivationState::Declared
        );
        assert!(!build_runner.can_activate_now());
    }
=======
>>>>>>> github/main
}
