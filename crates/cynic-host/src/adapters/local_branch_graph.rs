//! Local branch graph observation — binds `BranchGraphObservationPort` to the
//! host's own git repository (no SSH). Runs `git for-each-ref`, `git rev-parse HEAD`,
//! and `git log --all --pretty=format:%H %P` via `capture_command` and reuses the
//! pure `parse_git_for_each_ref` / `parse_git_log_graph` / `enrich_with_parents`
//! parsers shared with the remote adapter.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::branch_graph::port::{BranchGraphObservationError, BranchGraphObservationPort};
use crate::domain::branch_graph::probe::{
    enrich_with_parents, parse_git_for_each_ref, parse_git_log_graph,
};
use crate::process_observation::capture_command;
use cynic_protocol::branch_graph::BranchGraph;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalBranchGraphAdapter;

impl LocalBranchGraphAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl BranchGraphObservationPort for LocalBranchGraphAdapter {
    fn observe_branch_graph(&self) -> Result<BranchGraph, BranchGraphObservationError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

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

        let root_commit = capture_command("git", &["rev-parse", "HEAD"])
            .ok()
            .filter(|c| c.success)
            .map(|c| c.stdout.trim().to_string())
            .unwrap_or_default();

        let raw_log = capture_command("git", &["log", "--all", "--pretty=format:%H %P"])
            .ok()
            .filter(|c| c.success)
            .map(|c| c.stdout)
            .unwrap_or_default();

        let graph = parse_git_for_each_ref(&raw_refs, &root_commit, now);
        if raw_log.is_empty() {
            return Ok(graph);
        }
        let commit_map = parse_git_log_graph(&raw_log);
        Ok(enrich_with_parents(graph, &commit_map))
    }
}

// Also implement the protocol-level port so either trait can be used.
impl cynic_protocol::branch_graph::BranchObservationPort for LocalBranchGraphAdapter {
    fn observe(&self) -> Result<BranchGraph, String> {
        BranchGraphObservationPort::observe_branch_graph(self).map_err(|e| e)
    }
}
