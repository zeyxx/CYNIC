//! Local queue observation — composes branch_graph + discovery + gc
//! via in-process `capture_command` and pure `build_queue_plan`.

use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::branch_graph::{BranchGraph, BRANCH_GRAPH_SCHEMA};
use cynic_protocol::build_discovery::{
    BuildDiscoveryReport, HostTriple, Toolchain, BUILD_DISCOVERY_REPORT_SCHEMA,
};
use cynic_protocol::queue::QueuePlan;

use crate::adapters::local_build_artefact::LocalBuildArtefactAdapter;
use crate::domain::branch_graph::probe::{enrich_with_parents, parse_git_for_each_ref, parse_git_log_graph};
use crate::domain::build_artefact::port::BuildArtefactObservationPort;
use crate::domain::build_discovery::probe::parse_cargo_metadata;
use crate::domain::queue::probe::build_queue_plan;
use crate::process_observation::capture_command;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalQueueAdapter;

impl LocalQueueAdapter {
    pub fn observe(&self) -> Result<QueuePlan, String> {
        let bg = observe_branch_graph_local().unwrap_or_else(|_| BranchGraph {
            schema: BRANCH_GRAPH_SCHEMA.to_string(),
            root: "main".to_string(),
            root_commit: String::new(),
            nodes: vec![],
        });
        let discovery = observe_discovery_local().unwrap_or_else(|e| BuildDiscoveryReport {
            schema: BUILD_DISCOVERY_REPORT_SCHEMA.to_string(),
            toolchain: Toolchain("stable".to_string()),
            host_triple: HostTriple("x86_64-unknown-linux-gnu".to_string()),
            targets: vec![],
            crates: vec![],
            observed_at_unix_s: now_secs(),
            residual_unknowns: vec![format!("discovery_fallback:{e}")],
        });
        let gc_plan = LocalBuildArtefactAdapter
            .observe_build_artefacts()
            .map(|r| LocalBuildArtefactAdapter.plan_gc(&r))
            .unwrap_or_else(|_| cynic_protocol::build_artefact::BuildArtefactGcPlan {
                schema: cynic_protocol::build_artefact::BUILD_ARTEFACT_GC_PLAN_SCHEMA.to_string(),
                items: vec![],
                total_reclaimable_bytes: 0,
                total_reclaimable_human: "0B".to_string(),
            });
        Ok(build_queue_plan(&bg, &discovery, &gc_plan, now_secs()))
    }
}

impl crate::domain::queue::port::QueuePlanningPort for LocalQueueAdapter {
    fn plan_queue(
        &self,
        branch_graph: &BranchGraph,
        discovery: &BuildDiscoveryReport,
        gc_plan: &cynic_protocol::build_artefact::BuildArtefactGcPlan,
        now: u64,
    ) -> QueuePlan {
        build_queue_plan(branch_graph, discovery, gc_plan, now)
    }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn observe_branch_graph_local() -> Result<BranchGraph, String> {
    let root_commit = capture_command("git", &["rev-parse", "HEAD"])
        .ok()
        .filter(|c| c.success)
        .map(|c| c.stdout.trim().to_string())
        .unwrap_or_default();
    let raw_refs = capture_command(
        "git",
        &["for-each-ref", "--format=%(refname:short)|%(objectname)|%(upstream:short)|%(committerdate:unix)", "refs/heads/"],
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
    let g = parse_git_for_each_ref(&raw_refs, &root_commit, now_secs());
    let map = parse_git_log_graph(&raw_log);
    Ok(enrich_with_parents(g, &map))
}

fn observe_discovery_local() -> Result<BuildDiscoveryReport, String> {
    let json = capture_command("cargo", &["metadata", "--format-version=1", "--no-deps"])
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
            c.stdout.lines().find(|l| l.starts_with("host:")).map(|l| l["host:".len()..].trim().to_string()).unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string())
        })
        .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string());
    parse_cargo_metadata(&json, now_secs(), toolchain, host_triple)
}
