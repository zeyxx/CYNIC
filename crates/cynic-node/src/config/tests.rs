use super::*;

const VALID_TOML: &str = r#"
[kernel]
url = "http://127.0.0.1:3030"
api_key_env = "TEST_CYNIC_API_KEY"

[dog]
name = "test-dog"
model = "test-model:7b"
base_url = "http://127.0.0.1:8080/v1"

[process]
command = ["llama-server", "-m", "/models/test.gguf"]
"#;

fn make_config_with_env(toml: &str, kernel_key: &str) -> Result<Config, String> {
    let mut cfg: Config = toml::from_str(toml).map_err(|e| e.to_string())?;
    cfg.kernel.api_key = kernel_key.to_owned();
    if let Some(env_name) = &cfg.dog.api_key_env
        && let Ok(v) = std::env::var(env_name)
    {
        cfg.dog.api_key = Some(v);
    }
    Ok(cfg)
}

#[test]
fn derive_health_url_strips_v1_suffix() {
    assert_eq!(
        derive_health_url("http://host:8080/v1"),
        "http://host:8080/health"
    );
}

#[test]
fn derive_health_url_strips_trailing_slash_and_v1() {
    assert_eq!(
        derive_health_url("http://host:8080/v1/"),
        "http://host:8080/health"
    );
}

#[test]
fn derive_health_url_no_v1_suffix() {
    assert_eq!(
        derive_health_url("http://host:8080"),
        "http://host:8080/health"
    );
}

#[test]
fn derive_health_url_trailing_slash_only() {
    assert_eq!(
        derive_health_url("http://host:8080/"),
        "http://host:8080/health"
    );
}

#[test]
fn derive_models_url_appends_models() {
    assert_eq!(
        derive_models_url("http://host:8080/v1"),
        "http://host:8080/v1/models"
    );
}

#[test]
fn derive_models_url_strips_trailing_slash() {
    assert_eq!(
        derive_models_url("http://host:8080/v1/"),
        "http://host:8080/v1/models"
    );
}

#[test]
fn parse_valid_config_succeeds() {
    let cfg = make_config_with_env(VALID_TOML, "secret-key").unwrap();
    assert_eq!(cfg.dog.name, "test-dog");
    assert_eq!(cfg.dog.model, "test-model:7b");
    assert_eq!(cfg.dog.base_url, "http://127.0.0.1:8080/v1");
    assert_eq!(cfg.kernel.url, "http://127.0.0.1:3030");
    assert_eq!(cfg.kernel.api_key, "secret-key");
}

#[test]
fn parse_config_serde_defaults() {
    let cfg = make_config_with_env(VALID_TOML, "key").unwrap();
    assert_eq!(cfg.dog.context_size, 4096);
    assert_eq!(cfg.dog.timeout_secs, 60);
    assert_eq!(cfg.kernel.heartbeat_interval_secs, 40);
    assert_eq!(cfg.restart.max_attempts, 5);
    assert_eq!(cfg.restart.initial_delay_secs, 2);
    assert_eq!(cfg.restart.max_delay_secs, 120);
    assert_eq!(cfg.restart.min_uptime_secs, 10);
    assert_eq!(cfg.health.interval_secs, 15);
    assert_eq!(cfg.health.verify_interval_secs, 60);
    assert_eq!(cfg.health.timeout_secs, 5);
    assert_eq!(cfg.health.max_failures, 3);
    assert_eq!(cfg.health.startup_timeout_secs, 120);
    assert_eq!(cfg.process.stop_timeout_secs, 10);
}

#[test]
fn runtime_federation_config_prefers_explicit_values() {
    let cfg = make_config_with_env(
        r#"
[kernel]
url = "http://kernel-http:3030"
api_key_env = "TEST_CYNIC_API_KEY"

[dog]
name = "explicit-node"
model = "test-model:7b"
base_url = "http://127.0.0.1:8080/v1"

[federation]
kernel_ws_url = "wss://kernel-ws/events/ws"
api_key = "explicit-api-key"
node_public_key = "explicit-public-key"
queue_dir = "/tmp/cynic-node-queues"

[process]
command = ["llama-server", "-m", "/models/test.gguf"]
"#,
        "kernel-key",
    )
    .unwrap();

    let federation = runtime_federation_config(&cfg);

    assert_eq!(federation.kernel_ws_url, "wss://kernel-ws/events/ws");
    assert_eq!(federation.api_key, "explicit-api-key");
    assert_eq!(federation.node_public_key, "explicit-public-key");
    assert_eq!(federation.node_identity, "explicit-node");
    assert_eq!(
        federation.queue_dir,
        std::path::PathBuf::from("/tmp/cynic-node-queues")
    );
}

#[test]
fn runtime_federation_config_derives_defaults_when_section_missing() {
    let cfg = make_config_with_env(VALID_TOML, "key").unwrap();
    let federation = runtime_federation_config(&cfg);

    assert_eq!(federation.kernel_ws_url, "ws://127.0.0.1:3030/events/ws");
    assert_eq!(federation.node_identity, "test-dog");

    let local_mode = std::env::var("CYNIC_ALLOW_LOCAL_DEV_BUILD")
        .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"));
    let expected_queue_dir = if local_mode {
        let home = std::env::var("HOME").ok().map(std::path::PathBuf::from);
        home.map(|path| path.join(".cynic/node"))
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp/cynic/node"))
    } else {
        std::path::PathBuf::from("/var/lib/cynic/node")
    };
    assert_eq!(federation.queue_dir, expected_queue_dir);
}

#[test]
fn validate_rejects_empty_command() {
    let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
    cfg.process.command.clear();
    let result = validate(&cfg);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("process.command"));
}

#[test]
fn validate_rejects_empty_name() {
    let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
    cfg.dog.name = String::new();
    let result = validate(&cfg);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("dog.name"));
}

#[test]
fn validate_rejects_name_too_long() {
    let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
    cfg.dog.name = "x".repeat(65);
    let result = validate(&cfg);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("dog.name"));
}

#[test]
fn validate_accepts_name_max_length() {
    let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
    cfg.dog.name = "x".repeat(64);
    assert!(validate(&cfg).is_ok());
}

#[test]
fn validate_rejects_zero_stop_timeout() {
    let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
    cfg.process.stop_timeout_secs = 0;
    let result = validate(&cfg);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("stop_timeout_secs"));
}

#[test]
fn validate_rejects_zero_max_attempts() {
    let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
    cfg.restart.max_attempts = 0;
    let result = validate(&cfg);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("max_attempts"));
}

#[test]
fn validate_rejects_startup_timeout_not_greater_than_health_timeout() {
    let mut cfg = make_config_with_env(VALID_TOML, "key").unwrap();
    assert!(validate(&cfg).is_ok());

    cfg.health.startup_timeout_secs = cfg.health.timeout_secs;
    let result = validate(&cfg);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("startup_timeout_secs"));
}

#[test]
fn expand_env_vars_no_pattern() {
    assert_eq!(expand_env_vars("plain-string").unwrap(), "plain-string");
}

#[test]
fn expand_env_vars_uses_real_env() {
    let path_val = std::env::var("PATH").unwrap();
    let result = expand_env_vars("prefix-${PATH}-suffix").unwrap();
    assert_eq!(result, format!("prefix-{path_val}-suffix"));
}

#[test]
fn expand_env_vars_missing_var() {
    let result = expand_env_vars("${DEFINITELY_MISSING_XYZ_99}");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("DEFINITELY_MISSING_XYZ_99"));
}

#[test]
fn expand_env_vars_dollar_without_brace() {
    assert_eq!(expand_env_vars("price=$5").unwrap(), "price=$5");
}

#[test]
fn node_control_config_loads_without_inference_sections() {
    use std::io::Write;

    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(
        br#"
[node_control]
kernel_url = "http://kernel.synapse.cynic:3030"
node_id = "cynic-builder"
signing_key_path = "/var/lib/cynic/identity/node-signing.key"
enrollment_request_path = "/var/lib/cynic/identity/enrollment-request.json"
enrollment_grant_path = "/var/lib/cynic/identity/enrollment-grant.json"
enrollment_authority_public_key_path = "/etc/cynic/enrollment-authority.pub"
enrollment_token_path = "/var/lib/cynic/identity/enrollment-token"
enrollment_seed_id = "cynic-builder"
enrollment_role = "build_runner"
observed_capabilities = ["build_runner"]
"#,
    )
    .unwrap();

    let config = load_node_control(file.path().to_str().unwrap()).unwrap();
    assert_eq!(config.node_id, "cynic-builder");
    assert_eq!(config.enrollment_role, "build_runner");
    assert_eq!(config.observed_capabilities, vec!["build_runner"]);
    assert_eq!(
        config.agent_workload_state_root,
        std::path::PathBuf::from("/var/lib/cynic")
    );
}

#[test]
fn load_fails_if_api_key_env_not_set() {
    use std::io::Write;

    let toml_with_missing_env = VALID_TOML.replace(
        "TEST_CYNIC_API_KEY",
        "DEFINITELY_NONEXISTENT_ENV_VAR_XYZ_42",
    );
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(toml_with_missing_env.as_bytes()).unwrap();
    let path = f.path().to_str().unwrap().to_owned();

    let result = load(&path);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("DEFINITELY_NONEXISTENT_ENV_VAR_XYZ_42")
    );
}
