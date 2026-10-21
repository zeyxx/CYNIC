//! CLI surface for pool eligibility: `cynic build pool eligibility`.
//!
//! Reads the registry declaration (labels + required tools) for one node,
//! probes the local host's toolchain, and reports the verdict. Read-only:
//! it never registers, unregisters, or mutates a runner.

use clap::Subcommand;
use cynic_host::adapters::forgejo_run_observer::{
    observations_from_runs, RunRecord as ForgejoRunRecord,
};
use cynic_host::adapters::local_toolchain_probe::LocalToolchainProbe;
use cynic_host::domain::pool_eligibility::{
    evaluate_pool_eligibility, EnrollmentEvidence, PoolDeclaration, PoolEligibility, ToolKind,
    ToolRequirement,
};
use cynic_host::domain::pool_metrics::measure_pool;
use cynic_protocol::build_memory_capacity::{
    decide_memory_capacity, MemoryCapacityAssessment, MemoryCapacityDecision,
    DEFAULT_RESERVED_BYTES, DEFAULT_UNIT_RSS_BYTES,
};
use cynic_lifecycle::LocalArgvRunner;
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
        /// Pool label the host is serving (e.g. `cynic/build`). The registry
        /// resolves it to the enrolled nodes granting that label.
        #[arg(long)]
        label: Option<String>,
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
    /// Decide how many concurrent compilations this runner can hold.
    ///
    /// The F-gate is sharded into several jobs that share one build target
    /// directory, and the shard count is a function of the machine, not a
    /// constant: when two shards compile the same dependency tree at the
    /// same time, the kernel OOM-kills one of them and the CI reports the
    /// kill as a test failure. This verb reads the runner's memory and
    /// answers with the parallelism that memory can hold.
    Memory {
        /// Registry path (infra/registry.json).
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Measure the cost and latency of the pool jobs.
    Metrics {
        /// Pool label to measure (e.g. `cynic/build`).
        #[arg(long, default_value = "cynic/build")]
        label: String,
        /// Number of recent runs to observe.
        #[arg(long, default_value_t = 100)]
        limit: usize,
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
    status: Option<String>,
    #[serde(default)]
    enrollment: Option<PoolRegistryEnrollment>,
    #[serde(default)]
    pool: Option<PoolRegistryDeclaration>,
}

#[derive(Debug, Deserialize)]
struct PoolRegistryEnrollment {
    #[serde(default)]
    enrolled_by: Option<String>,
    #[serde(default)]
    date: Option<String>,
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
    /// `binary` (default) or `pkg_config`.
    #[serde(default)]
    kind: Option<String>,
}

/// Resolve the registry nodes granting a pool label, base-enrolled only.
///
/// A runner knows the label it serves, not its own registry id: the registry
/// is the authority for pool membership, so the label is resolved here.
pub fn read_nodes_granting_label(registry_path: &str, label: &str) -> Result<Vec<String>, String> {
    let raw = fs::read_to_string(registry_path)
        .map_err(|error| format!("failed to read registry {registry_path}: {error}"))?;
    let registry: PoolRegistry = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse registry {registry_path}: {error}"))?;
    let mut granted = Vec::new();
    for node in &registry.nodes {
        let pool = match &node.pool {
            Some(pool) => pool,
            None => continue,
        };
        if !pool.labels.iter().any(|declared| declared == label) {
            continue;
        }
        let enrolled = node
            .enrollment
            .as_ref()
            .map(|e| e.enrolled_by.is_some())
            .unwrap_or(false);
        if !enrolled || node.status.as_deref() == Some("stub") {
            continue;
        }
        let id = node
            .id
            .clone()
            .or_else(|| node.name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        granted.push(id);
    }
    if granted.is_empty() {
        return Err(format!(
            "no base-enrolled node grants the pool label {label}"
        ));
    }
    Ok(granted)
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
                kind: match tool.kind.as_deref() {
                    Some("pkg_config") => ToolKind::PkgConfig,
                    _ => ToolKind::Binary,
                },
            })
            .collect(),
    })
}

/// Read the base enrollment evidence of one node from the registry.
pub fn read_enrollment_evidence(
    registry_path: &str,
    node_id: &str,
) -> Result<EnrollmentEvidence, String> {
    let raw = fs::read_to_string(registry_path)
        .map_err(|error| format!("failed to read registry {registry_path}: {error}"))?;
    let registry: PoolRegistry = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse registry {registry_path}: {error}"))?;
    let node = registry
        .nodes
        .iter()
        .find(|node| node.id.as_deref() == Some(node_id) || node.name.as_deref() == Some(node_id))
        .ok_or_else(|| format!("node not found in registry: {node_id}"))?;
    let status = node.status.clone().unwrap_or_default();
    let stamp = node.enrollment.as_ref();
    Ok(EnrollmentEvidence {
        enrolled: stamp.is_some(),
        status,
        enrolled_by: stamp.and_then(|entry| entry.enrolled_by.clone()),
        enrolled_at: stamp.and_then(|entry| entry.date.clone()),
    })
}

/// Evaluate the host against a pool label.
///
/// The registry grants the label to the base-enrolled nodes that declare it;
/// the host must provide the union of the tools those nodes require. This is
/// what a runner can prove about itself: it serves a label, and the registry
/// says who may serve it.
pub fn evaluate_label(
    registry_path: &str,
    label: &str,
    observed_labels: &[String],
) -> Result<PoolEligibility, String> {
    let granted = read_nodes_granting_label(registry_path, label)?;
    let mut required: Vec<ToolRequirement> = Vec::new();
    for node_id in &granted {
        let declaration = read_pool_declaration(registry_path, node_id)?;
        for tool in declaration.required_tools {
            if !required.iter().any(|existing| existing.name == tool.name) {
                required.push(tool);
            }
        }
    }
    let tool_kinds: Vec<(String, ToolKind)> = required
        .iter()
        .map(|tool| (tool.name.clone(), tool.kind))
        .collect();
    let probe = LocalToolchainProbe::new(LocalArgvRunner);
    let observations = probe.probe_tools(&tool_kinds);
    Ok(evaluate_pool_eligibility(
        label,
        &PoolDeclaration {
            labels: vec![label.to_string()],
            required_tools: required,
        },
        &EnrollmentEvidence {
            enrolled: true,
            status: "observed".to_string(),
            enrolled_by: None,
            enrolled_at: None,
        },
        observed_labels,
        &observations,
    ))
}

/// Probe the local host for the declared tools and evaluate eligibility.
pub fn evaluate_local_node(
    registry_path: &str,
    node_id: &str,
    observed_labels: &[String],
) -> Result<PoolEligibility, String> {
    let declaration = read_pool_declaration(registry_path, node_id)?;
    let tool_kinds: Vec<(String, ToolKind)> = declaration
        .required_tools
        .iter()
        .map(|tool| (tool.name.clone(), tool.kind))
        .collect();
    let enrollment = read_enrollment_evidence(registry_path, node_id)?;
    let probe = LocalToolchainProbe::new(LocalArgvRunner);
    let observations = probe.probe_tools(&tool_kinds);
    Ok(evaluate_pool_eligibility(
        node_id,
        &declaration,
        &enrollment,
        observed_labels,
        &observations,
    ))
}

/// Measure the pool cost and latency from the sovereign Forgejo runs.
///
/// Read-only. Forgejo 11 does not expose the executor of a run, so the
/// attribution falls back to the label; the observation keeps the measured
/// latency, which is what placement decisions need.
pub fn measure_pool_from_forgejo(
    root: &std::path::Path,
    registry_path: &str,
    label: &str,
    limit: usize,
    json: bool,
) -> Result<(), String> {
    let (api_url, owner, repo, token) =
        super::forgejo::resolve_forgejo(root).map_err(|e| format!("registry error: {e}"))?;
    let url = format!("{api_url}/api/v1/repos/{owner}/{repo}/actions/tasks?limit={limit}");
    let value = super::forgejo::http_get_json(&url, &token)
        .map_err(|e| format!("forgejo request failed: {e}"))?;
    let runs: Vec<ForgejoRunRecord> = value
        .get("workflow_runs")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let observations = observations_from_runs(&runs, label, &|_| label.to_string());
    let report = measure_pool(&observations);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
        );
        return Ok(());
    }
    println!("Pool metrics: {label}");
    println!("  jobs:        {}", report.jobs);
    println!(
        "  outcome:     {} success, {} failed, {} cancelled ({:.1}% failure)",
        report.succeeded,
        report.failed,
        report.cancelled,
        report.failure_rate * 100.0
    );
    if let Some(mean) = report.mean_wall_seconds {
        println!("  mean wall:   {mean:.1}s");
    }
    if let Some(median) = report.median_wall_seconds {
        println!("  median wall: {median:.1}s");
    }
    if let Some(p95) = report.p95_wall_seconds {
        println!("  p95 wall:    {p95:.1}s");
    }
    if let Some(queue) = report.mean_queue_seconds {
        println!("  mean queue:  {queue:.1}s");
    }
    for runner in &report.runners {
        println!(
            "  runner {:<16} jobs={:<4} failure={:.1}%",
            runner.runner,
            runner.jobs,
            runner.failure_rate * 100.0
        );
    }
    let inefficiencies = &report.inefficiencies;
    println!();
    println!("Execution inefficiencies:");
    println!("  groups:              {}", inefficiencies.groups);
    println!(
        "  wasted:              {}s of {}s ({:.1}%)",
        inefficiencies.redundant_wall_seconds + inefficiencies.failed_wall_seconds,
        inefficiencies.total_wall_seconds,
        inefficiencies.wasted_ratio * 100.0
    );
    println!("  redundant jobs:      {}", inefficiencies.redundant_jobs);
    println!(
        "  rework (failed):     {}s",
        inefficiencies.failed_wall_seconds
    );
    if let Some(speedup) = inefficiencies.mean_parallel_speedup {
        println!("  mean speedup:        {speedup:.2}x");
    }
    println!("  serial groups:       {}", inefficiencies.serial_groups);
    for group in &inefficiencies.worst_groups {
        println!(
            "  worst {:<8} jobs={:<3} unique={:<3} redundant={:<3} span={}s speedup={}",
            &group.head_sha.chars().take(8).collect::<String>(),
            group.jobs,
            group.unique_jobs,
            group.redundant_jobs,
            group
                .span_seconds
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string()),
            group
                .parallel_speedup
                .map(|s| format!("{s:.2}x"))
                .unwrap_or_else(|| "-".to_string())
        );
    }
    Ok(())
}

/// Render the verdict for the operator.
/// Decide how many concurrent compilations this runner can hold.
///
/// The observation is the local capacity adapter; the decision is the
/// pure predicate in `cynic-protocol`. An unobserved memory budget is
/// REFUSED, never a silent default: a build that starts against an
/// unknown budget dies with SIGKILL, and the CI reports that kill as a
/// test failure.
pub fn evaluate_runner_memory() -> MemoryCapacityAssessment {
    use cynic_host::adapters::local_capacity::LocalCapacityAdapter;
    use cynic_protocol::capacity::LoadPolicy;

    let adapter = LocalCapacityAdapter::new();
    let available = match adapter.assess_capacity(&LoadPolicy::with_legacy_defaults("node-default"))
    {
        Ok(assessment) => Some(assessment.available_ram_bytes),
        Err(_) => None,
    };
    decide_memory_capacity(
        available,
        DEFAULT_UNIT_RSS_BYTES,
        DEFAULT_RESERVED_BYTES,
    )
}

pub fn render_memory(assessment: &MemoryCapacityAssessment) {
    match &assessment.decision {
        MemoryCapacityDecision::Proceed {
            jobs,
            available_bytes,
        } => {
            println!("memory capacity: {jobs} concurrent compilation(s)");
            println!("  available: {} bytes", available_bytes);
            println!("  per unit:  {} bytes", assessment.unit_rss_bytes);
            println!("  reserved:  {} bytes", assessment.reserved_bytes);
        }
        MemoryCapacityDecision::Refused {
            available_bytes,
            required_bytes,
        } => {
            println!("memory capacity: REFUSED");
            println!("  available: {} bytes", available_bytes);
            println!("  required:  {} bytes", required_bytes);
        }
        MemoryCapacityDecision::Unknown { reason } => {
            println!("memory capacity: UNKNOWN ({reason})");
        }
    }
}

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
    println!(
        "  enrollment:  {} ({})",
        if report.base_enrolled {
            "ENROLLED"
        } else {
            "NOT ENROLLED"
        },
        report.enrollment.status
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
