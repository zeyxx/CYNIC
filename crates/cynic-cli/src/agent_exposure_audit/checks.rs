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
