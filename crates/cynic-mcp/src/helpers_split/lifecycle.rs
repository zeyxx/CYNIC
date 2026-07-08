fn lifecycle_cli_path() -> PathBuf {
    project_root().join(
        std::env::var("CYNIC_LIFECYCLE_CLI_PATH")
            .unwrap_or_else(|_| "organs/forge/src/entrypoints/cli.py".to_string()),
    )
}

fn lifecycle_cli_command() -> String {
    std::env::var("CYNIC_LIFECYCLE_CLI_CMD").unwrap_or_else(|_| "python3".to_string())
}

fn run_lifecycle_payload_plan(profile: &str) -> Result<serde_json::Value, McpError> {
    let output = std::process::Command::new(lifecycle_cli_command())
        .arg(lifecycle_cli_path())
        .arg("lifecycle-profile-payloads")
        .arg(profile)
        .arg("organ-forge")
        .arg("organ-forge-request")
        .output()
        .map_err(|e| {
            McpError::internal_error(
                format!(
                    "Failed to run lifecycle CLI command ({}) {}: {e}",
                    lifecycle_cli_command(),
                    lifecycle_cli_path().display()
                ),
                None,
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(McpError::internal_error(
            format!("Lifecycle payload generation failed: {stderr}"),
            None,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).map_err(|e| {
        McpError::internal_error(format!("Lifecycle payload output was not JSON: {e}"), None)
    })
}

fn run_lifecycle_status() -> Result<serde_json::Value, McpError> {
    let output = std::process::Command::new(lifecycle_cli_command())
        .arg(lifecycle_cli_path())
        .arg("lifecycle-status")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .map_err(|e| {
            McpError::internal_error(
                format!(
                    "Failed to run lifecycle CLI command ({}) {}: {e}",
                    lifecycle_cli_command(),
                    lifecycle_cli_path().display()
                ),
                None,
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(McpError::internal_error(
            format!("Lifecycle status failed: {stderr}"),
            None,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).map_err(|e| {
        McpError::internal_error(format!("Lifecycle status output was not JSON: {e}"), None)
    })
}

fn lifecycle_payloads_from_plan(
    payload_plan: &serde_json::Value,
) -> Result<&Vec<serde_json::Value>, McpError> {
    if payload_plan.get("status").and_then(|value| value.as_str()) != Some("ok") {
        return Err(McpError::internal_error(
            format!("Lifecycle payload generation did not return ok: {payload_plan}"),
            None,
        ));
    }
    let payloads = payload_plan
        .get("payloads")
        .and_then(|value| value.as_array())
        .ok_or_else(|| McpError::internal_error("Lifecycle payloads missing array", None))?;
    if payloads.is_empty() {
        return Err(McpError::internal_error(
            "Lifecycle profile produced zero payloads",
            None,
        ));
    }
    Ok(payloads)
}

fn lifecycle_payload_dry_run_result(mut payload_plan: serde_json::Value) -> serde_json::Value {
    if let Some(object) = payload_plan.as_object_mut() {
        object.insert("dry_run".to_string(), serde_json::json!(true));
        object.insert("mutates_mempool".to_string(), serde_json::json!(false));
    }
    payload_plan
}

fn lifecycle_payload_domain(payloads: &[serde_json::Value], fallback_profile: &str) -> String {
    payloads
        .first()
        .and_then(|payload| payload.get("domain"))
        .and_then(|value| value.as_str())
        .unwrap_or(fallback_profile)
        .to_string()
}

fn lifecycle_payload_body_with_defaults(payload: &serde_json::Value) -> serde_json::Value {
    let mut body = payload.clone();
    if let Some(object) = body.as_object_mut() {
        object
            .entry("priority")
            .or_insert_with(|| serde_json::json!(99));
        object
            .entry("consumer")
            .or_insert_with(|| serde_json::json!("organ-forge"));
        object.entry("escalation_policy").or_insert_with(|| {
            serde_json::json!("canonical_build_runner_validation_only_no_cortex_build")
        });
    }
    body
}

fn lifecycle_payload_intent(payload_body: &serde_json::Value) -> Option<serde_json::Value> {
    payload_body
        .get("content")
        .and_then(|content| content.as_str())
        .and_then(|content| serde_json::from_str::<serde_json::Value>(content).ok())
        .and_then(|content| content.get("intent").cloned())
}

fn task_ids_from_task_listing(listing: &serde_json::Value) -> Vec<String> {
    listing
        .get("tasks")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|task| {
            task.get("id")
                .and_then(|value| value.as_str())
                .map(String::from)
        })
        .collect()
}

fn visible_dispatched_task_ids(
    dispatched: &[serde_json::Value],
    pending: &serde_json::Value,
    completed: &serde_json::Value,
) -> (Vec<String>, Vec<String>) {
    let visible_ids: std::collections::BTreeSet<String> = task_ids_from_task_listing(pending)
        .into_iter()
        .chain(task_ids_from_task_listing(completed))
        .collect();
    let dispatched_ids: Vec<String> = dispatched
        .iter()
        .filter_map(|task| {
            task.get("task_id")
                .and_then(|value| value.as_str())
                .map(String::from)
        })
        .collect();
    let visible_dispatched_ids: Vec<String> = dispatched_ids
        .iter()
        .filter(|task_id| visible_ids.contains(*task_id))
        .cloned()
        .collect();
    (dispatched_ids, visible_dispatched_ids)
}

fn missing_dispatched_task_ids(dispatched_ids: &[String], visible_ids: &[String]) -> Vec<String> {
    let visible: std::collections::BTreeSet<&String> = visible_ids.iter().collect();
    dispatched_ids
        .iter()
        .filter(|task_id| !visible.contains(*task_id))
        .cloned()
        .collect()
}

fn dispatch_agent_task_body(p: &DispatchAgentTaskParams) -> serde_json::Value {
    serde_json::json!({
        "kind": p.kind,
        "domain": p.domain,
        "content": p.content,
        "agent_id": p.agent_id,
        "due_at": p.due_at,
        "priority": p.priority,
        "consumer": p.consumer,
        "escalation_policy": p.escalation_policy,
    })
}

fn verified_agent_task_dispatch_result(
    response: &serde_json::Value,
    kind: &str,
    domain: &str,
    pending: &serde_json::Value,
    completed: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let task_id = response
        .get("task_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| format!("agent task dispatch response missing task_id: {response}"))?;
    let dispatched = vec![serde_json::json!({"task_id": task_id})];
    let (dispatched_ids, visible_dispatched_ids) =
        visible_dispatched_task_ids(&dispatched, pending, completed);
    let missing_dispatched_ids =
        missing_dispatched_task_ids(&dispatched_ids, &visible_dispatched_ids);
    if !missing_dispatched_ids.is_empty() {
        return Err(format!(
            "agent task dispatch read-back incomplete; missing_dispatched_ids={missing_dispatched_ids:?}, visible_dispatched_ids={visible_dispatched_ids:?}, kind={kind}, domain={domain}"
        ));
    }
    Ok(serde_json::json!({
        "status": "dispatched",
        "kind": kind,
        "domain": domain,
        "task_id": task_id,
        "visible_dispatched_ids": visible_dispatched_ids,
        "missing_dispatched_ids": missing_dispatched_ids,
        "kernel_response": response,
        "pending_count": pending.get("count"),
        "completed_count": completed.get("count"),
    }))
}

fn verified_agent_task_completion_result(
    response: &serde_json::Value,
    task_id: &str,
    kind: &str,
    domain: Option<&str>,
    completed: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let visible_completed_ids = task_ids_from_task_listing(completed);
    if !visible_completed_ids.iter().any(|id| id == task_id) {
        return Err(format!(
            "agent task result read-back incomplete; task_id={task_id}, visible_completed_ids={visible_completed_ids:?}, kind={kind}, domain={domain:?}"
        ));
    }
    Ok(serde_json::json!({
        "status": "completed",
        "task_id": task_id,
        "kind": kind,
        "domain": domain,
        "read_back_verified": true,
        "visible_completed_ids": visible_completed_ids,
        "kernel_response": response,
        "completed_count": completed.get("count"),
    }))
}

fn observe_request_body(p: &ObserveParams) -> serde_json::Value {
    serde_json::json!({
        "tool": p.tool,
        "target": p.target,
        "domain": p.domain,
        "status": p.status,
        "context": p.context,
        "project": p.project,
        "agent_id": p.agent_id,
        "session_id": p.session_id,
        "tags": p.tags,
        "value": p.value,
        "confidence": p.confidence,
        "consumer": p.consumer,
        "action": p.action,
        "depends_on": p.depends_on,
        "maturity": p.maturity,
    })
}
