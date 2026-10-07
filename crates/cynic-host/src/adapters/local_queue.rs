//! Local queue observation — composes branch_graph + discovery + gc
//! via in-process `capture_command` and pure `build_queue_plan`.

use std::time::{SystemTime, UNIX_EPOCH};

use cynic_build::build_discovery::{
    BUILD_DISCOVERY_REPORT_SCHEMA, BuildDiscoveryReport, HostTriple, Toolchain,
};
use cynic_protocol::branch_graph::{BRANCH_GRAPH_SCHEMA, BranchGraph};
use cynic_protocol::queue::QueuePlan;

use crate::adapters::local_build_artefact::LocalBuildArtefactAdapter;
use crate::domain::branch_graph::probe::{
    enrich_with_parents, parse_git_for_each_ref, parse_git_log_graph,
};
use crate::domain::build_artefact::port::BuildArtefactObservationPort;
use crate::domain::build_discovery::probe::parse_cargo_metadata;
use crate::domain::queue::probe::build_queue_plan;
use crate::process_observation::capture_command;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalQueueAdapter;

impl LocalQueueAdapter {
    pub fn observe(&self) -> Result<QueuePlan, String> {
        let mut unknowns: Vec<String> = Vec::new();
        let bg = match observe_branch_graph_local() {
            Ok(bg) => bg,
            Err(e) => {
                unknowns.push(format!("branch_graph_fallback:{e}"));
                BranchGraph {
                    schema: BRANCH_GRAPH_SCHEMA.to_string(),
                    root: "main".to_string(),
                    root_commit: String::new(),
                    nodes: vec![],
                }
            }
        };
        let discovery = match observe_discovery_local() {
            Ok(d) => d,
            Err(e) => {
                unknowns.push(format!("discovery_fallback:{e}"));
                BuildDiscoveryReport {
                    schema: BUILD_DISCOVERY_REPORT_SCHEMA.to_string(),
                    toolchain: Toolchain("stable".to_string()),
                    host_triple: HostTriple("x86_64-unknown-linux-gnu".to_string()),
                    targets: vec![],
                    crates: vec![],
                    observed_at_unix_s: now_secs(),
                    residual_unknowns: vec![format!("discovery_fallback:{e}")],
                }
            }
        };
        // A discovery that succeeded partially still names its gaps — those
        // propagate to the plan so callers can tell live from guessed.
        unknowns.extend(discovery.residual_unknowns.iter().cloned());
        let gc_plan = match LocalBuildArtefactAdapter.observe_build_artefacts_shallow() {
            Ok(report) => {
                // Shallow skips are the plan's unknowns too: sizes read 0 by
                // contract, never by accident.
                unknowns.extend(report.residual_unknowns.iter().cloned());
                LocalBuildArtefactAdapter.plan_gc(&report)
            }
            Err(e) => {
                unknowns.push(format!("build_artefact_gc_fallback:{e}"));
                cynic_build::build_artefact::BuildArtefactGcPlan {
                    schema: cynic_build::build_artefact::BUILD_ARTEFACT_GC_PLAN_SCHEMA.to_string(),
                    items: vec![],
                    total_reclaimable_bytes: 0,
                    total_reclaimable_human: "0B".to_string(),
                }
            }
        };
        let mut plan = build_queue_plan(&bg, &discovery, &gc_plan, now_secs());
        plan.residual_unknowns = unknowns;
        Ok(plan)
    }
}

impl crate::domain::queue::port::QueuePlanningPort for LocalQueueAdapter {
    fn plan_queue(
        &self,
        branch_graph: &BranchGraph,
        discovery: &BuildDiscoveryReport,
        gc_plan: &cynic_build::build_artefact::BuildArtefactGcPlan,
        now: u64,
    ) -> QueuePlan {
        build_queue_plan(branch_graph, discovery, gc_plan, now)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn observe_branch_graph_local() -> Result<BranchGraph, String> {
    let root_commit = capture_command("git", &["rev-parse", "HEAD"])
        .ok()
        .filter(|c| c.success)
        .map(|c| c.stdout.trim().to_string())
        .unwrap_or_default();
    let raw_refs = capture_command(
        "git",
        &[
            "for-each-ref",
            "--format=%(refname:short)|%(objectname)|%(upstream:short)|%(committerdate:unix)",
            "refs/heads/",
        ],
    )
    .ok()
    .filter(|c| c.success)
    .map(|c| c.stdout)
    .unwrap_or_default();
    let raw_log = capture_command("git", &["log", "--all", "--pretty=format:%H %P"])
        .ok()
        .filter(|c| c.success)
        .map(|c| c.stdout)
        .unwrap_or_default();
    // No branches observed means git is missing or this is not a repository —
    // an empty graph is a lie here, not a clean queue.
    if raw_refs.trim().is_empty() {
        return Err("git_branch_graph_unavailable".to_string());
    }
    let g = parse_git_for_each_ref(&raw_refs, &root_commit, now_secs());
    let map = parse_git_log_graph(&raw_log);
    Ok(enrich_with_parents(g, &map))
}

fn observe_discovery_local() -> Result<BuildDiscoveryReport, String> {
    let json = capture_command("cargo", cynic_protocol::cargo_metadata::CARGO_METADATA_ARGS)
        .ok()
        .filter(|c| c.success)
        .map(|c| c.stdout)
        .ok_or_else(|| "cargo_metadata_spawn_failed".to_string())?;
    let toolchain = capture_command("rustc", &["--version"])
        .ok()
        .filter(|c| c.success)
        .map(|c| c.stdout.trim().to_string())
        .unwrap_or_else(|| "stable".to_string());
    let host_triple = capture_command("rustc", &["-vV"])
        .ok()
        .filter(|c| c.success)
        .map(|c| {
            c.stdout
                .lines()
                .find(|l| l.starts_with("host:"))
                .map(|l| l["host:".len()..].trim().to_string())
                .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string())
        })
        .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string());
    parse_cargo_metadata(&json, now_secs(), toolchain, host_triple)
}
