//! Model catalog projection for inference backends.
//!
//! This module owns inference model-list semantics. Runtime adapters may probe
//! nodes and REST handlers may persist observations, but capability/mismatch
//! projection belongs to inference.

use serde::Serialize;

use crate::domain::backend_config::{BackendType, InferenceBackendConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelProbeObservation {
    pub actual_model: String,
    pub reachable: bool,
    pub latency_ms: u64,
    pub failure_reason: String,
}

impl ModelProbeObservation {
    pub fn skipped() -> Self {
        Self {
            actual_model: "N/A".to_string(),
            reachable: true,
            latency_ms: 0,
            failure_reason: "none".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ModelInfo {
    pub dog: String,
    pub node: String,
    pub expected_model: String,
    pub actual_model: String,
    pub mismatch: bool,
    pub reachable: bool,
    pub latency_ms: u64,
    pub capabilities: serde_json::Value,
    pub performance: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ListModelsResponse {
    pub models: Vec<ModelInfo>,
    pub timestamp: i64,
}

pub fn deterministic_model_info() -> ModelInfo {
    ModelInfo {
        dog: "deterministic-dog".to_string(),
        node: "deterministic".to_string(),
        expected_model: "deterministic".to_string(),
        actual_model: "N/A".to_string(),
        mismatch: false,
        reachable: true,
        latency_ms: 0,
        capabilities: serde_json::json!({
            "context_size": 0,
            "thinking_mode": false,
            "json_mode": false,
        }),
        performance: empty_performance(),
    }
}

pub fn should_probe_model(config: &InferenceBackendConfig) -> bool {
    config.backend_type != BackendType::Cli
}

pub fn project_model_info(
    config: &InferenceBackendConfig,
    node: &str,
    probe: ModelProbeObservation,
) -> ModelInfo {
    let mismatch = model_mismatch(&config.model, &probe.actual_model);

    ModelInfo {
        dog: config.name.clone(),
        node: node.to_string(),
        expected_model: config.model.clone(),
        actual_model: probe.actual_model,
        mismatch,
        reachable: probe.reachable,
        latency_ms: probe.latency_ms,
        capabilities: serde_json::json!({
            "context_size": config.context_size,
            "thinking_mode": !config.disable_thinking,
            "json_mode": config.json_mode,
        }),
        performance: empty_performance(),
    }
}

fn model_mismatch(expected_model: &str, actual_model: &str) -> bool {
    expected_model != actual_model
        && expected_model != "N/A"
        && actual_model != "N/A"
        && actual_model != "unknown"
        && actual_model != "parse_error"
}

fn empty_performance() -> serde_json::Value {
    serde_json::json!({
        "success_count": 0,
        "failures": 0,
        "json_valid_rate": 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::backend_config::AuthStyle;

    #[test]
    fn projects_backend_capabilities_and_mismatch() {
        let model = project_model_info(
            &config(),
            "node-a",
            ModelProbeObservation {
                actual_model: "other-model".to_string(),
                reachable: true,
                latency_ms: 42,
                failure_reason: "model_mismatch".to_string(),
            },
        );

        assert_eq!(model.dog, "dog-a");
        assert_eq!(model.node, "node-a");
        assert!(model.mismatch);
        assert_eq!(model.capabilities["context_size"], 32768);
        assert_eq!(model.capabilities["thinking_mode"], false);
        assert_eq!(model.capabilities["json_mode"], true);
    }

    #[test]
    fn cli_backends_are_not_model_probed() {
        let mut config = config();
        config.backend_type = BackendType::Cli;
        assert!(!should_probe_model(&config));
    }

    fn config() -> InferenceBackendConfig {
        InferenceBackendConfig {
            name: "dog-a".to_string(),
            backend_type: BackendType::OpenAi,
            base_url: "http://127.0.0.1:8080/v1".to_string(),
            api_key: None,
            model: "expected-model".to_string(),
            auth_style: AuthStyle::None,
            context_size: 32768,
            timeout_secs: 30,
            max_tokens: 4096,
            temperature: 0.3,
            disable_thinking: true,
            json_mode: true,
            supports_tool_use: false,
            health_url: Some("http://127.0.0.1:8080/health".to_string()),
            cli_extra_args: vec![],
            sovereign: true,
        }
    }
}
