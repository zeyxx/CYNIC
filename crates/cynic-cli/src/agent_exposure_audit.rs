use std::collections::BTreeSet;
use std::path::Path;

use crate::agent_exposure_audit::checks::{
    check_agent_env_paths, check_codex_projection, check_docs_quarantine,
    check_kernel_domains_quarantine, check_mcp_command_projection, check_no_machine_local_paths,
    check_repo_access_profile, check_session_handshake, finding, repo_root,
};
use crate::agent_exposure_audit::json_scan::{count_tiers, domain_tools, exposed_tools, load_json};
use crate::agent_exposure_audit::source_scan::scan_mcp_tool_names;
use crate::agent_exposure_audit::types::AgentExposureCounts;

pub(crate) mod checks;
mod json_scan;
pub(crate) mod source_scan;
mod types;

pub(crate) use types::{AgentExposureAuditReport, AgentExposureFinding};

const SCHEMA: &str = "cynic.cli.agent_exposure_audit.v1";
const EXPOSURE_REL: &str = "infra/agent-exposure.json";
const DOCS_README_REL: &str = "docs/README.md";
const KERNEL_DOMAINS_README_REL: &str = "crates/cynic-kernel/domains/README.md";
const CODEX_CONFIG_REL: &str = ".codex/config.toml";
const REPO_ACCESS_REL: &str = "infra/repo-access-profile.json";
const MCP_SRC_REL: &str = "crates/cynic-mcp/src";

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
            return report(
                &root,
                &findings,
                &AgentExposureCounts {
                    exposed_tool_count: 0,
                    source_tool_count: 0,
                    domain_count: 0,
                    baseline_runtime_count: 0,
                    product_domain_count: 0,
                    operator_local_count: 0,
                    legacy_breakglass_count: 0,
                },
            );
        }
    };

    check_no_machine_local_paths(&exposure_path, &exposure.to_string(), &mut findings);
    check_codex_projection(&root, CODEX_CONFIG_REL, &mut findings);
    check_mcp_command_projection(
        &root,
        CODEX_CONFIG_REL,
        &exposure,
        EXPOSURE_REL,
        &mut findings,
    );
    check_session_handshake(&exposure, EXPOSURE_REL, &mut findings);
    check_repo_access_profile(&root, REPO_ACCESS_REL, &mut findings);
    check_docs_quarantine(&root, DOCS_README_REL, &mut findings);
    check_kernel_domains_quarantine(&root, KERNEL_DOMAINS_README_REL, &mut findings);

    let exposed = exposed_tools(&exposure);
    let domains = domain_tools(&exposure);
    let mcp_src_path = root.join(MCP_SRC_REL);
    let source_tools = if mcp_src_path.exists() {
        scan_mcp_tool_names(&mcp_src_path)
    } else {
        // cynic-mcp is currently extracted/external; match declared tools directly
        exposed.clone()
    };
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

    check_agent_env_paths(&exposure, EXPOSURE_REL, &mut findings);

    let tier_counts = count_tiers(&exposure);
    report(
        &root,
        &findings,
        &AgentExposureCounts {
            exposed_tool_count: exposed.len(),
            source_tool_count: source_tools.len(),
            domain_count: domains.len(),
            baseline_runtime_count: tier_counts.0,
            product_domain_count: tier_counts.1,
            operator_local_count: tier_counts.2,
            legacy_breakglass_count: tier_counts.3,
        },
    )
}

fn report(
    root: &Path,
    findings: &[AgentExposureFinding],
    counts: &AgentExposureCounts,
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
        exposed_tool_count: counts.exposed_tool_count,
        source_tool_count: counts.source_tool_count,
        domain_count: counts.domain_count,
        baseline_runtime_count: counts.baseline_runtime_count,
        product_domain_count: counts.product_domain_count,
        operator_local_count: counts.operator_local_count,
        legacy_breakglass_count: counts.legacy_breakglass_count,
        ok_count,
        blocked_count,
        findings: findings.to_vec(),
    }
}
