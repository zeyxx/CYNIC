fn coordination_protocol_root(protocol: &serde_json::Value) -> &serde_json::Value {
    protocol.get("coordination_protocol").unwrap_or(protocol)
}

fn resolve_default_client(explicit: &Option<String>) -> Result<String, McpError> {
    explicit.clone().or_else(coordination_default_client).ok_or_else(|| {
        McpError::invalid_params(
            "coordination call requires `client` in strict mode or set CYNIC_COORDINATION_CLIENT_ID",
            None,
        )
    })
}

fn scope_allows_project(profile_scope: Option<&str>, project: &str) -> bool {
    match profile_scope {
        None => false,
        Some(scope) if scope.trim() == "*" => true,
        Some(scope) => scope
            .split(',')
            .map(|entry| entry.trim())
            .filter(|entry| !entry.is_empty())
            .any(|entry| entry == project),
    }
}

fn operation_is_allowed(profile: &serde_json::Value, operation: &str) -> bool {
    profile
        .get("allowed_operations")
        .and_then(|value| value.as_array())
        .is_some_and(|operations| {
            operations.iter().any(|value| {
                value
                    .as_str()
                    .is_some_and(|op| op.eq_ignore_ascii_case(operation))
            })
        })
}

fn project_profiles_match(profile: &serde_json::Value, project: &str) -> bool {
    scope_allows_project(
        profile
            .get("project_scope")
            .and_then(|value| value.as_str()),
        project,
    )
}

fn enforce_coordination_policy(
    explicit_client: &Option<String>,
    operation: &str,
    project: &str,
) -> Result<(), McpError> {
    let protocol = load_coordination_protocol()?;
    let permission_model = coordination_protocol_root(&protocol)
        .get("permission_model")
        .and_then(|value| value.as_object())
        .ok_or_else(|| {
            McpError::internal_error(
                "Coordination protocol missing permission_model object",
                None,
            )
        })?;

    let strict_mode = coordination_strict_mode()
        || permission_model
            .get("default_mode")
            .and_then(|value| value.as_str())
            .is_some_and(|value| value == "strict");
    if !strict_mode {
        return Ok(());
    }

    let client = resolve_default_client(explicit_client)?;
    let default_allow_unknown = permission_model
        .get("default_allow_unknown_client")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let clients = permission_model
        .get("profiles")
        .and_then(|value| value.get("clients"))
        .and_then(|value| value.as_object());
    let client_profile = clients.and_then(|map| map.get(&client));
    if client_profile.is_none() {
        if default_allow_unknown {
            return Ok(());
        }
        return Err(McpError::invalid_request(
            format!("coordination client '{client}' is not authorized"),
            None,
        ));
    }

    let client_profile = client_profile.unwrap();
    if !client_profile.is_object() {
        return Err(McpError::internal_error(
            format!("coordination client profile for '{client}' must be an object"),
            None,
        ));
    }

    if !project_profiles_match(client_profile, project) {
        return Err(McpError::invalid_request(
            format!("coordination client '{client}' is not allowed for project '{project}'",),
            None,
        ));
    }

    if !operation_is_allowed(client_profile, operation) {
        return Err(McpError::invalid_request(
            format!("coordination client '{client}' is not authorized for '{operation}'",),
            None,
        ));
    }

    Ok(())
}

fn coordination_project_profiles(project: &str) -> Option<serde_json::Value> {
    let profile = load_consumer_profile(project).ok().flatten()?;
    let coordination = profile.get("coordination")?;
    let minimum_fields = coordination.get("minimum_fields_per_event")?;
    Some(minimum_fields.clone())
}

fn coordination_protocol_path() -> String {
    std::env::var("CYNIC_COORDINATION_PROTOCOL_PATH")
        .unwrap_or_else(|_| DEFAULT_COORDINATION_PROTOCOL_PATH.to_string())
}

fn coordination_client_profile_dir() -> String {
    std::env::var("CYNIC_COORDINATION_CLIENT_PROFILE_DIR")
        .unwrap_or_else(|_| DEFAULT_COORDINATION_CLIENT_PROFILE_DIR.to_string())
}

fn coordination_strict_mode() -> bool {
    std::env::var("CYNIC_COORDINATION_STRICT")
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(true)
}

fn coordination_default_client() -> Option<String> {
    std::env::var("CYNIC_COORDINATION_CLIENT_ID").ok()
}

fn load_coordination_json(path: &str) -> Result<serde_json::Value, McpError> {
    let full_path = project_root().join(path);
    let content = std::fs::read_to_string(&full_path).map_err(|e| {
        McpError::internal_error(
            format!("Coordination contract file unavailable ({full_path:?}): {e}"),
            None,
        )
    })?;
    serde_json::from_str(&content).map_err(|e| {
        McpError::internal_error(
            format!("Coordination contract file is invalid JSON ({full_path:?}): {e}"),
            None,
        )
    })
}

fn load_coordination_protocol() -> Result<serde_json::Value, McpError> {
    load_coordination_json(&coordination_protocol_path())
}

fn contract_events_required_fields(
    protocol: &serde_json::Value,
    profile: Option<&serde_json::Value>,
    event: &str,
) -> Vec<String> {
    let mut fields = Vec::new();
    let protocol_root = coordination_protocol_root(protocol);
    if let Some(obj) = protocol_root
        .get("event_contract")
        .and_then(|value| value.get("required_fields_by_event"))
        .and_then(|value| value.get(event))
    {
        if let Some(list) = obj.as_array() {
            fields.extend(
                list.iter()
                    .filter_map(|value| value.as_str())
                    .map(String::from),
            );
        }
    }

    if let Some(list) = profile.and_then(|value| value.as_array()) {
        fields.extend(
            list.iter()
                .filter_map(|value| value.as_str())
                .map(String::from),
        );
    }

    fields.sort();
    fields.dedup();
    fields
}

fn value_is_present(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::String(text) => !text.trim().is_empty(),
        serde_json::Value::Array(items) => !items.is_empty(),
        serde_json::Value::Object(obj) => !obj.is_empty(),
        serde_json::Value::Bool(_) => true,
        serde_json::Value::Number(_) => true,
    }
}

fn flatten_payload_for_validation(
    value: &serde_json::Value,
) -> serde_json::Map<String, serde_json::Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn validate_coordination_event_requirements(
    event: &str,
    project: &str,
    payload: &serde_json::Value,
) -> Result<(), McpError> {
    let protocol = load_coordination_protocol()?;
    let profile = coordination_project_profiles(project)
        .and_then(|value| value.get("minimum_fields_per_event").cloned())
        .and_then(|minimum_fields| minimum_fields.get(event).cloned());
    let required = contract_events_required_fields(&protocol, profile.as_ref(), event);
    if required.is_empty() {
        return Ok(());
    }

    let normalized = flatten_payload_for_validation(payload);
    for field in required {
        if !normalized
            .get(&field)
            .is_some_and(|value| value_is_present(value))
        {
            return Err(McpError::invalid_params(
                format!("coordination field '{field}' is required for event '{event}'"),
                None,
            ));
        }
    }
    Ok(())
}

fn emit_coordination_event_payload(
    op: &str,
    event: &str,
    project: &str,
    workstream: &str,
    branch: Option<&str>,
    thread_id: Option<&str>,
    summary: &str,
    artifacts: Option<Vec<String>>,
    open_points: Option<Vec<String>>,
    extras: serde_json::Value,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "coordination_contract": {
            "version": COORDINATION_SCHEMA,
            "operation": op,
        },
        "project": project,
        "event": event,
        "workstream": workstream,
        "status": "ok",
        "summary": summary,
        "artifacts": artifacts.unwrap_or_default(),
        "open_points": open_points.unwrap_or_default(),
        "thread_id": thread_id,
        "branch": branch,
    });
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("extra".to_string(), extras);
    }
    payload
}

fn validate_coordination_event_name(event: &str) -> bool {
    matches!(
        event,
        "workstream.started"
            | "workstream.blocked"
            | "workstream.updated"
            | "interface.changed"
            | "run.completed"
            | "decision.recorded"
            | "thread.reply"
            | "artifact.share"
            | "decision.record"
    )
}

fn coordination_tool_call_result(rows: Vec<serde_json::Value>) -> CallToolResult {
    CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&rows).unwrap_or_else(|_| "[]".to_string()),
    )])
}

fn coordination_filter_row(
    row: &serde_json::Value,
    project: &str,
    workstream: &Option<String>,
    thread_id: &Option<String>,
) -> bool {
    let value = row.get("value").and_then(|value| value.as_object());
    let row_project = row
        .get("project")
        .and_then(|value| value.as_str())
        .or_else(|| {
            value
                .and_then(|v| v.get("project"))
                .and_then(|value| value.as_str())
        })
        .unwrap_or("");
    let row_workstream = row
        .get("workstream")
        .and_then(|value| value.as_str())
        .or_else(|| {
            value
                .and_then(|v| v.get("workstream"))
                .and_then(|value| value.as_str())
                .or_else(|| {
                    extract_workstream_from_coordination_target(
                        row.get("target").and_then(|value| value.as_str()),
                    )
                })
        })
        .unwrap_or("");
    let row_thread_id = row
        .get("thread_id")
        .and_then(|value| value.as_str())
        .or_else(|| {
            value
                .and_then(|v| v.get("thread_id"))
                .and_then(|value| value.as_str())
                .or_else(|| {
                    extract_thread_from_coordination_target(
                        row.get("target").and_then(|value| value.as_str()),
                    )
                })
        })
        .unwrap_or("");
    if row_project != project {
        return false;
    }
    if let Some(filter) = workstream
        && !filter.is_empty()
        && row_workstream != filter
    {
        return false;
    }
    if let Some(filter) = thread_id
        && !filter.is_empty()
        && row_thread_id != filter
    {
        return false;
    }
    true
}

fn extract_thread_from_coordination_target(target: Option<&str>) -> Option<&str> {
    target.and_then(|target| target.split_once('#').map(|(_, thread)| thread))
}

fn extract_workstream_from_coordination_target(target: Option<&str>) -> Option<&str> {
    let target = target?;
    let after_prefix = target.strip_prefix("coord://")?;
    let without_thread = after_prefix.split('#').next()?;
    let mut parts = without_thread.split('/');
    let _project = parts.next()?;
    parts.next().filter(|workstream| !workstream.is_empty())
}

fn project_workstream_thread_key(
    project: &str,
    workstream: &str,
    thread_id: &Option<&str>,
) -> String {
    match thread_id {
        Some(thread_id) if !thread_id.is_empty() => {
            format!("coord://{project}/{workstream}#{thread_id}")
        }
        _ => format!("coord://{project}/{workstream}"),
    }
}

fn normalize_coordination_row(row: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "observed_at": row.get("created_at").cloned().unwrap_or(serde_json::Value::Null),
        "status": row.get("status"),
        "target": row.get("target"),
        "tags": row.get("tags"),
        "value": row.get("value"),
        "domain": row.get("domain"),
    })
}
