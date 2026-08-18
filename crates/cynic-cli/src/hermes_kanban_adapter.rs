use async_trait::async_trait;
use cynic_inference::{CoordinationPort, CoordinationReceipt, TaskSpec};
use cynic_protocol::{CoordinationTaskObservation, CoordinationTaskStatus};
use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct HermesKanbanCoordinator {
    pub board: Option<String>,
}

#[async_trait]
impl CoordinationPort for HermesKanbanCoordinator {
    async fn observe_or_create(
        &self,
        correlation_id: &str,
        coordination_id: Option<&str>,
        task: &TaskSpec,
    ) -> Result<CoordinationReceipt, String> {
        let task_id = coordination_id
            .map(str::to_string)
            .unwrap_or_else(|| task.task_id.clone());
        let mut command = tokio::process::Command::new("hermes");
        command.arg("kanban");
        if let Some(board) = &self.board {
            command.args(["--board", board]);
        }
        command.args(["show", &task_id, "--json"]);
        let output = command
            .output()
            .await
            .map_err(|error| format!("kanban_observe_spawn:{error}"))?;
        if !output.status.success() {
            return self.create_task(correlation_id, task).await;
        }
        let value: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("kanban_observe_json:{error}"))?;
        let observation = parse_observation(&value, &task_id, correlation_id);
        Ok(CoordinationReceipt {
            coordination_kind: "hermes_kanban".into(),
            coordination_id: task_id,
            correlation_id: correlation_id.into(),
            observation,
        })
    }

    async fn complete(&self, receipt: &CoordinationReceipt, summary: &str) -> Result<(), String> {
        let mut command = tokio::process::Command::new("hermes");
        command.arg("kanban");
        if let Some(board) = &self.board {
            command.args(["--board", board]);
        }
        command.args(["complete", &receipt.coordination_id, "--summary", summary]);
        let output = command
            .output()
            .await
            .map_err(|error| format!("kanban_complete_spawn:{error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "kanban_complete_failed:{}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    async fn block(&self, receipt: &CoordinationReceipt, reason: &str) -> Result<(), String> {
        let mut command = tokio::process::Command::new("hermes");
        command.arg("kanban");
        if let Some(board) = &self.board {
            command.args(["--board", board]);
        }
        command
            .arg("block")
            .arg(&receipt.coordination_id)
            .arg(reason)
            .args(["--kind", "transient"]);
        let output = command
            .output()
            .await
            .map_err(|error| format!("kanban_block_spawn:{error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "kanban_block_failed:{}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    async fn heartbeat(&self, receipt: &CoordinationReceipt) -> Result<(), String> {
        let mut command = tokio::process::Command::new("hermes");
        command.arg("kanban");
        if let Some(board) = &self.board {
            command.args(["--board", board]);
        }
        command.args(["heartbeat", &receipt.coordination_id]);
        let output = command
            .output()
            .await
            .map_err(|error| format!("kanban_heartbeat_spawn:{error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "kanban_heartbeat_failed:{}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }
}

impl HermesKanbanCoordinator {
    async fn create_task(
        &self,
        correlation_id: &str,
        task: &TaskSpec,
    ) -> Result<CoordinationReceipt, String> {
        let mut command = tokio::process::Command::new("hermes");
        command.arg("kanban");
        if let Some(board) = &self.board {
            command.args(["--board", board]);
        }
        command
            .arg("create")
            .arg(&task.task_id)
            .arg("--body")
            .arg(&task.prompt)
            .arg("--idempotency-key")
            .arg(correlation_id)
            .args(["--created-by", "cynic", "--json"]);
        let output = command
            .output()
            .await
            .map_err(|error| format!("kanban_create_spawn:{error}"))?;
        if !output.status.success() {
            return Err(format!(
                "kanban_create_failed:{}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let value: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("kanban_create_json:{error}"))?;
        let task_id = value["id"]
            .as_str()
            .ok_or_else(|| "kanban_create_missing_id".to_string())?;
        Ok(CoordinationReceipt {
            coordination_kind: "hermes_kanban".into(),
            coordination_id: task_id.into(),
            correlation_id: correlation_id.into(),
            observation: parse_observation(&value, task_id, correlation_id),
        })
    }
}

fn parse_observation(
    value: &Value,
    task_id: &str,
    correlation_id: &str,
) -> CoordinationTaskObservation {
    let task = value.get("task").unwrap_or(value);
    let status = match task["status"].as_str().unwrap_or("unknown") {
        "triage" => CoordinationTaskStatus::Triage,
        "todo" => CoordinationTaskStatus::Todo,
        "ready" => CoordinationTaskStatus::Ready,
        "running" | "in_progress" => CoordinationTaskStatus::Running,
        "review" => CoordinationTaskStatus::Review,
        "done" | "completed" => CoordinationTaskStatus::Done,
        "blocked" => CoordinationTaskStatus::Blocked,
        "scheduled" => CoordinationTaskStatus::Scheduled,
        "archived" => CoordinationTaskStatus::Archived,
        _ => CoordinationTaskStatus::Unknown,
    };
    CoordinationTaskObservation {
        schema: cynic_protocol::COORDINATION_TASK_OBSERVATION_SCHEMA.into(),
        coordination_kind: "hermes_kanban".into(),
        task_id: task_id.into(),
        status,
        assignee: task["assignee"].as_str().map(str::to_string),
        run_id: task["current_run_id"]
            .as_str()
            .or_else(|| task["run_id"].as_str())
            .map(str::to_string),
        lease_expires_at: task["lease_expires_at"].as_str().map(str::to_string),
        heartbeat_at: task["heartbeat_at"].as_str().map(str::to_string),
        parent_ids: strings(&value["parent_ids"]),
        child_ids: strings(&value["child_ids"]),
        correlation_id: Some(correlation_id.into()),
        message: value["message"].as_str().map(str::to_string),
    }
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}
