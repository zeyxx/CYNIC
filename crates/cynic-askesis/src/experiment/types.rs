use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomainSpec {
    pub id: String,
    pub task_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DimensionSpec {
    pub id: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScaleSpec {
    pub id: String,
    pub sample_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExperimentSpec {
    pub schema: String,
    pub id: String,
    pub domains: Vec<DomainSpec>,
    pub dimensions: Vec<DimensionSpec>,
    pub scales: Vec<ScaleSpec>,
    pub repetitions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrialPlan {
    pub schema: &'static str,
    pub experiment_id: String,
    pub trial_id: String,
    pub domain: DomainSpec,
    pub coordinates: Vec<(String, String)>,
    pub scale: ScaleSpec,
    pub repetition: u32,
    pub mutates: bool,
    pub status: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    Unknown,
    Running,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceScenario {
    pub desired_service: String,
    pub desired_state: ServiceState,
    pub initial_state: ServiceState,
    pub restart_succeeds: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReconciliationAction {
    ObserveService { service: String },
    RestartService { service: String, consent: bool },
    ReadBackService { service: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceObservation {
    pub service: String,
    pub state: ServiceState,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReconciliationStepProof {
    pub step: u32,
    pub action: ReconciliationAction,
    pub mutates: bool,
    pub observation: Option<ServiceObservation>,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReconciliationProof {
    pub schema: &'static str,
    pub status: &'static str,
    pub policy: String,
    pub desired_state: ServiceObservation,
    pub max_actions: u32,
    pub converged: bool,
    pub mutation_count: u32,
    pub steps: Vec<ReconciliationStepProof>,
    pub unknowns: Vec<String>,
}
