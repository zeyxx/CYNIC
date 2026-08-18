use serde::{Deserialize, Serialize};

pub const FLEET_INFERENCE_DEPLOYMENT_SCHEMA: &str = "cynic.inference.fleet_deployment.v1";
pub const FLEET_INFERENCE_OBSERVATION_SCHEMA: &str = "cynic.inference.fleet_observation.v1";
pub const FLEET_INFERENCE_PLAN_SCHEMA: &str = "cynic.inference.fleet_plan.v1";

/// Model-serving engine selected by operator intent. Engine-specific argv
/// remains in a node adapter; this type is intentionally only a portable
/// capability name. It deliberately excludes agent runtimes such as Hermes:
/// agents consume a published model endpoint but neither own model weights nor
/// decide their DNS cutover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetInferenceRuntime {
    LlamaCpp,
    Vllm,
}

/// Declarative model-serving desired state. `model_ref` must be immutable
/// at the deployment boundary (digest, pinned revision, or content-addressed
/// local artifact reference), never an unqualified moving tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetInferenceDeployment {
    pub schema: String,
    pub deployment_id: String,
    pub selector_tags: Vec<String>,
    pub replicas: u16,
    pub runtime: FleetInferenceRuntime,
    pub model_ref: String,
    pub service_name: String,
    pub dns_zone: String,
    pub port: u16,
}

/// Read-only node/runtime read-back supplied by a fleet or node adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetInferenceNodeObservation {
    pub schema: String,
    pub node_id: String,
    pub observed: bool,
    pub tags: Vec<String>,
    pub address: Option<String>,
    pub supported_runtimes: Vec<FleetInferenceRuntime>,
    pub active_runtime: Option<FleetInferenceRuntime>,
    pub active_model_ref: Option<String>,
    pub health_ok: Option<bool>,
    pub observed_at_unix_ms: Option<u64>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetInferenceTargetAction {
    Keep,
    Start,
    /// Engines cannot be assumed to swap weights safely in process. A node
    /// must create/read-back a replacement before DNS moves to it.
    RollingReplace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetInferenceTargetPlan {
    pub node_id: String,
    pub address: String,
    pub action: FleetInferenceTargetAction,
    pub runtime: FleetInferenceRuntime,
    pub model_ref: String,
    pub bounded_action: String,
    pub required_read_back: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetInferenceDnsAction {
    Verify,
    PublishAfterReadBack,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetInferenceDnsRecord {
    pub host: String,
    pub port: u16,
    pub node_id: String,
    pub address: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetInferencePlan {
    pub schema: String,
    pub deployment_id: String,
    pub status: String,
    pub targets: Vec<FleetInferenceTargetPlan>,
    pub dns_action: FleetInferenceDnsAction,
    pub dns_records: Vec<FleetInferenceDnsRecord>,
    pub dns_read_back_required: Vec<String>,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
}
