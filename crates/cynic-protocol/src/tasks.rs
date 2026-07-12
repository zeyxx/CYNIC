use serde_json::{Value, json};
use std::fmt::Write;

pub const AGENT_TASKS_PATH: &str = "/agent-tasks";
pub const AGENT_TASKS_COMPLETED_PATH: &str = "/agent-tasks/completed";

pub fn agent_task_claim_path(task_id: &str) -> String {
    format!("/agent-tasks/{}/claim", percent_encode_component(task_id))
}

pub fn agent_task_claim_payload(agent_id: &str) -> Value {
    json!({ "agent_id": agent_id })
}

pub fn agent_task_result_path(task_id: &str) -> String {
    format!("/agent-tasks/{}/result", percent_encode_component(task_id))
}

pub fn agent_task_result_payload(
    success: Option<bool>,
    result: Option<Value>,
    error: Option<Value>,
    kind: Option<&str>,
    domain: Option<&str>,
) -> Value {
    let mut payload = serde_json::Map::new();
    if let Some(success) = success {
        payload.insert(
            "status".to_string(),
            Value::String(if success { "completed" } else { "failed" }.to_string()),
        );
    }
    if let Some(kind) = kind.filter(|value| !value.is_empty()) {
        payload.insert("kind".to_string(), Value::String(kind.to_string()));
    }
    if let Some(domain) = domain.filter(|value| !value.is_empty()) {
        payload.insert("domain".to_string(), Value::String(domain.to_string()));
    }
    if let Some(result) = result {
        payload.insert("result".to_string(), result_value(result));
    }
    if let Some(error) = error {
        payload.insert("error".to_string(), result_value(error));
    }
    Value::Object(payload)
}

pub fn agent_task_listing_params(
    kind: &str,
    domain: Option<&str>,
    limit: usize,
) -> Vec<(String, String)> {
    let mut params = vec![("kind".to_string(), kind.to_string())];
    if let Some(domain) = domain.filter(|value| !value.is_empty()) {
        params.push(("domain".to_string(), domain.to_string()));
    }
    params.push(("limit".to_string(), limit.to_string()));
    params
}

pub fn agent_task_listing_query(kind: &str, domain: Option<&str>, limit: usize) -> String {
    agent_task_listing_params(kind, domain, limit)
        .into_iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                percent_encode_component(&key),
                percent_encode_component(&value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

pub fn agent_task_collection_path(completed: bool) -> &'static str {
    if completed {
        AGENT_TASKS_COMPLETED_PATH
    } else {
        AGENT_TASKS_PATH
    }
}

pub fn task_id_in_agent_task_listings(task_id: &str, listings: &[Value]) -> bool {
    if task_id.is_empty() {
        return false;
    }
    listings.iter().any(|listing| {
        listing
            .get("tasks")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .any(|task| task.get("id").and_then(Value::as_str) == Some(task_id))
    })
}

fn result_value(value: Value) -> Value {
    match value {
        Value::Object(_) | Value::Array(_) => {
            Value::String(serde_json::to_string(&value).expect("json serialization should succeed"))
        }
        other => other,
    }
}

fn percent_encode_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if is_unreserved(byte) {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(&mut encoded, "%{byte:02X}");
        }
    }
    encoded
}

fn is_unreserved(byte: u8) -> bool {
    matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encodes_task_paths() {
        assert_eq!(
            agent_task_claim_path("gate 2/request&deploy"),
            "/agent-tasks/gate%202%2Frequest%26deploy/claim"
        );
        assert_eq!(
            agent_task_result_path("gate 2/request&deploy"),
            "/agent-tasks/gate%202%2Frequest%26deploy/result"
        );
    }

    #[test]
    fn listing_query_matches_python_shape() {
        let query = agent_task_listing_query("hermes", Some("gate 2/request&deploy"), 10);
        assert_eq!(
            query,
            "kind=hermes&domain=gate%202%2Frequest%26deploy&limit=10"
        );
    }

    #[test]
    fn serializes_object_result_values_as_compact_json_strings() {
        let payload = agent_task_result_payload(
            Some(true),
            Some(json!({"ok": true})),
            None,
            Some("hermes"),
            Some("organ-forge"),
        );
        assert_eq!(payload["status"], "completed");
        assert_eq!(payload["kind"], "hermes");
        assert_eq!(payload["domain"], "organ-forge");
        assert_eq!(payload["result"], "{\"ok\":true}");
    }

    #[test]
    fn detects_task_id_in_listing_payload() {
        let listings = vec![json!({
            "tasks": [
                {"id": "task-1"},
                {"id": "task-2"}
            ]
        })];
        assert!(task_id_in_agent_task_listings("task-2", &listings));
        assert!(!task_id_in_agent_task_listings("task-3", &listings));
    }
}
