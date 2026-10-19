//! CLI surface for pool eligibility: `cynic build pool eligibility`.
//!
//! Reads the registry declaration (labels + required tools) for one node,
//! probes the local host's toolchain, and reports the verdict. Read-only:
//! it never registers, unregisters, or mutates a runner.

use clap::Subcommand;
use cynic_host::adapters::local_toolchain_probe::LocalToolchainProbe;
use cynic_host::domain::pool_eligibility::{
    evaluate_pool_eligibility, PoolDeclaration, PoolEligibility, ToolRequirement,
};
use cynic_lifecycle::{ArgvRunner, LocalArgvRunner};
use serde::Deserialize;
use std::fs;

/// CLI arguments for `cynic build pool`.
#[derive(Subcommand, Debug)]
pub(crate) enum BuildPoolCommand {
    /// Evaluate whether this node may serve its declared pool labels.
    Eligibility {
        /// Node id in the registry (defaults to the local node).
        #[arg(long)]
        node: Option<String>,
        /// Labels the runner advertises, as observed.
        #[arg(long, value_delimiter = ',', default_value = "")]
        observed_labels: Vec<String>,
        /// Registry path (infra/registry.json).
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Deserialize)]
struct PoolRegistry {
    nodes: Vec<PoolRegistryNode>,
}

#[derive(Debug, Deserialize)]
struct PoolRegistryNode {
    id: Option<String>,
    name: Option<String>,
    #[serde(default)]
    pool: Option<PoolRegistryDeclaration>,
}

#[derive(Debug, Deserialize)]
struct PoolRegistryDeclaration {
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    required_tools: Vec<PoolRegistryTool>,
}

#[derive(Debug, Deserialize)]
struct PoolRegistryTool {
    name: String,
    #[serde(default)]
    min_version: Option<String>,
    #[serde(default)]
    required: Option<bool>,
}

/// Best-effort local node id, used when the operator omits `--node`.
pub fn local_node_id() -> String {
    std::env::var("CYNIC_NODE_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "local-kernel".to_string())
}

/// Read the pool declaration of one node from the registry.
pub fn read_pool_declaration(
    registry_path: &str,
    node_id: &str,
) -> Result<PoolDeclaration, String> {
    let raw = fs::read_to_string(registry_path)
        .map_err(|error| format!("failed to read registry {registry_path}: {error}"))?;
    let registry: PoolRegistry = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse registry {registry_path}: {error}"))?;
    let node = registry
        .nodes
        .iter()
        .find(|node| node.id.as_deref() == Some(node_id) || node.name.as_deref() == Some(node_id))
        .ok_or_else(|| format!("node not found in registry: {node_id}"))?;
    let declaration = node
        .pool
        .as_ref()
        .ok_or_else(|| format!("node {node_id} declares no pool membership"))?;
    Ok(PoolDeclaration {
        labels: declaration.labels.clone(),
        required_tools: declaration
            .required_tools
            .iter()
            .map(|tool| ToolRequirement {
                name: tool.name.clone(),
                min_version: tool.min_version.clone(),
                required: tool.required.unwrap_or(true),
            })
            .collect(),
    })
}

/// Probe the local host for the declared tools and evaluate eligibility.
pub fn evaluate_local_node(
    registry_path: &str,
    node_id: &str,
    observed_labels: &[String],
) -> Result<PoolEligibility, String> {
    let declaration = read_pool_declaration(registry_path, node_id)?;
    let tool_names: Vec<String> = declaration
        .required_tools
        .iter()
        .map(|tool| tool.name.clone())
        .collect();
    let probe = LocalToolchainProbe::new(LocalArgvRunner);
    let observations = probe.probe_tools(&tool_names);
    Ok(evaluate_pool_eligibility(
        node_id,
        &declaration,
        observed_labels,
        &observations,
    ))
}

/// Render the verdict for the operator.
pub fn render_eligibility(report: &PoolEligibility) {
    println!("Pool eligibility: {}", report.node_id);
    println!(
        "  verdict:     {}",
        if report.eligible {
            "ELIGIBLE"
        } else {
            "INELIGIBLE"
        }
    );
    println!("  declared:    {}", report.declared_labels.join(", "));
    println!("  observed:    {}", report.observed_labels.join(", "));
    if !report.missing_labels.is_empty() {
        println!("  missing:     {}", report.missing_labels.join(", "));
    }
    if !report.undeclared_labels.is_empty() {
        println!("  undeclared:  {}", report.undeclared_labels.join(", "));
    }
    for tool in &report.tools {
        let state = if tool.present { "ok" } else { "MISSING" };
        println!(
            "  tool {:<8} {:<8} {}",
            tool.name,
            state,
            tool.version.clone().unwrap_or_default()
        );
        if let Some(detail) = &tool.detail {
            println!("             {detail}");
        }
    }
    for reason in &report.reasons {
        println!("  blocker: {reason}");
    }
}
