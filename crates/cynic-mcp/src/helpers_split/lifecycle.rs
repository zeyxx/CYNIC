fn run_lifecycle_payload_plan(profile: &str) -> Result<serde_json::Value, McpError> {
    Ok(serde_json::json!({
        "schema": "cynic.lifecycle_payload_plan.v1.minimal",
        "status": "ok",
        "profile": profile,
        "source": "infra/registry.json",
        "payloads": [
            {
                "kind": "operator",
                "domain": "kernel_release",
                "content": serde_json::json!({
                    "intent": profile,
                    "authority": "contracts/foundation.json",
                    "registry": "infra/registry.json",
                    "required_local_gate": "make check"
                }).to_string(),
                "priority": 99,
                "consumer": "cynic-runtime",
                "escalation_policy": "minimal_registry_backed_dispatch_only"
            }
        ]
    }))
}

fn run_lifecycle_status() -> Result<serde_json::Value, McpError> {
    Ok(serde_json::json!({
        "schema": "cynic.lifecycle_status.v1.minimal",
        "status": "minimal_baseline",
        "authority": [
            "contracts/foundation.json",
            "infra/runtime-authority.json",
            "infra/registry.json"
        ],
        "local_gate": "make check",
        "runtime_artifacts": [
            "cynic-kernel",
            "cynic-node",
            "cynic-mcp",
            "cynic"
        ],
        "deleted_transport": "legacy Python lifecycle CLI"
    }))
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

#[cfg(test)]
fn lifecycle_payload_body_with_defaults(payload: &serde_json::Value) -> serde_json::Value {
    let mut body = payload.clone();
    if let Some(object) = body.as_object_mut() {
        object
            .entry("priority")
            .or_insert_with(|| serde_json::json!(99));
        object
            .entry("consumer")
            .or_insert_with(|| serde_json::json!("cynic-runtime"));
        object.entry("escalation_policy").or_insert_with(|| {
            serde_json::json!("canonical_build_runner_validation_only_no_cortex_build")
        });
    }
    body
}

#[cfg(test)]
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
