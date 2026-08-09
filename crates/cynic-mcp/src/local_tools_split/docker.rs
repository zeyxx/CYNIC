use super::shared::{DockerCmdResult, DockerResult};
use cynic_host::ports::argv_execution::{ArgvExecutionPort, ArgvExecutionRequest};
use serde_json::Value;

pub async fn run_docker_container_list(runner: &dyn ArgvExecutionPort, all: bool) -> DockerResult {
    let args = if all {
        vec![
            "ps".to_string(),
            "--all".to_string(),
            "--no-trunc".to_string(),
            "--format".to_string(),
            "{{json .}}".to_string(),
        ]
    } else {
        vec![
            "ps".to_string(),
            "--no-trunc".to_string(),
            "--format".to_string(),
            "{{json .}}".to_string(),
        ]
    };
    let command = format!("docker {}", args.join(" "));
    let raw = run_docker_cmd(runner, &args).await;
    if !raw.success {
        return DockerResult {
            success: false,
            command,
            stdout: raw.stdout,
            stderr: raw.stderr,
            exit_code: raw.exit_code,
        };
    }

    let containers = raw
        .stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>();

    let stdout = serde_json::json!({
        "schema": "cynic.docker.container_list.v1",
        "containers": containers,
        "count": containers.len(),
    })
    .to_string();

    DockerResult {
        success: true,
        command,
        stdout,
        stderr: raw.stderr,
        exit_code: raw.exit_code,
    }
}

pub async fn run_docker_container_logs(
    runner: &dyn ArgvExecutionPort,
    container: &str,
    tail: Option<u32>,
    timestamps: bool,
) -> DockerResult {
    let mut args = vec!["logs".to_string()];
    if timestamps {
        args.push("--timestamps".to_string());
    }
    let tail = tail.unwrap_or(200);
    args.push("--tail".to_string());
    args.push(tail.to_string());
    args.push(container.to_string());
    let command = format!("docker {}", args.join(" "));
    let raw = run_docker_cmd(runner, &args).await;
    if !raw.success {
        return DockerResult {
            success: false,
            command,
            stdout: raw.stdout,
            stderr: raw.stderr,
            exit_code: raw.exit_code,
        };
    }

    DockerResult {
        success: true,
        command,
        stdout: serde_json::json!({
            "schema": "cynic.docker.container_logs.v1",
            "container": container,
            "tail": tail,
            "timestamps": timestamps,
            "logs": raw.stdout,
        })
        .to_string(),
        stderr: raw.stderr,
        exit_code: raw.exit_code,
    }
}

pub async fn run_docker_container_inspect(
    runner: &dyn ArgvExecutionPort,
    container: &str,
) -> DockerResult {
    let args = vec!["inspect".to_string(), container.to_string()];
    let command = format!("docker {}", args.join(" "));
    let raw = run_docker_cmd(runner, &args).await;
    if !raw.success {
        return DockerResult {
            success: false,
            command,
            stdout: raw.stdout,
            stderr: raw.stderr,
            exit_code: raw.exit_code,
        };
    }

    let inspection: serde_json::Value = serde_json::from_str(&raw.stdout)
        .unwrap_or_else(|_| serde_json::json!({ "raw": raw.stdout }));
    DockerResult {
        success: true,
        command,
        stdout: serde_json::json!({
            "schema": "cynic.docker.container_inspect.v1",
            "container": container,
            "inspection": inspection,
        })
        .to_string(),
        stderr: raw.stderr,
        exit_code: raw.exit_code,
    }
}

/// Collect the Docker storage evidence needed by the host census. This is an
/// observation bundle, not a normalized authority and never mutates Docker.
pub async fn run_docker_storage_observe(runner: &dyn ArgvExecutionPort) -> DockerResult {
    let mut normalized_evidence = cynic_host::container_runtime::DockerStorageEvidence::default();
    let commands = [
        vec!["info", "--format", "{{json .}}"],
        vec!["system", "df"],
        vec!["ps", "--all", "--no-trunc", "--format", "{{json .}}"],
        vec!["image", "ls", "--no-trunc", "--format", "{{json .}}"],
        vec!["volume", "ls", "--format", "{{json .}}"],
    ];
    let mut evidence = serde_json::Map::new();
    let mut failures = Vec::new();
    for args in commands {
        let key = args.join(" ");
        let owned = args
            .iter()
            .map(|argument| (*argument).to_string())
            .collect::<Vec<_>>();
        let raw = run_docker_cmd(runner, &owned).await;
        if raw.success {
            let stdout = raw.stdout;
            evidence.insert(key.clone(), Value::String(stdout.clone()));
            let value = Some(stdout);
            match key.as_str() {
                "info --format {{json .}}" => normalized_evidence.info_json = value,
                "system df" => normalized_evidence.system_df = value,
                "ps --all --no-trunc --format {{json .}}" => {
                    normalized_evidence.containers_jsonl = value
                }
                "image ls --no-trunc --format {{json .}}" => {
                    normalized_evidence.images_jsonl = value
                }
                "volume ls --format {{json .}}" => normalized_evidence.volumes_jsonl = value,
                _ => {}
            }
        } else {
            normalized_evidence.failures.push(key.clone());
            failures.push(serde_json::json!({
                "command": key,
                "exit_code": raw.exit_code,
                "stdout": raw.stdout,
                "stderr": raw.stderr,
            }));
        }
    }
    // Optional capability probe: its absence must not invalidate the Docker
    // storage census because the base Docker CLI is independently useful.
    let toolkit_args = ["mcp".to_string(), "--help".to_string()];
    let toolkit_probe = run_docker_cmd(runner, &toolkit_args).await;
    evidence.insert(
        "mcp --help".into(),
        serde_json::json!({
            "available": toolkit_probe.success,
            "stdout": toolkit_probe.stdout,
            "stderr": toolkit_probe.stderr,
            "exit_code": toolkit_probe.exit_code,
        }),
    );
    let success = failures.is_empty();
    let normalized = cynic_host::container_runtime::normalize_docker_evidence(
        &normalized_evidence,
        chrono::Utc::now().timestamp_millis().max(0) as u128,
    );
    DockerResult {
        success,
        command: "docker storage observation bundle".into(),
        stdout: serde_json::json!({
            "schema": "cynic.docker.storage_observation_bundle.v1",
            "evidence": evidence,
            "normalized": normalized,
            "failures": failures,
            "mutates": false,
            "residual_unknowns": [
                "byte attribution between shared image layers is not normalized",
                "volume ownership and persistence intent require inspect/project evidence",
                if toolkit_probe.success {
                    "Docker MCP Toolkit CLI surface is available; profile inventory is not yet normalized"
                } else {
                    "Docker MCP Toolkit CLI surface is not exposed by the installed Docker command"
                }
            ]
        })
        .to_string(),
        stderr: String::new(),
        exit_code: if success { Some(0) } else { Some(1) },
    }
}

async fn run_docker_cmd(runner: &dyn ArgvExecutionPort, args: &[String]) -> DockerCmdResult {
    let output = runner
        .execute(ArgvExecutionRequest {
            argv: std::iter::once("docker".to_string())
                .chain(args.iter().cloned())
                .collect(),
            current_dir: None,
            stdin: None,
            timeout: std::time::Duration::from_secs(30),
        })
        .await;
    DockerCmdResult {
        success: output.success(),
        stdout: output.stdout,
        stderr: output.spawn_error.unwrap_or_else(|| output.stderr),
        exit_code: output.exit_code,
    }
}
