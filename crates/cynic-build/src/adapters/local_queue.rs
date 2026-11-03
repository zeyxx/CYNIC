//! Local queue observation — composes branch_graph + discovery + gc
//! via cynic-vcs, cynic-build discovery, and retention adapters.

use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::branch_graph::{BRANCH_GRAPH_SCHEMA, BranchGraph, BranchObservationPort};
use cynic_protocol::build_discovery::{
    BUILD_DISCOVERY_REPORT_SCHEMA, BuildDiscoveryReport, HostTriple, Toolchain,
};
use cynic_protocol::queue::QueuePlan;
use cynic_vcs::branch_graph::{BranchGraphObservationPort, LocalBranchGraphAdapter};

use crate::adapters::build_discovery::LocalBuildDiscoveryAdapter;
use crate::adapters::local_build_artefact::LocalBuildArtefactAdapter;
use crate::build_artefact::RetentionPlanner;
use crate::build_queue::build_queue_plan;
use crate::domain::build_discovery::port::BuildDiscoveryObservationPort;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalQueueAdapter;

impl LocalQueueAdapter {
    pub fn observe(&self) -> Result<QueuePlan, String> {
        let mut unknowns: Vec<String> = Vec::new();
        let bg = match LocalBranchGraphAdapter.observe() {
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
        let discovery = match LocalBuildDiscoveryAdapter.observe() {
            Ok(d) => d,
            Err(e) => {
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
        unknowns.extend(discovery.residual_unknowns.iter().cloned());
        let gc_plan = match LocalBuildArtefactAdapter::observe_shallow() {
            Ok(obs) => {
                let policy = cynic_protocol::retention::default_build_artefact_policy();
                LocalBuildArtefactAdapter.plan(&obs, &policy)
            }
            Err(e) => {
                unknowns.push(format!("build_artefact_gc_fallback:{e}"));
                cynic_protocol::retention::RetentionPlan {
                    schema: cynic_protocol::retention::RETENTION_PLAN_SCHEMA.to_string(),
                    items: vec![],
                    total_reclaimable_bytes: 0,
                    policy_version: "BuildArtefact".to_string(),
                }
            }
        };
        let mut plan = build_queue_plan(&bg, &discovery, &gc_plan, now_secs());
        plan.residual_unknowns = unknowns;
        Ok(plan)
    }

    pub fn plan_queue(
        &self,
        branch_graph: &BranchGraph,
        discovery: &BuildDiscoveryReport,
        gc_plan: &cynic_protocol::retention::RetentionPlan,
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
