use serde::{Deserialize, Serialize};

pub const KYON_SCENARIO_SCHEMA: &str = "cynic.askesis.kyon_scenario.v1";
pub const KYON_EXPERIMENT_PLAN_SCHEMA: &str = "cynic.askesis.kyon_experiment_plan.v1";
pub const KYON_EXPERIMENT_ASSESSMENT_SCHEMA: &str = "cynic.askesis.kyon_experiment_assessment.v1";
pub const KYON_PREFLIGHT_OBSERVATION_SCHEMA: &str = "cynic.askesis.kyon_preflight_observation.v1";
pub const KYON_EXECUTION_DECISION_SCHEMA: &str = "cynic.askesis.kyon_execution_decision.v1";
pub const KYON_ACTION_COMMAND_SCHEMA: &str = "cynic.askesis.kyon_action_command.v1";
pub const KYON_READ_BACK_REQUEST_SCHEMA: &str = "cynic.askesis.kyon_read_back_request.v1";
pub const KYON_EXPERIMENT_PROOF_SCHEMA: &str = "cynic.askesis.kyon_experiment_proof.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityKind {
    ProtectedPerson,
    Child,
    TrustedPerson,
    Guardian,
    Professional,
    Institution,
    Adversary,
    AiMember,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub authority: AuthorityKind,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanIntent {
    pub owner_actor_id: String,
    pub statement: String,
    pub revision_requires_explicit_confirmation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicStatus {
    Observed,
    SelfReported,
    Scientific,
    Traditional,
    ModelInferred,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataExposure {
    LocalRaw,
    LocalDerived,
    Commitment,
    ZeroKnowledgeAttestation,
    ConsentedEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationSpec {
    pub id: String,
    pub kind: String,
    pub epistemic_status: EpistemicStatus,
    pub exposure: DataExposure,
    pub retention: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioHypothesis {
    pub id: String,
    pub level: String,
    pub statement: String,
    pub disconfirming_observation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectiveExperiment {
    pub id: String,
    pub description: String,
    pub mutates_device_state: bool,
    pub reversible: bool,
    pub requires_confirmation: bool,
    pub stop_condition: String,
    pub forbidden_outcomes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadBackSpec {
    pub observations: Vec<String>,
    pub sovereignty_outcomes: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KyonScenario {
    pub schema: String,
    pub id: String,
    pub title: String,
    pub protected_actor_id: String,
    pub actors: Vec<Actor>,
    pub harm_to_prevent: Vec<String>,
    pub intent: HumanIntent,
    pub observations: Vec<ObservationSpec>,
    pub hypotheses: Vec<ScenarioHypothesis>,
    pub experiments: Vec<ProtectiveExperiment>,
    pub read_back: ReadBackSpec,
    pub federation_enabled_by_default: bool,
    pub raw_data_federation_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedExperiment {
    pub schema: String,
    pub scenario_id: String,
    pub experiment_id: String,
    pub protected_actor_id: String,
    pub desired_state: String,
    pub preflight: Vec<String>,
    pub baseline_observations: Vec<String>,
    pub bounded_action: String,
    pub mutates_device_state: bool,
    pub requires_explicit_confirmation: bool,
    pub execution_authorized: bool,
    pub stop_condition: String,
    pub read_back_observations: Vec<String>,
    pub sovereignty_outcomes: Vec<String>,
    pub forbidden_outcomes: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentOutcomeEvidence {
    pub scenario_id: String,
    pub experiment_id: String,
    pub confirmation_observed: bool,
    pub stop_condition_observed: bool,
    pub action_continued_after_stop: bool,
    pub read_back_observations: Vec<String>,
    pub sovereignty_outcomes: Vec<String>,
    pub forbidden_outcomes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentOutcomeAssessment {
    pub schema: String,
    pub scenario_id: String,
    pub experiment_id: String,
    pub status: String,
    pub sovereignty_status: String,
    pub causal_conclusion: String,
    pub missing_observations: Vec<String>,
    pub missing_sovereignty_outcomes: Vec<String>,
    pub violations: Vec<String>,
    pub residual_unknowns: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightObservation {
    pub schema: String,
    pub scenario_id: String,
    pub experiment_id: String,
    pub protected_actor_id: String,
    pub protected_actor_identity_observed: bool,
    pub policy_version: Option<String>,
    pub baseline_observations: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplicitConfirmation {
    pub scenario_id: String,
    pub experiment_id: String,
    pub actor_id: String,
    pub policy_version: String,
    pub confirmation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedActionCommand {
    pub schema: String,
    pub scenario_id: String,
    pub experiment_id: String,
    pub protected_actor_id: String,
    pub action: String,
    pub mutates_device_state: bool,
    pub expected_policy_version: String,
    pub confirmation_ref: String,
    pub stop_condition: String,
    pub forbidden_outcomes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionDecision {
    pub schema: String,
    pub scenario_id: String,
    pub experiment_id: String,
    pub status: String,
    pub blockers: Vec<String>,
    pub command: Option<BoundedActionCommand>,
    pub evidence_refs: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionReceipt {
    pub scenario_id: String,
    pub experiment_id: String,
    pub status: String,
    pub action_evidence_refs: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadBackRequest {
    pub schema: String,
    pub scenario_id: String,
    pub experiment_id: String,
    pub protected_actor_id: String,
    pub expected_policy_version: String,
    pub confirmation_ref: String,
    pub stop_condition: String,
    pub required_observations: Vec<String>,
    pub required_sovereignty_outcomes: Vec<String>,
    pub forbidden_outcomes: Vec<String>,
    pub action_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentProofBundle {
    pub schema: String,
    pub scenario_id: String,
    pub experiment_id: String,
    pub status: String,
    pub policy_version: Option<String>,
    pub confirmation_ref: Option<String>,
    pub preflight_evidence_refs: Vec<String>,
    pub action_evidence_refs: Vec<String>,
    pub read_back_evidence_refs: Vec<String>,
    pub assessment: ExperimentOutcomeAssessment,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
}
