//! Declarative Hermes gateway intent and read-only lifecycle state.

use serde::{Deserialize, Serialize};

pub const HERMES_GATEWAY_STATUS_SCHEMA: &str = "cynic.inference.hermes_gateway_status.v1";
pub const HERMES_GATEWAY_MUTATION_SCHEMA: &str = "cynic.inference.hermes_gateway_mutation.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HermesGatewayState {
    Inactive,
    Activating,
    Active,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HermesGatewayProfile {
    pub runtime_id: String,
    pub executable: String,
    pub home_dir: String,
    pub host: String,
    pub port: u16,
    pub api_key_present: bool,
    pub max_agent_turns: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HermesGatewayStatus {
    pub schema: &'static str,
    pub runtime_id: String,
    pub state: HermesGatewayState,
    pub pid: Option<u32>,
    pub endpoint: String,
    pub health_ok: bool,
    pub capabilities_ok: bool,
    pub model_count: Option<usize>,
    pub message: Option<String>,
}

impl HermesGatewayStatus {
    pub fn inactive(
        runtime_id: impl Into<String>,
        endpoint: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            schema: HERMES_GATEWAY_STATUS_SCHEMA,
            runtime_id: runtime_id.into(),
            state: HermesGatewayState::Inactive,
            pid: None,
            endpoint: endpoint.into(),
            health_ok: false,
            capabilities_ok: false,
            model_count: None,
            message: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HermesGatewayMutation {
    pub schema: &'static str,
    pub operation: String,
    pub runtime_id: String,
    pub ok: bool,
    pub status: HermesGatewayStatus,
    pub message: Option<String>,
}
