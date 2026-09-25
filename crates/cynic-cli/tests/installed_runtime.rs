use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};
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

    ensure_dir_all(&config_root, "config root");
    ensure_dir_all(&policy_root, "policy root");
    ensure_dir_all(&libexec_root, "libexec root");
    ensure_dir_all(&state_root, "state root");
    ensure_dir_all(&log_root, "log root");
    ensure_dir_all(&outside_repo, "outside repo");
    write_text_file(&registry_path, "{}\n", "registry file");

    let mut command = Command::new(env!("CARGO_BIN_EXE_cynic"));
    let output = output_expect(
        command
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
            .arg("--json"),
        "run cynic runtime paths command",
    );

    assert_success_output(&output, "cynic runtime paths");

    let payload: serde_json::Value = parse_json_payload(&output.stdout, "runtime paths");

    assert_eq!(payload["mode"], "installed");
    assert_eq!(payload["projection_class"], "runtime_authority");
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

#[test]
fn network_dns_plan_runs_outside_repo_without_project_root() {
    let root = temp_fixture("installed-runtime-dns-plan");
    let outside_repo = root.join("outside");

    ensure_dir_all(&outside_repo, "outside repo");

    let mut command = Command::new(env!("CARGO_BIN_EXE_cynic"));
    let output = output_expect(
        command
            .current_dir(&outside_repo)
            .env_remove("CYNIC_PROJECT_ROOT")
            .env_remove("CYNIC_REGISTRY")
            .arg("network")
            .arg("dns")
            .arg("plan")
            .arg("--json"),
        "run cynic network dns plan",
    );

    assert_success_output(&output, "cynic network dns plan");

    let payload: serde_json::Value = parse_json_payload(&output.stdout, "dns plan");

    assert_eq!(
        payload["schema"],
        "cynic.organ_fleet.dns_activation_plan.v1"
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("CYNIC_PROJECT_ROOT"),
        "standalone dns plan must not require a checkout root"
    );
}

#[test]
fn runtime_report_persists_with_report_dir_override_without_project_root() {
    let root = temp_fixture("installed-runtime-report");
    let outside_repo = root.join("outside");
    let report_root = root.join("var/lib/cynic/reports");
    let output_path = report_root.join("runtime-report.json");

    ensure_dir_all(&outside_repo, "outside repo");
    ensure_dir_all(&report_root, "report root");

    let mut command = Command::new(env!("CARGO_BIN_EXE_cynic"));
    let output = output_expect(
        command
            .current_dir(&outside_repo)
            .env_remove("CYNIC_PROJECT_ROOT")
            .env_remove("CYNIC_REGISTRY")
            .env("CYNIC_REPORT_DIR", &report_root)
            .arg("runtime")
            .arg("report")
            .arg("--json")
            .arg("--persist"),
        "run cynic runtime report",
    );

    assert_success_output(&output, "cynic runtime report");

    let data: serde_json::Value = parse_json_payload(&output.stdout, "runtime report");

    assert_eq!(data["schema"], "cynic.runtime.report.v1");
    assert_eq!(
        data["persistence"]["default_path"],
        "/var/lib/cynic/reports/runtime-report.json"
    );
    assert_eq!(
        data["persistence"]["written_path"],
        serde_json::Value::String(output_path.to_string_lossy().to_string())
    );
    assert_eq!(data["persistence"]["mutates_filesystem"], true);
    assert!(
        output_path.exists(),
        "runtime report should be persisted to CYNIC_REPORT_DIR"
    );
}

#[test]
fn runtime_report_supports_explicit_output_without_project_root() {
    let root = temp_fixture("installed-runtime-report-output");
    let outside_repo = root.join("outside");
    let output_path = root.join("explicit-runtime-report.json");

    ensure_dir_all(&outside_repo, "outside repo");

    let mut command = Command::new(env!("CARGO_BIN_EXE_cynic"));
    let output = output_expect(
        command
            .current_dir(&outside_repo)
            .env_remove("CYNIC_PROJECT_ROOT")
            .env_remove("CYNIC_REGISTRY")
            .arg("runtime")
            .arg("report")
            .arg("--json")
            .arg("--output")
            .arg(&output_path),
        "run cynic runtime report",
    );

    assert_success_output(&output, "cynic runtime report");

    let data: serde_json::Value = parse_json_payload(&output.stdout, "runtime report");

    assert_eq!(data["schema"], "cynic.runtime.report.v1");
    assert_eq!(
        data["persistence"]["written_path"],
        serde_json::Value::String(output_path.to_string_lossy().to_string())
    );
    assert_eq!(data["persistence"]["mutates_filesystem"], true);
    assert!(
        output_path.exists(),
        "runtime report should be written to explicit output"
    );
}

fn temp_fixture(name: &str) -> PathBuf {
    let unique = current_time_nanos();
    let path = std::env::temp_dir().join(format!("cynic-cli-{name}-{unique}"));
    ensure_dir_all(&path, "fixture dir");
    path
}

fn current_time_nanos() -> u128 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(error) => {
            panic!("failed to read clock: {error}");
        }
    }
}

fn ensure_dir_all(path: &PathBuf, context: &str) {
    if let Err(error) = fs::create_dir_all(path) {
        panic!("failed to create {context} at {}: {error}", path.display());
    }
}

fn write_text_file(path: &PathBuf, content: &str, context: &str) {
    if let Err(error) = fs::write(path, content) {
        panic!("failed to write {context} at {}: {error}", path.display());
    }
}

fn output_expect(command: &mut Command, context: &str) -> Output {
    match command.output() {
        Ok(output) => output,
        Err(error) => panic!("failed to execute {context}: {error}"),
    }
}

fn assert_success_output(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn parse_json_payload(raw: &[u8], context: &str) -> serde_json::Value {
    match serde_json::from_slice(raw) {
        Ok(payload) => payload,
        Err(error) => panic!("failed to parse {context} json payload: {error}"),
    }
}

#[test]
fn inference_start_process_supervisor_can_launch_and_supervise_fake_llama_server() {
    let root = temp_fixture("installed-runtime-inference-start");
    let profiles_root = root.join("profiles");
    let vault_root = root.join("vault");
    let alias = format!("fake-{}", current_time_nanos());
    let port = unique_port("start", &alias);

    ensure_dir_all(&profiles_root, "profiles root");
    ensure_dir_all(&vault_root, "vault root");

    let model_path = vault_root.join(format!("{alias}.gguf"));
    write_minimal_gguf(&model_path, "llama", &alias);

    let fake_server = write_fake_llama_server(&root, &alias, &alias);
    write_fake_profile(
        &profiles_root.join(format!("{alias}.env")),
        fake_server.to_string_lossy().as_ref(),
        model_path.to_string_lossy().as_ref(),
        &alias,
        port,
    );

    let principal_home = root.join("principal");
    ensure_dir_all(&principal_home, "principal home root");

    let start = run_cynic_cli(
        root.clone(),
        Some(("CYNIC_INFERENCE_SUPERVISOR", "process")),
        vec![
            (
                "CYNIC_MODEL_VAULT",
                vault_root.to_string_lossy().into_owned(),
            ),
            (
                "CYNIC_MODEL_PROFILES",
                profiles_root.to_string_lossy().into_owned(),
            ),
            (
                "CYNIC_PRINCIPAL_HOME",
                principal_home.to_string_lossy().into_owned(),
            ),
            (
                "CYNIC_LLAMA_SERVER_BIN",
                fake_server.to_string_lossy().into_owned(),
            ),
        ],
<<<<<<< HEAD
        &["runtime", "inference-start", &alias, "--yes", "--json"],
=======
        &["runtime", "inference-start", &alias, "--json"],
>>>>>>> github/main
    );
    assert_success_output(&start, "inference-start fake model");
    let started = parse_json_payload(&start.stdout, "inference-start payload");
    assert_eq!(
        started["ok"],
        serde_json::Value::Bool(true),
        "start report should be ok"
    );
    assert_eq!(
        started["status"]["state"],
        serde_json::Value::String("active".into()),
        "start should expose active state"
    );

    let status = run_cynic_cli(
        root.clone(),
        Some(("CYNIC_INFERENCE_SUPERVISOR", "process")),
        vec![
            (
                "CYNIC_MODEL_VAULT",
                vault_root.to_string_lossy().into_owned(),
            ),
            (
                "CYNIC_MODEL_PROFILES",
                profiles_root.to_string_lossy().into_owned(),
            ),
            (
                "CYNIC_PRINCIPAL_HOME",
                principal_home.to_string_lossy().into_owned(),
            ),
        ],
        &["runtime", "inference-status", "--json"],
    );
    assert_success_output(&status, "inference-status while running");
    let running = parse_json_payload(&status.stdout, "inference-status payload");
    assert_eq!(
        running["state"],
        serde_json::Value::String("active".into()),
        "status should report active"
    );

    let log_path = principal_home
        .join(".local/state/cynic/inference/process")
        .join(format!("{alias}.log"));
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        let mut handle = match OpenOptions::new().create(true).append(true).open(&log_path) {
            Ok(handle) => handle,
            Err(error) => panic!("failed to append test marker to {log_path:?}: {error}"),
        };
        if let Err(error) = writeln!(handle, "INFERENCE_STATUS_TAIL_MARKER") {
            panic!("failed to write test marker to {log_path:?}: {error}");
        }
    }

    let status_tail = run_cynic_cli(
        root.clone(),
        Some(("CYNIC_INFERENCE_SUPERVISOR", "process")),
        vec![
            (
                "CYNIC_MODEL_VAULT",
                vault_root.to_string_lossy().into_owned(),
            ),
            (
                "CYNIC_MODEL_PROFILES",
                profiles_root.to_string_lossy().into_owned(),
            ),
            (
                "CYNIC_PRINCIPAL_HOME",
                principal_home.to_string_lossy().into_owned(),
            ),
        ],
        &["runtime", "inference-status", "--tail", "6"],
    );
    assert_success_output(&status_tail, "inference-status with tail while running");
    assert!(
        String::from_utf8_lossy(&status_tail.stdout).contains("INFERENCE_STATUS_TAIL_MARKER"),
        "inference-status --tail should print process log tail"
    );

    let stop = run_cynic_cli(
        root,
        Some(("CYNIC_INFERENCE_SUPERVISOR", "process")),
        vec![
            (
                "CYNIC_MODEL_VAULT",
                vault_root.to_string_lossy().into_owned(),
            ),
            (
                "CYNIC_MODEL_PROFILES",
                profiles_root.to_string_lossy().into_owned(),
            ),
            (
                "CYNIC_PRINCIPAL_HOME",
                principal_home.to_string_lossy().into_owned(),
            ),
        ],
<<<<<<< HEAD
        &["runtime", "inference-stop", &alias, "--yes", "--json"],
=======
        &["runtime", "inference-stop", &alias, "--json"],
>>>>>>> github/main
    );
    assert_success_output(&stop, "inference-stop fake model");
    let stopped = parse_json_payload(&stop.stdout, "inference-stop payload");
    assert_eq!(
        stopped["ok"],
        serde_json::Value::Bool(true),
        "stop report should be ok"
    );
}

#[test]
fn inference_status_reports_inactive_when_nothing_is_running_for_clean_principal() {
    let root = temp_fixture("installed-runtime-inference-status-inactive");
    let principal_home = root.join("principal");
    ensure_dir_all(&principal_home, "principal home root");

    let status = run_cynic_cli(
        root,
        Some(("CYNIC_INFERENCE_SUPERVISOR", "process")),
<<<<<<< HEAD
        vec![(
            "CYNIC_PRINCIPAL_HOME",
            principal_home.to_string_lossy().into_owned(),
        )],
=======
        vec![("CYNIC_PRINCIPAL_HOME", principal_home.to_string_lossy().into_owned())],
>>>>>>> github/main
        &["runtime", "inference-status", "--json"],
    );

    assert_success_output(&status, "inference-status when idle");
    let payload = parse_json_payload(&status.stdout, "inference-status payload");
<<<<<<< HEAD
    assert_eq!(
        payload["state"],
        serde_json::Value::String("inactive".into())
    );
=======
    assert_eq!(payload["state"], serde_json::Value::String("inactive".into()));
>>>>>>> github/main
    assert!(payload["alias"].is_null());
}

#[test]
<<<<<<< HEAD
fn models_status_projects_the_same_read_only_supervisor_observation() {
    let root = temp_fixture("operator-models-status-inactive");
    let principal_home = root.join("principal");
    ensure_dir_all(&principal_home, "principal home root");

    let status = run_cynic_cli(
        root,
        Some(("CYNIC_INFERENCE_SUPERVISOR", "process")),
        vec![(
            "CYNIC_PRINCIPAL_HOME",
            principal_home.to_string_lossy().into_owned(),
        )],
        &["models", "status", "--json"],
    );

    assert_success_output(&status, "operator models status when idle");
    let payload = parse_json_payload(&status.stdout, "operator models status payload");
    assert_eq!(
        payload["state"],
        serde_json::Value::String("inactive".into())
    );
}

#[test]
fn inference_start_requires_explicit_operator_consent() {
    let root = temp_fixture("installed-runtime-inference-start-consent");
    let principal_home = root.join("principal");
    ensure_dir_all(&principal_home, "principal home root");

    let start = run_cynic_cli(
        root,
        Some(("CYNIC_INFERENCE_SUPERVISOR", "process")),
        vec![(
            "CYNIC_PRINCIPAL_HOME",
            principal_home.to_string_lossy().into_owned(),
        )],
        &["runtime", "inference-start", "auto", "--json"],
    );

    assert!(!start.status.success(), "start without --yes must fail");
    assert!(
        String::from_utf8_lossy(&start.stderr).contains("without --yes"),
        "consent failure should name the missing flag"
    );
}

#[test]
=======
>>>>>>> github/main
fn inference_stop_requires_running_instance_or_alias() {
    let root = temp_fixture("installed-runtime-inference-stop-noop");
    let principal_home = root.join("principal");
    ensure_dir_all(&principal_home, "principal home root");

    let stop = run_cynic_cli(
        root,
        Some(("CYNIC_INFERENCE_SUPERVISOR", "process")),
<<<<<<< HEAD
        vec![(
            "CYNIC_PRINCIPAL_HOME",
            principal_home.to_string_lossy().into_owned(),
        )],
        &["runtime", "inference-stop", "--yes", "--json"],
    );

    assert!(
        !stop.status.success(),
        "inference stop without running instance should fail"
    );
    assert!(
        String::from_utf8_lossy(&stop.stderr).contains("no running process instance to stop"),
=======
        vec![("CYNIC_PRINCIPAL_HOME", principal_home.to_string_lossy().into_owned())],
        &["runtime", "inference-stop", "--json"],
    );

    assert!(!stop.status.success(), "inference stop without running instance should fail");
    assert!(
        String::from_utf8_lossy(&stop.stderr)
            .contains("no running process instance to stop"),
>>>>>>> github/main
        "stop failure should be explicit"
    );
}

#[test]
fn inference_start_fails_fast_when_gguf_architecture_is_unsupported() {
    let root = temp_fixture("installed-runtime-inference-start-unsupported");
    let profiles_root = root.join("profiles");
    let vault_root = root.join("vault");
    let alias = format!("unsupported-{}", current_time_nanos());

    ensure_dir_all(&profiles_root, "profiles root");
    ensure_dir_all(&vault_root, "vault root");

    let model_path = vault_root.join(format!("{alias}.gguf"));
    write_minimal_gguf(&model_path, "dspark", &alias);

    let profile_path = profiles_root.join(format!("{alias}.env"));
    let port = unique_port("unsupported", &alias);
    write_text_file(
        &profile_path,
        &format!(
            "LLAMA_SERVER_BIN=/usr/bin/true\nLLAMA_MODEL_PATH={}\nLLAMA_HOST=127.0.0.1\nLLAMA_PORT={port}\nLLAMA_CTX_SIZE=1024\n",
            model_path.display()
        ),
        "unsupported model profile",
    );

    let start = run_cynic_cli(
        root,
        Some(("CYNIC_INFERENCE_SUPERVISOR", "process")),
        vec![
            (
                "CYNIC_MODEL_VAULT",
                vault_root.to_string_lossy().into_owned(),
            ),
            (
                "CYNIC_MODEL_PROFILES",
                profiles_root.to_string_lossy().into_owned(),
            ),
        ],
<<<<<<< HEAD
        &["runtime", "inference-start", &alias, "--yes", "--json"],
=======
        &["runtime", "inference-start", &alias, "--json"],
>>>>>>> github/main
    );

    assert!(!start.status.success(), "unsupported GGUF must fail fast");
    assert!(
        !String::from_utf8_lossy(&start.stdout).is_empty()
            || !String::from_utf8_lossy(&start.stderr).is_empty(),
        "failure should emit telemetry in stdout or stderr"
    );
}

fn write_minimal_gguf(path: &PathBuf, architecture: &str, model_name: &str) {
    let mut file = match fs::File::create(path) {
        Ok(file) => file,
        Err(error) => panic!("failed to create fake model {path:?}: {error}"),
    };

    let write_u32 = |file: &mut fs::File, value: u32| {
        if let Err(error) = file.write_all(&value.to_le_bytes()) {
            panic!("failed to write gguf u32 for {path:?}: {error}");
        }
    };

    let write_u64 = |file: &mut fs::File, value: u64| {
        if let Err(error) = file.write_all(&value.to_le_bytes()) {
            panic!("failed to write gguf u64 for {path:?}: {error}");
        }
    };

    let write_str = |file: &mut fs::File, value: &str| {
        let bytes = value.as_bytes();
        write_u64(file, bytes.len() as u64);
        if let Err(error) = file.write_all(bytes) {
            panic!("failed to write gguf string '{value}' for {path:?}: {error}");
        }
    };

    if let Err(error) = file.write_all(b"GGUF") {
        panic!("failed to write gguf magic {path:?}: {error}");
    }

    write_u32(&mut file, 3);
    write_u64(&mut file, 0);
    write_u64(&mut file, 2);

    write_str(&mut file, "general.architecture");
    write_u32(&mut file, 8);
    write_str(&mut file, architecture);

    write_str(&mut file, "general.name");
    write_u32(&mut file, 8);
    write_str(&mut file, model_name);
}

fn write_fake_llama_server(root: &PathBuf, alias: &str, model_id: &str) -> PathBuf {
    let path = root.join(format!("fake_llama_server_{alias}.py"));
    let payload_template = r#"#!/usr/bin/env python3
import argparse
import json
from http.server import BaseHTTPRequestHandler, HTTPServer

parser = argparse.ArgumentParser()
parser.add_argument("-m", default="")
parser.add_argument("--host", default="127.0.0.1")
parser.add_argument("--port", type=int, default=8080)
args, _ = parser.parse_known_args()


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path.startswith("/v1/models"):
            body = json.dumps({"data": [{"id": "__MODEL_ID__"}]})
            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(body)))
            self.end_headers()
            self.wfile.write(body.encode("utf-8"))
            return

        self.send_response(404)
        self.end_headers()


httpd = HTTPServer((args.host, args.port), Handler)
httpd.serve_forever()
"#;
    let payload = payload_template.replace("__MODEL_ID__", model_id);
    if let Err(error) = fs::write(&path, payload) {
        panic!("failed to write fake llama binary {path:?}: {error}");
    }
    let mut perms = match fs::metadata(&path) {
        Ok(meta) => meta.permissions(),
        Err(error) => panic!("failed to read fake llama metadata {path:?}: {error}"),
    };
    perms.set_mode(0o755);
    if let Err(error) = fs::set_permissions(&path, perms) {
        panic!("failed to chmod fake llama binary {path:?}: {error}");
    }
    path
}

fn write_fake_profile(path: &PathBuf, server_bin: &str, model_path: &str, _alias: &str, port: u16) {
    let content = format!(
        "LLAMA_SERVER_BIN={server_bin}\nLLAMA_MODEL_PATH={model_path}\nLLAMA_HOST=127.0.0.1\nLLAMA_PORT={port}\nLLAMA_CTX_SIZE=4096\n",
        server_bin = server_bin,
        model_path = model_path,
        port = port
    );
    write_text_file(path, &content, "fake profile");
}

fn run_cynic_cli(
    root: PathBuf,
    principal: Option<(&str, &str)>,
    envs: Vec<(&str, String)>,
    args: &[&str],
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cynic"));
    let mut cmd = command.current_dir(&root).env_remove("CYNIC_PROJECT_ROOT");
    let mut all = vec![("CYNIC_PROGRESS", String::from("0"))];
    if let Some((key, value)) = principal {
        all.push((key, value.to_string()));
    }
    for (k, v) in all {
        cmd = cmd.env(k, v);
    }
    for (k, v) in envs {
        cmd = cmd.env(k, v);
    }

    cmd.args(args);
    output_expect(&mut cmd, "run cynic inference cli")
}

fn unique_port(prefix: &str, seed: &str) -> u16 {
    let value = format!("{prefix}-{seed}");
    let mut acc = 0u64;
    for b in value.as_bytes() {
        acc = acc.wrapping_mul(131).wrapping_add(*b as u64);
    }
    ((acc % 20000) + 20000) as u16
}
