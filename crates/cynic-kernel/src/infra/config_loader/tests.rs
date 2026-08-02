use super::backend::{derive_health_url, load_backends};
use super::storage::load_storage_config;
use crate::infra::config_types::{AuthStyle, BackendType};
use cynic_protocol::{SecretClass, SecretProvider};
use std::io::Write;
use std::path::Path;

#[test]
fn parse_valid_toml() {
    let toml_content = r#"
[backend.local]
base_url = "http://localhost:8080/v1"
model = "phi-3-mini"
auth_style = "none"
"#;
    let dir = std::env::temp_dir().join("cynic_config_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("backends.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(toml_content.as_bytes()).unwrap();

    let configs = load_backends(&path);
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].name, "local");
    assert!(configs[0].api_key.is_none());
    assert!(matches!(configs[0].auth_style, AuthStyle::None));

    std::fs::remove_file(&path).ok();
}

#[test]
fn missing_file_returns_empty() {
    let configs = load_backends(Path::new("/nonexistent/backends.toml"));
    assert!(configs.is_empty());
}

#[test]
fn storage_config_retains_only_runtime_secret_references() {
    let toml_content = r#"
[storage]
url = "http://storage.internal:8000"
namespace = "cynic"
database = "main"
username = "cynic-runtime"
secret_env = "CYNIC_STORAGE_RUNTIME_PASSWORD"
secret_file = "/run/cynic-test/storage-runtime-password"
os_credential = "storage-runtime-password"

[backend.local]
base_url = "http://localhost:8080/v1"
model = "test"
"#;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("inference.toml");
    std::fs::write(&path, toml_content).unwrap();

    let config = load_storage_config(&path);
    assert_eq!(config.identity.username, "cynic-runtime");
    assert_eq!(config.identity.secret_sources.len(), 5);
    assert_eq!(
        config.identity.secret_sources[0].provider,
        SecretProvider::ProcessEnvironment
    );
    assert_eq!(
        config.identity.secret_sources[0].locator,
        "CYNIC_STORAGE_RUNTIME_PASSWORD"
    );
    assert_eq!(
        config.identity.secret_sources[1].provider,
        SecretProvider::PrincipalOwnedFile
    );
    assert_eq!(
        config.identity.secret_sources[1].locator,
        "/run/cynic-test/storage-runtime-password"
    );
    assert!(
        config
            .identity
            .secret_sources
            .iter()
            .all(|source| source.class == SecretClass::RuntimeMachineIdentity)
    );
}

#[test]
fn derive_health_url_strips_v1() {
    assert_eq!(
        derive_health_url("http://10.0.0.1:8080/v1"),
        "http://10.0.0.1:8080/health"
    );
    assert_eq!(
        derive_health_url("https://api.example.com/v1"),
        "https://api.example.com/health"
    );
}

#[test]
fn derive_health_url_no_v1() {
    assert_eq!(
        derive_health_url("http://10.0.0.1:8080"),
        "http://10.0.0.1:8080/health"
    );
    assert_eq!(
        derive_health_url("http://10.0.0.1:8080/"),
        "http://10.0.0.1:8080/health"
    );
}

#[test]
fn parse_toml_with_remediation() {
    let toml_content = r#"
[backend.sovereign]
base_url = "http://10.0.0.1:8080/v1"
model = "qwen3"
auth_style = "bearer"

[backend.sovereign.remediation]
node = "user@10.0.0.1"
restart_command = "systemctl --user restart llama-server"
max_retries = 5
cooldown_secs = 30
"#;
    let dir = std::env::temp_dir().join("cynic_config_remediation_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("backends.toml");
    std::fs::write(&path, toml_content).unwrap();

    let configs = load_backends(&path);
    assert_eq!(configs.len(), 1);
    assert_eq!(
        configs[0].health_url.as_deref(),
        Some("http://10.0.0.1:8080/health")
    );
    let rem = configs[0]
        .remediation
        .as_ref()
        .expect("remediation should be present");
    assert_eq!(rem.node, "user@10.0.0.1");
    assert_eq!(rem.max_retries, 5);
    assert_eq!(rem.cooldown_secs, 30);

    std::fs::remove_file(&path).ok();
}

#[test]
fn cloud_backend_without_remediation_gets_no_health_url() {
    let toml_content = r#"
[backend.gemini]
base_url = "https://api.google.com/v1"
model = "gemini-flash"
"#;
    let dir = std::env::temp_dir().join("cynic_config_no_rem_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("backends.toml");
    std::fs::write(&path, toml_content).unwrap();

    let configs = load_backends(&path);
    assert_eq!(configs.len(), 1);
    assert!(configs[0].remediation.is_none());
    assert!(
        configs[0].health_url.is_none(),
        "cloud backend should have no health_url, got {:?}",
        configs[0].health_url
    );

    std::fs::remove_file(&path).ok();
}

#[test]
fn sovereign_backend_with_remediation_gets_derived_health_url() {
    let toml_content = r#"
[backend.sovereign]
base_url = "http://10.0.0.1:8080/v1"
model = "qwen3"

[backend.sovereign.remediation]
node = "user@10.0.0.1"
restart_command = "systemctl restart llama-server"
"#;
    let dir = std::env::temp_dir().join("cynic_config_sov_health_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("backends.toml");
    std::fs::write(&path, toml_content).unwrap();

    let configs = load_backends(&path);
    assert_eq!(configs.len(), 1);
    assert!(configs[0].remediation.is_some());
    assert_eq!(
        configs[0].health_url.as_deref(),
        Some("http://10.0.0.1:8080/health"),
        "sovereign with remediation should get derived health_url"
    );

    std::fs::remove_file(&path).ok();
}

#[test]
fn explicit_health_url_overrides_derivation() {
    let toml_content = r#"
[backend.custom]
base_url = "http://10.0.0.1:8080/v1"
model = "custom-model"
health_url = "http://custom-health:9090/ready"
"#;
    let dir = std::env::temp_dir().join("cynic_config_explicit_health_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("backends.toml");
    std::fs::write(&path, toml_content).unwrap();

    let configs = load_backends(&path);
    assert_eq!(configs.len(), 1);
    assert_eq!(
        configs[0].health_url.as_deref(),
        Some("http://custom-health:9090/ready"),
        "explicit health_url should be used even without remediation"
    );

    std::fs::remove_file(&path).ok();
}

#[test]
fn parse_toml_with_inference_params() {
    let toml_content = r#"
[backend.sovereign]
base_url = "http://10.0.0.1:8080/v1"
model = "qwen3-4b"
auth_style = "none"
timeout_secs = 90
max_tokens = 2048
temperature = 0.2
disable_thinking = true
"#;
    let dir = std::env::temp_dir().join("cynic_config_inference_params_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("backends.toml");
    std::fs::write(&path, toml_content).unwrap();

    let configs = load_backends(&path);
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].timeout_secs, 90);
    assert_eq!(configs[0].max_tokens, 2048);
    assert!((configs[0].temperature - 0.2).abs() < 0.01);
    assert!(configs[0].disable_thinking);

    std::fs::remove_file(&path).ok();
}

#[test]
fn inference_params_default_when_absent() {
    let toml_content = r#"
[backend.cloud]
base_url = "https://api.example.com/v1"
model = "gpt-4"
"#;
    let dir = std::env::temp_dir().join("cynic_config_inference_defaults_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("backends.toml");
    std::fs::write(&path, toml_content).unwrap();

    let configs = load_backends(&path);
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].timeout_secs, 30);
    assert_eq!(configs[0].max_tokens, 4096);
    assert!((configs[0].temperature - 0.3).abs() < 0.01);
    assert!(!configs[0].disable_thinking);

    std::fs::remove_file(&path).ok();
}

#[test]
fn parse_cli_backend_type() {
    let toml_content = r#"
[backend.gemini-flash]
backend_type = "cli"
base_url = "gemini"
model = "gemini-2.5-flash"
auth_style = "none"
timeout_secs = 60
"#;
    let dir = std::env::temp_dir().join("cynic_config_cli_type_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("backends.toml");
    std::fs::write(&path, toml_content).unwrap();

    let configs = load_backends(&path);
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].backend_type, BackendType::Cli);
    assert_eq!(configs[0].name, "gemini-flash");
    assert_eq!(configs[0].base_url, "gemini");
    assert!(
        configs[0].health_url.is_none(),
        "CLI backend should have no health_url"
    );

    std::fs::remove_file(&path).ok();
}

#[test]
fn parse_inference_array_schema() {
    let toml_content = r#"
[[backend]]
name = "llama-local"
base_url = "http://127.0.0.1:8080/v1"
model = "qwen2.5"
sovereign = true
api_key = "plain-text-key"
"#;
    let dir = std::env::temp_dir().join("cynic_config_inference_array_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("inference.toml");
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(toml_content.as_bytes()).unwrap();

    let configs = load_backends(&path);
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].name, "llama-local");
    assert_eq!(configs[0].base_url, "http://127.0.0.1:8080/v1");
    assert_eq!(configs[0].model, "qwen2.5");
    assert!(configs[0].api_key.as_deref().is_some());
    assert_eq!(configs[0].api_key.as_deref(), Some("plain-text-key"));

    std::fs::remove_file(&path).ok();
}
