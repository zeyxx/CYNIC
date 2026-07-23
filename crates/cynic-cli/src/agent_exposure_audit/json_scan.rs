use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde_json::Value;

pub(crate) fn load_json(path: &Path) -> Result<Value, String> {
    let content = fs::read_to_string(path).map_err(|err| format!("cannot read: {err}"))?;
    serde_json::from_str(&content).map_err(|err| format!("cannot parse JSON: {err}"))
}

pub(crate) fn exposed_tools(exposure: &Value) -> BTreeSet<String> {
    exposure
        .get("mcp_servers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|server| server.get("name").and_then(Value::as_str) == Some("cynic"))
        .and_then(|server| server.get("tools"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect()
}

pub(crate) fn domain_tools(exposure: &Value) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    let Some(domains) = exposure.get("mcp_tool_domains").and_then(Value::as_object) else {
        return out;
    };
    for (name, payload) in domains {
        let tools = payload
            .get("tools")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect();
        out.insert(name.clone(), tools);
    }
    out
}

pub(crate) fn count_tiers(exposure: &Value) -> (usize, usize, usize, usize) {
    let mut baseline = 0usize;
    let mut product = 0usize;
    let mut operator = 0usize;
    let mut legacy = 0usize;
    let Some(tools) = exposure
        .get("mcp_servers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|server| server.get("name").and_then(Value::as_str) == Some("cynic"))
        .and_then(|server| server.get("tools"))
        .and_then(Value::as_array)
    else {
        return (0, 0, 0, 0);
    };
    for tool in tools {
        match tool.get("tier").and_then(Value::as_str).unwrap_or("") {
            "baseline_runtime" => baseline += 1,
            "product_domain" => product += 1,
            "operator_local" => operator += 1,
            "legacy_breakglass" => legacy += 1,
            _ => {}
        }
    }
    (baseline, product, operator, legacy)
}
