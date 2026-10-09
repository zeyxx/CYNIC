//! Test: kernel owns backend config types locally.
//! No dependency on cynic-inference for these pure types.

use crate::infra::backend_config::{AuthStyle, BackendType, InferenceBackendConfig};

#[test]
fn backend_config_types_are_locally_owned() {
    let config = InferenceBackendConfig {
        name: "test-backend".to_string(),
        backend_type: BackendType::OpenAi,
        base_url: "http://localhost:8080/v1".to_string(),
        api_key: None,
        model: "test-model".to_string(),
        auth_style: AuthStyle::None,
        context_size: 4096,
        timeout_secs: 120,
        max_tokens: 4096,
        temperature: 0.7,
        fleet_node: None,
        disable_thinking: false,
        json_mode: false,
        supports_tool_use: false,
        health_url: None,
        cli_extra_args: vec![],
        sovereign: true,
    };

    assert_eq!(config.name, "test-backend");
    assert_eq!(config.backend_type, BackendType::OpenAi);
    assert!(config.sovereign);
}
