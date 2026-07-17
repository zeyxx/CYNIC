use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

const SCHEMA: &str = "cynic.cli.agent_exposure_audit.v1";
const EXPOSURE_REL: &str = "infra/agent-exposure.json";
const DOCS_README_REL: &str = "docs/README.md";
const KERNEL_DOMAINS_README_REL: &str = "crates/cynic-kernel/domains/README.md";
const CODEX_CONFIG_REL: &str = ".codex/config.toml";
const REPO_ACCESS_REL: &str = "infra/repo-access-profile.json";
const MCP_SRC_REL: &str = "crates/cynic-mcp/src";

const DOCS_QUARANTINE_MARKERS: &[&str] = &[
    "Not runtime authority",
    "quarantined historical",
    "do not treat files under `docs/` as law",
];

const KERNEL_DOMAINS_QUARANTINE_MARKERS: &[&str] = &[
    "product-domain residual",
    "Not low-layer runtime law",
    "product_domain_residual",
];

#[derive(Debug, Serialize)]
pub(crate) struct AgentExposureAuditReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) rule: &'static str,
    pub(crate) exposure_path: String,
    pub(crate) exposed_tool_count: usize,
    pub(crate) source_tool_count: usize,
    pub(crate) domain_count: usize,
    pub(crate) baseline_runtime_count: usize,
    pub(crate) product_domain_count: usize,
    pub(crate) operator_local_count: usize,
    pub(crate) legacy_breakglass_count: usize,
    pub(crate) ok_count: usize,
    pub(crate) blocked_count: usize,
    pub(crate) findings: Vec<AgentExposureFinding>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AgentExposureFinding {
    pub(crate) severity: &'static str,
    pub(crate) kind: &'static str,
    pub(crate) path: String,
    pub(crate) detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool: Option<String>,
}

pub(crate) fn audit_agent_exposure() -> AgentExposureAuditReport {
    let root = repo_root();
    let mut findings = Vec::new();

    let exposure_path = root.join(EXPOSURE_REL);
    let exposure = match load_json(&exposure_path) {
        Ok(value) => value,
        Err(detail) => {
            findings.push(finding(
                "critical",
                "agent_exposure_unreadable",
                EXPOSURE_REL,
                detail,
                None,
            ));
            return report(&root, &findings, 0, 0, 0, 0, 0, 0, 0);
        }
    };

    check_no_machine_local_paths(&exposure_path, &exposure.to_string(), &mut findings);
    check_codex_projection(&root, &mut findings);
    check_repo_access_profile(&root, &mut findings);
    check_docs_quarantine(&root, &mut findings);
    check_kernel_domains_quarantine(&root, &mut findings);

    let exposed = exposed_tools(&exposure);
    let domains = domain_tools(&exposure);
    let source_tools = scan_mcp_tool_names(&root.join(MCP_SRC_REL));

    if domains.is_empty() {
        findings.push(finding(
            "critical",
            "mcp_tool_domains_missing",
            EXPOSURE_REL,
            "agent exposure SSOT must classify active cynic MCP tools under mcp_tool_domains",
            None,
        ));
    }

    if exposed.is_empty() {
        findings.push(finding(
            "critical",
            "mcp_server_tools_missing",
            EXPOSURE_REL,
            "mcp_servers[cynic].tools must list every exposed tool name",
            None,
        ));
    }

    let domain_union: BTreeSet<String> = domains.values().flatten().cloned().collect();

    for tool in exposed.difference(&domain_union) {
        findings.push(finding(
            "critical",
            "exposed_tool_missing_domain",
            EXPOSURE_REL,
            "tool is listed under mcp_servers but missing from mcp_tool_domains",
            Some(tool.clone()),
        ));
    }
    for tool in domain_union.difference(&exposed) {
        findings.push(finding(
            "critical",
            "domain_tool_not_exposed",
            EXPOSURE_REL,
            "tool is classified in mcp_tool_domains but missing from mcp_servers tools list",
            Some(tool.clone()),
        ));
    }

    for tool in source_tools.difference(&exposed) {
        findings.push(finding(
            "critical",
            "source_tool_not_in_exposure",
            MCP_SRC_REL,
            "MCP #[tool] name is implemented in source but not declared in agent-exposure.json",
            Some(tool.clone()),
        ));
    }
    for tool in exposed.difference(&source_tools) {
        findings.push(finding(
            "critical",
            "exposure_tool_not_in_source",
            EXPOSURE_REL,
            "agent-exposure declares a tool that has no matching #[tool(name=...)] in cynic-mcp source",
            Some(tool.clone()),
        ));
    }

    if exposure
        .pointer("/mcp_servers/0/env/CYNIC_PROJECT_ROOT")
        .and_then(Value::as_str)
        .is_some_and(is_machine_local_path)
    {
        findings.push(finding(
            "critical",
            "absolute_machine_path",
            EXPOSURE_REL,
            "CYNIC_PROJECT_ROOT must not be a committed machine-local absolute path; inject at runtime",
            None,
        ));
    }

    let tier_counts = count_tiers(&exposure);
    report(
        &root,
        &findings,
        exposed.len(),
        source_tools.len(),
        domains.len(),
        tier_counts.0,
        tier_counts.1,
        tier_counts.2,
        tier_counts.3,
    )
}

fn report(
    root: &Path,
    findings: &[AgentExposureFinding],
    exposed_tool_count: usize,
    source_tool_count: usize,
    domain_count: usize,
    baseline_runtime_count: usize,
    product_domain_count: usize,
    operator_local_count: usize,
    legacy_breakglass_count: usize,
) -> AgentExposureAuditReport {
    let blocked_count = findings
        .iter()
        .filter(|finding| finding.severity == "critical")
        .count();
    let ok_count = if blocked_count == 0 { 1 } else { 0 };
    AgentExposureAuditReport {
        schema: SCHEMA,
        status: if blocked_count == 0 { "ok" } else { "blocked" },
        rule: "agent-exposure.json must faithfully list and domain-classify every cynic-mcp #[tool], avoid machine-local paths, and keep docs quarantined as non-authority",
        exposure_path: root.join(EXPOSURE_REL).display().to_string(),
        exposed_tool_count,
        source_tool_count,
        domain_count,
        baseline_runtime_count,
        product_domain_count,
        operator_local_count,
        legacy_breakglass_count,
        ok_count,
        blocked_count,
        findings: findings.to_vec(),
    }
}

fn finding(
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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_json(path: &Path) -> Result<Value, String> {
    let content = fs::read_to_string(path).map_err(|err| format!("cannot read: {err}"))?;
    serde_json::from_str(&content).map_err(|err| format!("cannot parse JSON: {err}"))
}

fn exposed_tools(exposure: &Value) -> BTreeSet<String> {
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

fn domain_tools(exposure: &Value) -> BTreeMap<String, BTreeSet<String>> {
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

fn count_tiers(exposure: &Value) -> (usize, usize, usize, usize) {
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

fn scan_mcp_tool_names(src_root: &Path) -> BTreeSet<String> {
    let mut tools = BTreeSet::new();
    let Ok(entries) = collect_rust_files(src_root) else {
        return tools;
    };
    for path in entries {
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        tools.extend(extract_tool_names(&source));
    }
    tools
}

fn collect_rust_files(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
        if !dir.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out)?;
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                out.push(path);
            }
        }
        Ok(())
    }
    walk(root, &mut out)?;
    out.sort();
    Ok(out)
}

fn extract_tool_names(source: &str) -> BTreeSet<String> {
    let mut tools = BTreeSet::new();
    let bytes = source.as_bytes();
    let mut idx = 0usize;
    while let Some(rel) = source[idx..].find("#[tool(") {
        let start = idx + rel;
        let window = &source[start..source.len().min(start + 400)];
        if let Some(name) = parse_tool_name(window) {
            tools.insert(name);
        }
        idx = start + 7;
        if idx >= bytes.len() {
            break;
        }
    }
    tools
}

fn parse_tool_name(window: &str) -> Option<String> {
    // Prefer explicit name = "..." inside the attribute block.
    let name_key = "name";
    let mut rest = window;
    while let Some(pos) = rest.find(name_key) {
        let after = rest[pos + name_key.len()..].trim_start();
        if !after.starts_with('=') {
            rest = &rest[pos + name_key.len()..];
            continue;
        }
        let after_eq = after[1..].trim_start();
        if let Some(stripped) = after_eq.strip_prefix('"') {
            if let Some(end) = stripped.find('"') {
                return Some(stripped[..end].to_string());
            }
        }
        rest = after;
    }
    None
}

fn check_no_machine_local_paths(
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

fn check_codex_projection(root: &Path, findings: &mut Vec<AgentExposureFinding>) {
    let path = root.join(CODEX_CONFIG_REL);
    let Ok(content) = fs::read_to_string(&path) else {
        findings.push(finding(
            "critical",
            "codex_projection_missing",
            CODEX_CONFIG_REL,
            "Codex MCP projection file is missing",
            None,
        ));
        return;
    };
    if content.contains("/home/") || content.contains("/Users/") {
        findings.push(finding(
            "critical",
            "absolute_machine_path",
            CODEX_CONFIG_REL,
            "Codex MCP env must not commit machine-local absolute paths; resolve project root at runtime",
            None,
        ));
    }
}

fn check_repo_access_profile(root: &Path, findings: &mut Vec<AgentExposureFinding>) {
    let path = root.join(REPO_ACCESS_REL);
    let Ok(content) = fs::read_to_string(&path) else {
        // Optional operator profile; missing is fine.
        return;
    };
    if content.contains("/home/") || content.contains("/Users/") {
        findings.push(finding(
            "critical",
            "absolute_machine_path",
            REPO_ACCESS_REL,
            "repo-access profile must use portable relative paths or runtime-resolved roots, not committed home paths",
            None,
        ));
    }
}

fn check_docs_quarantine(root: &Path, findings: &mut Vec<AgentExposureFinding>) {
    check_quarantine_readme(
        root,
        DOCS_README_REL,
        DOCS_QUARANTINE_MARKERS,
        "docs_quarantine_missing",
        "docs_quarantine_markers_missing",
        "docs/README.md must exist and declare docs as non-runtime-authority quarantine",
        findings,
    );
}

fn check_kernel_domains_quarantine(root: &Path, findings: &mut Vec<AgentExposureFinding>) {
    check_quarantine_readme(
        root,
        KERNEL_DOMAINS_README_REL,
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
    let missing: Vec<&str> = markers
        .iter()
        .copied()
        .filter(|marker| {
            !content
                .to_ascii_lowercase()
                .contains(&marker.to_ascii_lowercase())
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_tool_names_from_attribute_blocks() {
        let source = r#"
            #[tool(
                name = "cynic_health",
                description = "health"
            )]
            async fn health() {}

            #[tool(name = "cynic_judge", description = "judge")]
            async fn judge() {}
        "#;
        let names = extract_tool_names(source);
        assert!(names.contains("cynic_health"));
        assert!(names.contains("cynic_judge"));
        assert_eq!(names.len(), 2);
    }
}
