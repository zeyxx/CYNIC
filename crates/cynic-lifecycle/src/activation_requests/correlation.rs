use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub fn activation_request_key_from_content(content: &Value) -> Result<String, String> {
    let action = content
        .get("action")
        .ok_or_else(|| "payload.content.action missing".to_string())?;
    let canonical = json!({
        "lifecycle": content.get("lifecycle"),
        "action": action,
        "targets": content.get("targets").cloned().unwrap_or_else(|| json!([])),
        "coord_targets": content.get("coord_targets").cloned().unwrap_or_else(|| json!([]))
    });
    serde_json::to_string(&canonical).map_err(|err| err.to_string())
}

pub fn activation_correlation_id_from_content(content: &Value) -> Result<String, String> {
    let key = activation_request_key_from_content(content)?;
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let digest = hasher.finalize();
    let hex = hex::encode(digest);
    Ok(format!("activation:{}", &hex[..16]))
}
