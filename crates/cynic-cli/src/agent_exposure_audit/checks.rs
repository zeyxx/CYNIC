use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::agent_exposure_audit::types::AgentExposureFinding;

pub(crate) const DOCS_QUARANTINE_MARKERS: &[&str] = &[
    "Not runtime authority",
    "quarantined historical",
    "do not treat files under `docs/` as law",
];

pub(crate) const KERNEL_DOMAINS_QUARANTINE_MARKERS: &[&str] = &[
    "product-domain residual",
    "Not low-layer runtime law",
    "product_domain_residual",
];

pub(crate) fn finding(
    severity: &'static str,
    kind: &'static str,
    path: &str,
    detail: impl Into<String>,
    tool: Option<String>,
) -> AgentExposureFinding {
    AgentExposureFinding {
        severity,
        kind,
        path: path.to_string(),
        detail: detail.into(),
        tool,
    }
}

pub(crate) fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(crate) fn check_no_machine_local_paths(
    path: &Path,
    content: &str,
    findings: &mut Vec<AgentExposureFinding>,
) {
    if content.contains("/home/") || content.contains("/Users/") {
        findings.push(finding(
            "critical",
            "absolute_machine_path",
            &path_rel(path),
            "committed agent projection must not embed machine-local home paths",
            None,
        ));
    }
    if content
        .lines()
        .any(|line| line.trim_start().starts_with("[mcp_servers.cynic.tools."))
    {
        findings.push(finding(
            "critical",
            "codex_manual_tool_projection",
            codex_config_rel,
            "Codex must discover the canonical MCP tool set at runtime; per-tool approval tables are a duplicated grant surface",
            None,
        ));
    }
}

pub(crate) fn check_codex_projection(
    root: &Path,
    codex_config_rel: &str,
    findings: &mut Vec<AgentExposureFinding>,
) {
    let path = root.join(codex_config_rel);
    let Ok(content) = fs::read_to_string(&path) else {
        findings.push(finding(
            "critical",
            "codex_projection_missing",
            codex_config_rel,
            "Codex MCP projection file is missing",
            None,
        ));
        return;
    };
    if content.contains("/home/") || content.contains("/Users/") {
        findings.push(finding(
            "critical",
            "absolute_machine_path",
            codex_config_rel,
            "Codex MCP env must not commit machine-local absolute paths; resolve project root at runtime",
            None,
        ));
    }
}

pub(crate) fn check_mcp_command_projection(
    root: &Path,
    codex_config_rel: &str,
    exposure: &Value,
    exposure_rel: &str,
    findings: &mut Vec<AgentExposureFinding>,
) {
    let command = exposure
        .pointer("/mcp_servers/0/command")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if command.is_empty() {
        findings.push(finding(
            "critical",
            "mcp_command_missing",
            exposure_rel,
            "CYNIC MCP desired command is missing",
            None,
        ));
        return;
    }
    if command.contains('/') || command.starts_with('.') {
        findings.push(finding(
            "critical",
            "mcp_command_not_path_agnostic",
            exposure_rel,
            "CYNIC MCP must resolve an installed runtime-basket binary from PATH",
            Some(command.to_string()),
        ));
    }
    let config = fs::read_to_string(root.join(codex_config_rel)).unwrap_or_default();
    let expected = format!("command = \"{command}\"");
    if !config.lines().any(|line| line.trim() == expected) {
        findings.push(finding(
            "critical",
            "codex_mcp_command_drift",
            codex_config_rel,
            "Codex projection does not use the command declared by agent exposure",
            Some(command.to_string()),
        ));
    }

    let antigravity_rel = "infra/agent-projections/antigravity-mcp.json";
    let antigravity_path = root.join(antigravity_rel);
    let Ok(antigravity_bytes) = fs::read(&antigravity_path) else {
        findings.push(finding(
            "critical",
            "antigravity_projection_missing",
            antigravity_rel,
            "Antigravity must use the canonical runtime-discovery MCP projection",
            None,
        ));
        return;
    };
    let Ok(antigravity) = serde_json::from_slice::<Value>(&antigravity_bytes) else {
        findings.push(finding(
            "critical",
            "antigravity_projection_invalid",
            antigravity_rel,
            "Antigravity MCP projection must be valid JSON",
            None,
        ));
        return;
    };
    let projection_matches = antigravity.get("command").and_then(Value::as_str) == Some(command)
        && antigravity.get("tool_source").and_then(Value::as_str) == Some("runtime_discovery")
        && antigravity.get("tool_cache").and_then(Value::as_str) == Some("forbidden")
        && antigravity.get("exposure_ssot").and_then(Value::as_str) == Some(exposure_rel);
    if !projection_matches {
        findings.push(finding(
            "critical",
            "antigravity_projection_drift",
            antigravity_rel,
            "Antigravity must point at the canonical MCP command and discover tools at runtime without a duplicated cache",
            Some(command.to_string()),
        ));
    }
}

pub(crate) fn check_session_handshake(
    exposure: &Value,
    exposure_rel: &str,
    findings: &mut Vec<AgentExposureFinding>,
) {
    let handshake = exposure.get("session_handshake");
    let required_sequence = handshake
        .and_then(|value| value.get("required_sequence"))
        .and_then(Value::as_array);
    let contract_id = handshake
        .and_then(|value| value.get("contract_id"))
        .and_then(Value::as_str);
    let contract_revision = handshake
        .and_then(|value| value.get("contract_revision"))
        .and_then(Value::as_u64);
    let required_capabilities = handshake
        .and_then(|value| value.get("required_capabilities"))
        .and_then(Value::as_array);
    let incompatible_status = handshake
        .and_then(|value| value.get("incompatible_status"))
        .and_then(Value::as_str);
    let unknown_status = handshake
        .and_then(|value| value.get("unknown_status"))
        .and_then(Value::as_str);
    let validation_role = handshake
        .and_then(|value| value.get("validation_execution_role"))
        .and_then(Value::as_str);
    let shell_fallback = handshake
        .and_then(|value| value.get("shell_build_fallback"))
        .and_then(Value::as_bool);
    let workload_contract = handshake
        .and_then(|value| value.get("mutation_execution_contract"))
        .and_then(Value::as_str);
    let scheduler_status = handshake
        .and_then(|value| value.get("recursive_scheduler_status"))
        .and_then(Value::as_str);
    let legacy_task_authority = handshake
        .and_then(|value| value.get("legacy_agent_task_runtime_authority"))
        .and_then(Value::as_bool);
    let constitution = exposure.get("constitutional_execution");
    let constitution_contract = constitution
        .and_then(|value| value.get("execution_contract"))
        .and_then(Value::as_str);
    let constitution_status = constitution
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str);
    let legacy_queue_authority = constitution
        .and_then(|value| value.pointer("/legacy_agent_task/runtime_authority"))
        .and_then(Value::as_bool);
    let missing_runtime_capabilities = constitution
        .and_then(|value| value.get("missing_runtime_capabilities"))
        .and_then(Value::as_array);
    if contract_id != Some("cynic.agent_mcp")
        || contract_revision.is_none_or(|revision| revision == 0)
        || required_capabilities.is_none_or(|items| items.is_empty())
        || incompatible_status != Some("blocked_incompatible")
        || unknown_status != Some("blocked_unknown_compatibility")
        || required_sequence.is_none_or(|items| items.is_empty())
        || validation_role != Some("build_runner")
        || shell_fallback != Some(false)
        || workload_contract != Some("cynic.agent_workload_envelope.v1")
        || scheduler_status != Some("inactive")
        || legacy_task_authority != Some(false)
        || constitution_contract != Some("cynic.agent_workload_envelope.v1")
        || constitution_status != Some("inactive")
        || legacy_queue_authority != Some(false)
        || missing_runtime_capabilities.is_none_or(|items| items.is_empty())
    {
        findings.push(finding(
            "critical",
            "agent_session_handshake_incomplete",
            exposure_rel,
            "agent handshake must declare a capability-based compatibility contract, startup tools, build_runner planning, the inactive constitutional scheduler, and forbid legacy queue or shell execution authority",
            None,
        ));
    }
}

pub(crate) fn check_agent_env_paths(
    exposure: &Value,
    exposure_rel: &str,
    findings: &mut Vec<AgentExposureFinding>,
) {
    let Some(env_obj) = exposure
        .pointer("/mcp_servers/0/env")
        .and_then(Value::as_object)
    else {
        return;
    };
    for (key, value) in env_obj {
        if let Some(text) = value.as_str() {
            if is_machine_local_path(text) {
                findings.push(finding(
                    "critical",
                    "absolute_machine_path",
                    exposure_rel,
                    format!("agent env entry '{key}' carries machine-local absolute path '{text}'"),
                    None,
                ));
            }
        }
    }
}

pub(crate) fn check_repo_access_profile(
    root: &Path,
    repo_access_rel: &str,
    findings: &mut Vec<AgentExposureFinding>,
) {
    let path = root.join(repo_access_rel);
    let Ok(content) = fs::read_to_string(&path) else {
        return;
    };
    if content.contains("/home/") || content.contains("/Users/") {
        findings.push(finding(
            "critical",
            "absolute_machine_path",
            repo_access_rel,
            "repo-access profile must use portable relative paths or runtime-resolved roots, not committed home paths",
            None,
        ));
    }
}

pub(crate) fn check_docs_quarantine(
    root: &Path,
    docs_readme_rel: &str,
    findings: &mut Vec<AgentExposureFinding>,
) {
    check_quarantine_readme(
        root,
        docs_readme_rel,
        DOCS_QUARANTINE_MARKERS,
        "docs_quarantine_missing",
        "docs_quarantine_markers_missing",
        "docs/README.md must exist and declare docs as non-runtime-authority quarantine",
        findings,
    );
}

pub(crate) fn check_kernel_domains_quarantine(
    root: &Path,
    kernel_domains_readme_rel: &str,
    findings: &mut Vec<AgentExposureFinding>,
) {
    check_quarantine_readme(
        root,
        kernel_domains_readme_rel,
        KERNEL_DOMAINS_QUARANTINE_MARKERS,
        "kernel_domains_quarantine_missing",
        "kernel_domains_quarantine_markers_missing",
        "crates/cynic-kernel/domains/README.md must mark product domain prompts as non-runtime law",
        findings,
    );
}

fn check_quarantine_readme(
    root: &Path,
    relative: &str,
    markers: &[&str],
    missing_kind: &'static str,
    markers_kind: &'static str,
    missing_detail: &str,
    findings: &mut Vec<AgentExposureFinding>,
) {
    let path = root.join(relative);
    let Ok(content) = fs::read_to_string(&path) else {
        findings.push(finding(
            "critical",
            missing_kind,
            relative,
            missing_detail,
            None,
        ));
        return;
    };
    let lowered = content.to_ascii_lowercase();
    let missing: Vec<&str> = markers
        .iter()
        .copied()
        .filter(|marker| !lowered.contains(&marker.to_ascii_lowercase()))
        .collect();
    if !missing.is_empty() {
        findings.push(finding(
            "critical",
            markers_kind,
            relative,
            format!("quarantine README missing markers: {}", missing.join(", ")),
            None,
        ));
    }
}

fn is_machine_local_path(value: &str) -> bool {
    value.starts_with("/home/") || value.starts_with("/Users/")
}

fn path_rel(path: &Path) -> String {
    let root = repo_root();
    path.strip_prefix(&root)
        .map(|rel| rel.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}
