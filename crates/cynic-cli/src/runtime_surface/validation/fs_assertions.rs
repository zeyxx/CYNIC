use std::path::Path;

use serde_json::Value;

use super::steps::RuntimeValidationStep;

pub(super) fn validate_directory_contains(
    name: &'static str,
    dir: &Path,
    needle: &str,
    expectation: &'static str,
) -> RuntimeValidationStep {
    let haystack = read_directory_text(dir);
    let ok = haystack.contains(needle);
    RuntimeValidationStep {
        name,
        command: vec![
            "read-dir".to_string(),
            dir.display().to_string(),
            format!("contains:{needle}"),
        ],
        exit_code: Some(if ok { 0 } else { 1 }),
        expectation,
        status: if ok { "ok" } else { "blocked" },
        observed_schema: None,
        observed_status: None,
        detail: if ok {
            format!("found {needle}")
        } else {
            format!("missing {needle}")
        },
    }
}

pub(super) fn validate_directory_excludes(
    name: &'static str,
    dir: &Path,
    needle: &str,
    expectation: &'static str,
) -> RuntimeValidationStep {
    let haystack = read_directory_text(dir);
    let ok = !haystack.contains(needle);
    RuntimeValidationStep {
        name,
        command: vec![
            "read-dir".to_string(),
            dir.display().to_string(),
            format!("excludes:{needle}"),
        ],
        exit_code: Some(if ok { 0 } else { 1 }),
        expectation,
        status: if ok { "ok" } else { "blocked" },
        observed_schema: None,
        observed_status: None,
        detail: if ok {
            format!("did not find {needle}")
        } else {
            format!("found forbidden {needle}")
        },
    }
}

pub(super) fn registry_dns_zone(registry_path: &str) -> Result<String, String> {
    let raw = std::fs::read_to_string(registry_path)
        .map_err(|error| format!("failed to read registry {registry_path}: {error}"))?;
    let registry: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse registry {registry_path}: {error}"))?;
    registry
        .pointer("/network_policy/planes/synapse/dns_zone")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            format!("registry {registry_path} missing network_policy.planes.synapse.dns_zone")
        })
}

fn read_directory_text(dir: &Path) -> String {
    let mut output = String::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    output.push_str(&text);
                    output.push('\n');
                }
            }
        }
    }
    output
}
