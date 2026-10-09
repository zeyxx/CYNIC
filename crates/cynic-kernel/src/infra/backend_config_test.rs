//! Test: kernel owns backend config types and loader locally.
//! No dependency on cynic-inference for these pure types.

use crate::infra::backend_config::{AuthStyle, BackendType, InferenceBackendConfig};
use crate::infra::config_loader::{load_inference_config_from_path_public, LoadedInferenceConfig};

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

#[test]
fn config_loader_reads_toml_and_injects_env_api_key() {
    let toml = r#"
[[backend]]
name = "test-local"
base_url = "http://localhost:8080/v1"
model = "test-model"
sovereign = true
"#;
    let path = std::env::temp_dir().join(format!(
        "cynic_kernel_test_{}.toml",
        std::process::id()
    ));
    std::fs::write(&path, toml).unwrap();

    let result = load_inference_config_from_path_public(&path);
    std::fs::remove_file(&path).unwrap();

    let loaded: LoadedInferenceConfig = result.expect("should parse");
    assert_eq!(loaded.backends.len(), 1);
    assert_eq!(loaded.backends[0].name, "test-local");
    assert!(loaded.backends[0].sovereign);
}
