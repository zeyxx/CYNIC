use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn installed_runtime_paths_are_resolved_outside_repo() {
    let root = temp_fixture("installed-runtime");
    let config_root = root.join("etc/cynic");
    let policy_root = root.join("etc/cynic-policies");
    let libexec_root = root.join("usr/local/libexec");
    let state_root = root.join("var/lib/cynic");
    let log_root = root.join("var/log/cynic");
    let provider_hosts_path = config_root.join("provider-fallback.hosts");
    let registry_path = config_root.join("registry.json");
    let outside_repo = root.join("outside");

    fs::create_dir_all(&config_root).expect("config root");
    fs::create_dir_all(&policy_root).expect("policy root");
    fs::create_dir_all(&libexec_root).expect("libexec root");
    fs::create_dir_all(&state_root).expect("state root");
    fs::create_dir_all(&log_root).expect("log root");
    fs::create_dir_all(&outside_repo).expect("outside repo");
    fs::write(&registry_path, "{}\n").expect("registry");

    let output = Command::new(env!("CARGO_BIN_EXE_cynic"))
        .current_dir(&outside_repo)
        .env_remove("CYNIC_PROJECT_ROOT")
        .env_remove("CYNIC_REGISTRY")
        .env("CYNIC_CONFIG_ROOT", &config_root)
        .env("CYNIC_POLICY_ROOT", &policy_root)
        .env("CYNIC_LIBEXEC_ROOT", &libexec_root)
        .env("CYNIC_STATE_ROOT", &state_root)
        .env("CYNIC_LOG_ROOT", &log_root)
        .env("CYNIC_PROVIDER_HOSTS_PATH", &provider_hosts_path)
        .arg("runtime")
        .arg("paths")
        .arg("--json")
        .output()
        .expect("run cynic");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("runtime paths json");

    assert_eq!(payload["mode"], "installed");
    assert_eq!(
        payload["registry_path"],
        registry_path.to_string_lossy().to_string()
    );
    assert_eq!(
        payload["config_root"],
        config_root.to_string_lossy().to_string()
    );
    assert_eq!(
        payload["policy_root"],
        policy_root.to_string_lossy().to_string()
    );
    assert_eq!(
        payload["libexec_root"],
        libexec_root.to_string_lossy().to_string()
    );
    assert_eq!(
        payload["state_root"],
        state_root.to_string_lossy().to_string()
    );
    assert_eq!(payload["log_root"], log_root.to_string_lossy().to_string());
    assert_eq!(
        payload["provider_hosts_path"],
        provider_hosts_path.to_string_lossy().to_string()
    );
}

fn temp_fixture(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cynic-cli-{name}-{unique}"));
    fs::create_dir_all(&path).expect("fixture dir");
    path
}
