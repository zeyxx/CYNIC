#![allow(dead_code)]
//! MCP tool proof index: static audit that maps every implemented `#[tool]`
//! back to its exposure declaration, producing a machine-readable read-back
//! proof. This covers the gap where ICE 1.96 prevents cynic-mcp from compiling
//! a runtime call ledger on this host.
//!
//! Schema: cynic.cli.mcp_call_proof_index.v1

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::agent_exposure_audit::checks::repo_root;
use crate::agent_exposure_audit::source_scan::scan_workspace_mcp_tools;
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct McpToolProof {
    pub name: String,
    pub description: String,
    pub source_file: String,
    pub line: usize,
    pub declared: bool,
    pub domain: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct McpCallProofIndex {
    pub schema: &'static str,
    pub status: &'static str,
    pub rule: &'static str,
    pub tool_count: usize,
    pub declared_count: usize,
    pub phantom_count: usize,
    pub undeclared_count: usize,
    pub coverage_percent: f64,
    pub tools: Vec<McpToolProof>,
    pub phantoms: Vec<String>,
    pub undeclared: Vec<String>,
}

const SCHEMA: &str = "cynic.cli.mcp_call_proof_index.v1";

pub(crate) fn mcp_call_proof_index() -> McpCallProofIndex {
    let root = repo_root();
    // Real workspace-wide production scan. No fabricated coverage: with zero
    // in-repo `#[tool]` implementations, every declared tool is an explicit
    // phantom and coverage is 0 — never 100 by construction.
    let discoveries = scan_workspace_mcp_tools();
    // Load declared tools + domains from agent-exposure
    let exposure_path = root.join("infra/agent-exposure.json");
    let exposure: serde_json::Value = std::fs::read_to_string(&exposure_path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or(serde_json::Value::Null);

    let declared: BTreeSet<String> = exposure
        .pointer("/mcp_servers/0/tools")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    t.get("name")
                        .and_then(|n| n.as_str().map(|s| s.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    let tools: Vec<McpToolProof> = discoveries
        .iter()
        .map(|d| {
            let domain = exposure
                .pointer("/mcp_tool_domains")
                .and_then(|v| v.as_object())
                .and_then(|obj| {
                    obj.iter().find_map(|(domain, tools)| {
                        tools
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .any(|t| t.as_str() == Some(&d.name))
                                    .then(|| domain.clone())
                            })
                            .flatten()
                    })
                });
            McpToolProof {
                name: d.name.clone(),
                description: d.description.clone(),
                source_file: d.source_file.clone(),
                line: d.line,
                declared: declared.contains(&d.name),
                domain,
            }
        })
        .collect();

    let impl_names: BTreeSet<String> = discoveries.iter().map(|d| d.name.clone()).collect();

    let (phantom, undeclared, coverage) = if impl_names.is_empty() {
        // No in-repo implementations: every declared tool lacks a counterpart.
        let p: Vec<String> = declared.iter().cloned().collect();
        let c = if declared.is_empty() { 100.0 } else { 0.0 };
        (p, vec![], c)
    } else {
        let p: Vec<String> = declared.difference(&impl_names).cloned().collect();
        let u: Vec<String> = impl_names.difference(&declared).cloned().collect();
        let c =
            (impl_names.intersection(&declared).count() as f64 / impl_names.len() as f64) * 100.0;
        (p, u, c)
    };

    let blocked = !phantom.is_empty() || !undeclared.is_empty();

    McpCallProofIndex {
        schema: SCHEMA,
        status: if blocked { "blocked" } else { "ok" },
        rule: "every implemented MCP #[tool] must be declared in agent-exposure.json and every declared tool must resolve to a #[tool] impl",
        tool_count: tools.len(),
        declared_count: declared.len(),
        phantom_count: phantom.len(),
        undeclared_count: undeclared.len(),
        coverage_percent: (coverage * 100.0).round() / 100.0,
        tools,
        phantoms: phantom,
        undeclared,
    }
}

pub(crate) fn persist_mcp_proof_index(
    index: &McpCallProofIndex,
    output: Option<&PathBuf>,
    persist: bool,
) -> Result<(), String> {
    if persist {
        let root = repo_root();
        let path = output
            .map(|p| root.join(p))
            .unwrap_or_else(|| root.join("proofs/mcp-call-proof-index.json"));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(index).map_err(|e| e.to_string())?;
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_index_tracks_rollout_gradient() {
        // Gradient: every tool implemented in cynic-mcp converts exactly one
        // phantom into coverage. Undeclared must always be zero — an
        // implemented tool without exposure declaration is never acceptable.
        let index = mcp_call_proof_index();
        assert!(index.tool_count >= 3);
        assert!(index.declared_count > index.tool_count);
        assert_eq!(index.undeclared_count, 0);
        assert_eq!(index.phantom_count, index.declared_count - index.tool_count);
        assert_eq!(index.status, "blocked");
    }
}
