//! Network Actuation Types & Profile Models
//! Clean-room extracted from cynic-node to cynic-network per Foundation §481 (SoC).
//! Pure data structures for actuation plans, bundles, and status reporting.

use serde::{Deserialize, Serialize};

pub const NETWORK_ACTUATOR_MANIFEST_SCHEMA: &str = "cynic.network_actuator.bundle_manifest.v1";
pub const NETWORK_ACTUATOR_ACTIVATION_PLAN_SCHEMA: &str =
    "cynic.network_actuator.activation_plan.v1";
pub const NETWORK_ACTUATOR_APPLY_REPORT_SCHEMA: &str = "cynic.network_actuator.apply_report.v1";
pub const NETWORK_ACTUATOR_STATUS_SCHEMA: &str = "cynic.network_actuator.activation_status.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkActuatorBundleManifest {
    pub schema: String,
    pub files: Vec<String>,
    pub binary_sha256: String,
    pub generated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkActuatorActivationPlan {
    pub schema: String,
    pub profile: String,
    pub commands: Vec<Vec<String>>,
    pub target_network: String,
    pub generated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkActuatorApplyReport {
    pub schema: String,
    pub status: String,
    pub profile: String,
    pub applied_commands_count: usize,
    pub executed_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkActuatorActivationStatus {
    pub schema: String,
    pub active_profile: Option<String>,
    pub is_active: bool,
    pub last_checked_unix_ms: u64,
}
