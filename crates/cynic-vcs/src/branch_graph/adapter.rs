//! Local branch graph observation — binds `BranchGraphObservationPort` to the
//! host's own git repository (no SSH). Runs `git for-each-ref`, `git rev-parse HEAD`,
//! and `git log --all --pretty=format:%H %P` via `LocalArgvRunner` and reuses the
//! pure `parse_git_for_each_ref` / `parse_git_log_graph` / `enrich_with_parents`
//! parsers.

use std::time::{SystemTime, UNIX_EPOCH};

use cynic_process::ArgvRunner;
use cynic_process::LocalArgvRunner;
use cynic_protocol::branch_graph::BranchGraph;

use crate::branch_graph::port::{BranchGraphObservationError, BranchGraphObservationPort};
use crate::branch_graph::probe::{enrich_with_parents, parse_git_for_each_ref, parse_git_log_graph};

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalBranchGraphAdapter;

impl LocalBranchGraphAdapter {
    pub fn new() -> Self {
        Self
    }
}

fn run_git(runner: &LocalArgvRunner, args: &[&str]) -> Option<String> {
    let out = runner.run(args);
    if out.spawn_error.is_some() {
        return None;
    }
    if out.exit_code != Some(0) {
        return None;
    }
    Some(out.stdout)
}

impl BranchGraphObservationPort for LocalBranchGraphAdapter {
    fn observe_branch_graph(&self) -> Result<BranchGraph, BranchGraphObservationError> {
        let runner = LocalArgvRunner;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let raw_refs = run_git(
            &runner,
            &[
                "git",
                "for-each-ref",
                "--format=%(refname:short)|%(objectname)|%(upstream:short)|%(committerdate:unix)",
                "refs/heads/",
            ],
        )
        .unwrap_or_default();

        let root_commit = run_git(&runner, &["git", "rev-parse", "HEAD"])
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let raw_log = run_git(&runner, &["git", "log", "--all", "--pretty=format:%H %P"])
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
