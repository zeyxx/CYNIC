//! Local queue observation — composes branch_graph + discovery + gc
//! via in-process commands and pure `build_queue_plan`.

use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::build_discovery::{
    BUILD_DISCOVERY_REPORT_SCHEMA, BuildDiscoveryReport, DiscoveredCrate, HostTriple, Toolchain,
};
use cynic_protocol::branch_graph::{
    enrich_with_parents, parse_git_for_each_ref, parse_git_log_graph, BRANCH_GRAPH_SCHEMA,
    BranchGraph,
};
use cynic_protocol::queue::QueuePlan;

use crate::adapters::local_build_artefact::LocalBuildArtefactAdapter;
use crate::build_artefact::RetentionPlanner;
use cynic_protocol::cargo_metadata::parse_cargo_metadata;
use crate::build_queue::build_queue_plan;

pub struct CaptureOutput {
    pub success: bool,
    pub stdout: String,
}

fn capture_command(bin: &str, args: &[&str]) -> Result<CaptureOutput, String> {
    let mut cmd = std::process::Command::new(bin);
    cmd.args(args);
    let output = cmd.output().map_err(|e| format!("exec error: {e}"))?;
    Ok(CaptureOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
    })
}

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

fn observe_branch_graph_local() -> Result<BranchGraph, String> {
    let root_commit = capture_command("git", &["rev-parse", "HEAD"])
        .map_err(|e| format!("git rev-parse failed: {e}"))?
        .stdout
        .trim()
        .to_string();

    let refs_raw = capture_command(
        "git",
        &[
            "for-each-ref",
            "--format=%(refname)|%(objectname)|%(committerdate:raw)|%(upstream:short)",
            "refs/heads/",
        ],
    )
    .map_err(|e| format!("git for-each-ref failed: {e}"))?
    .stdout;

    let log_raw = capture_command("git", &["log", "--all", "--pretty=format:%H %P"])
        .map_err(|e| format!("git log graph failed: {e}"))?
        .stdout;

    let parent_map = parse_git_log_graph(&log_raw);
    let mut nodes = parse_git_for_each_ref(&refs_raw, "main");
    enrich_with_parents(&mut nodes, &parent_map);

    Ok(BranchGraph {
        schema: BRANCH_GRAPH_SCHEMA.to_string(),
        root: "main".to_string(),
        root_commit,
        nodes,
    })
}

fn observe_discovery_local() -> Result<BuildDiscoveryReport, String> {
    let mut residual_unknowns: Vec<String> = Vec::new();
    let meta_raw = capture_command("cargo", &cynic_protocol::cargo_metadata::cargo_metadata_argv())
        .map_err(|e| format!("cargo metadata failed: {e}"))?
        .stdout;
    let disc_plan = parse_cargo_metadata(&meta_raw)
        .map_err(|e| format!("parse cargo metadata failed: {e}"))?;

    let toolchain = match capture_command("rustc", &["--version"]) {
        Ok(c) => Toolchain(c.stdout.trim().to_string()),
        Err(e) => {
            residual_unknowns.push(format!("rustc_version_unknown:{e}"));
            Toolchain("stable".to_string())
        }
    };
    let host_triple = match capture_command("rustc", &["-vV"]) {
        Ok(c) => {
            let triple = c
                .stdout
                .lines()
                .find(|l| l.starts_with("host:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .unwrap_or("x86_64-unknown-linux-gnu")
                .to_string();
            HostTriple(triple)
        }
        Err(e) => {
            residual_unknowns.push(format!("host_triple_unknown:{e}"));
            HostTriple("x86_64-unknown-linux-gnu".to_string())
        }
    };

    let crates = disc_plan
        .crates
        .into_iter()
        .map(|c| DiscoveredCrate {
            name: c.name,
            version: c.version,
            targets: c.targets,
        })
        .collect();

    Ok(BuildDiscoveryReport {
        schema: BUILD_DISCOVERY_REPORT_SCHEMA.to_string(),
        toolchain,
        host_triple,
        targets: vec![],
        crates,
        observed_at_unix_s: now_secs(),
        residual_unknowns,
    })
}
