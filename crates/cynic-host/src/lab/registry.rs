use serde_json::Value;

pub(crate) fn read_registry(registry_path: &str) -> Result<Value, String> {
    let raw = std::fs::read_to_string(registry_path)
        .map_err(|error| format!("failed to read registry {registry_path}: {error}"))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse registry {registry_path}: {error}"))
}
