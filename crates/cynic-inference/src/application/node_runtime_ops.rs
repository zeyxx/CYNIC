use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DogNodeBinding {
    pub dog: String,
    pub node: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DogOperationOutcome {
    pub node: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelProbeResult {
    pub actual_model: String,
    pub reachable: bool,
    pub latency_ms: u64,
    pub failure_reason: String,
}

pub fn dog_to_node(dog: &str, bindings: &[DogNodeBinding]) -> String {
    for binding in bindings {
        if binding.dog == dog {
            return binding.node.clone();
        }
    }

    if dog.contains("qwen-72b") || dog.contains("nemotron") {
        "cynic-host-gpu".to_string()
    } else {
        "cynic-vm-anvil".to_string()
    }
}

pub async fn attempt_node_recovery(node: &str) -> String {
    run_ts_exec(node, "systemctl --user restart llama-server").await
}

pub async fn attempt_node_start(node: &str) -> String {
    run_ts_exec(node, "systemctl start llama-server").await
}

async fn run_ts_exec(node: &str, command: &str) -> String {
    let timeout_duration = std::time::Duration::from_secs(30);
    let script_path = std::env::var("CYNIC_SCRIPTS").unwrap_or_else(|_| "scripts".to_string())
        + "/ts_exec_call.sh";

    let mut cmd = tokio::process::Command::new(&script_path);
    cmd.arg(node).arg(command).arg("30");

    let result = tokio::time::timeout(
        timeout_duration + std::time::Duration::from_secs(5),
        cmd.output(),
    )
    .await;

    match result {
        Err(_) => "timed_out".to_string(),
        Ok(Err(_)) => "failed".to_string(),
        Ok(Ok(output)) => {
            if let Ok(response_text) = String::from_utf8(output.stdout)
                && let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text)
                && let Some(exit_code) = json.get("exit_code").and_then(|value| value.as_i64())
                && exit_code == 0
            {
                return "succeeded".to_string();
            }
            "failed".to_string()
        }
    }
}

pub async fn probe_node_model(node: &str, expected_model: &str) -> ModelProbeResult {
    let url = format!("http://{node}:8080/health");
    let timeout = std::time::Duration::from_secs(5);

    let start = Instant::now();

    let Ok(client) = reqwest::Client::builder().timeout(timeout).build() else {
        return ModelProbeResult {
            actual_model: "N/A".to_string(),
            reachable: false,
            latency_ms: 0,
            failure_reason: "client_init_failed".to_string(),
        };
    };

    let response = match tokio::time::timeout(timeout, client.get(&url).send()).await {
        Err(_) => {
            return ModelProbeResult {
                actual_model: "N/A".to_string(),
                reachable: false,
                latency_ms: 0,
                failure_reason: "timeout".to_string(),
            };
        }
        Ok(Err(_)) => {
            return ModelProbeResult {
                actual_model: "N/A".to_string(),
                reachable: false,
                latency_ms: 0,
                failure_reason: "unreachable".to_string(),
            };
        }
        Ok(Ok(response)) => response,
    };

    let latency_ms = start.elapsed().as_millis() as u64;

    let Ok(body) = response.text().await else {
        return ModelProbeResult {
            actual_model: "N/A".to_string(),
            reachable: true,
            latency_ms,
            failure_reason: "parse_error".to_string(),
        };
    };

    let actual_model = match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(json) => json
            .get("model")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string(),
        Err(_) => "parse_error".to_string(),
    };

    let failure_reason = if actual_model == "parse_error" {
        "parse_error".to_string()
    } else if expected_model != "N/A" && actual_model != expected_model && actual_model != "unknown"
    {
        "mismatch".to_string()
    } else {
        "none".to_string()
    };

    ModelProbeResult {
        actual_model,
        reachable: true,
        latency_ms,
        failure_reason,
    }
}

pub fn to_outcome(node: &str, status: String) -> DogOperationOutcome {
    DogOperationOutcome {
        node: node.to_string(),
        status,
    }
}

pub async fn start_dog_runtime(node: &str) -> DogOperationOutcome {
    DogOperationOutcome {
        node: node.to_string(),
        status: attempt_node_start(node).await,
    }
}

pub async fn remediate_dog_runtime(node: &str) -> DogOperationOutcome {
    DogOperationOutcome {
        node: node.to_string(),
        status: attempt_node_recovery(node).await,
    }
}

pub async fn probe_model_runtime(node: &str, expected_model: &str) -> ModelProbeResult {
    probe_node_model(node, expected_model).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dog_to_node_prefers_explicit_binding() {
        let bindings = vec![DogNodeBinding {
            dog: "qwen2.5".to_string(),
            node: "node-a".to_string(),
        }];

        assert_eq!(dog_to_node("qwen2.5", &bindings), "node-a");
    }

    #[test]
    fn dog_to_node_falls_back_for_known_large_model() {
        assert_eq!(dog_to_node("qwen-72b-x", &[]), "cynic-host-gpu");
    }

    #[test]
    fn outcome_builds() {
        let outcome = DogOperationOutcome {
            node: "n1".to_string(),
            status: "ok".to_string(),
        };
        assert_eq!(outcome.node, "n1");
        assert_eq!(outcome.status, "ok");
    }
}
