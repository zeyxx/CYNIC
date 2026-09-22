//! Remote branch graph observation over SSH.
//!
//! Reuses the pure `parse_git_for_each_ref` / `parse_git_log_graph` /
//! `enrich_with_parents` parsers shared with `LocalBranchGraphAdapter`;
//! the transport (SSH) is fully injected via `ArgvRunner` so unit tests
//! inject `MapArgvRunner` returning canned git stdout.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::branch_graph::port::{BranchGraphObservationError, BranchGraphObservationPort};
use crate::domain::branch_graph::probe::{
    enrich_with_parents, parse_git_for_each_ref, parse_git_log_graph,
};
use cynic_lifecycle::ArgvRunner;
use cynic_protocol::branch_graph::BranchGraph;

pub struct RemoteBranchGraphAdapter<R: ArgvRunner> {
    runner: Arc<R>,
    ssh_target: String,
}

impl<R: ArgvRunner> RemoteBranchGraphAdapter<R> {
    pub fn new(runner: Arc<R>, ssh_target: impl Into<String>) -> Result<Self, String> {
        let ssh_target = ssh_target.into();
        if ssh_target.trim().is_empty() || ssh_target.starts_with('-') {
            return Err("invalid_ssh_target".to_string());
        }
        Ok(Self { runner, ssh_target })
    }

    fn ssh_run(&self, remote_argv: &[&str]) -> Result<String, BranchGraphObservationError> {
        let mut argv: Vec<&str> = vec![
            "ssh",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            "-o",
            "StrictHostKeyChecking=yes",
            &self.ssh_target,
        ];
        argv.extend_from_slice(remote_argv);
        let out = self.runner.run(&argv);
        if out.success() {
            Ok(out.stdout)
        } else {
            Err(format!(
                "remote_branch_graph_failed: {}",
                out.spawn_error.clone().unwrap_or(out.stderr)
            ))
        }
    }
}

impl<R: ArgvRunner> BranchGraphObservationPort for RemoteBranchGraphAdapter<R> {
    fn observe_branch_graph(&self) -> Result<BranchGraph, BranchGraphObservationError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let raw_refs = self
            .ssh_run(&[
                "git",
                "for-each-ref",
                "--format=%(refname:short)|%(objectname)|%(upstream:short)|%(committerdate:unix)",
                "refs/heads/",
            ])
            .unwrap_or_default();

        let root_commit = self
            .ssh_run(&["git", "rev-parse", "HEAD"])
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let raw_log = self
            .ssh_run(&["git", "log", "--all", "--pretty=format:%H %P"])
            .unwrap_or_default();

        let graph = parse_git_for_each_ref(&raw_refs, &root_commit, now);
        if raw_log.is_empty() {
            return Ok(graph);
        }
        let commit_map = parse_git_log_graph(&raw_log);
        Ok(enrich_with_parents(graph, &commit_map))
    }
}

impl<R: ArgvRunner> cynic_protocol::branch_graph::BranchObservationPort
    for RemoteBranchGraphAdapter<R>
{
    fn observe(&self) -> Result<BranchGraph, String> {
        BranchGraphObservationPort::observe_branch_graph(self).map_err(|e| e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::{ArgvOutput, MapArgvRunner};
    use std::sync::Arc;

    fn ok(stdout: &str) -> ArgvOutput {
        ArgvOutput {
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
            spawn_error: None,
        }
    }

    #[test]
    fn rejects_invalid_target() {
        let r = Arc::new(MapArgvRunner::default());
        assert!(RemoteBranchGraphAdapter::new(r, "").is_err());
        let r2 = Arc::new(MapArgvRunner::default());
        assert!(RemoteBranchGraphAdapter::new(r2, "--evil").is_err());
    }

    #[test]
    fn observes_empty_when_no_refs() {
        // MapArgvRunner prefix-matches: any ssh argv returns empty ok.
        let runner = MapArgvRunner {
            responses: vec![(vec!["ssh".to_string()], ok(""))],
        };
        let adapter = RemoteBranchGraphAdapter::new(Arc::new(runner), "cynic@10.0.0.1").unwrap();
        let graph = adapter.observe_branch_graph().unwrap();
        assert!(graph.nodes.is_empty());
        assert_eq!(graph.root, "main");
    }

    #[test]
    fn observes_with_refs_and_log() {
        let mut runner = MapArgvRunner::default();
        // Need distinct ssh prefixes: we use generic ssh stub that matches all,
        // so we can't differentiate commands. Use a fake that handles per-command is complex;
        // here we test with a custom runner that inspects argv.
        struct BranchFake;
        impl ArgvRunner for BranchFake {
            fn run(&self, argv: &[&str]) -> ArgvOutput {
                let joined = argv.join(" ");
                if joined.contains("for-each-ref") {
                    return ok(
                        "main|aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa|origin/main|1700000000\nfeature|bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb||1700000000",
                    );
                }
                if joined.contains("rev-parse") {
                    return ok("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
                }
                if joined.contains("log") {
                    return ok(
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    );
                }
                ok("")
            }
        }
        let adapter =
            RemoteBranchGraphAdapter::new(Arc::new(BranchFake), "cynic@10.0.0.1").unwrap();
        let graph = adapter.observe_branch_graph().unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.root, "main");
        let feat = graph.nodes.iter().find(|n| n.name == "feature").unwrap();
        assert_eq!(feat.parent.as_deref(), Some("main"));
        let _ = runner;
    }
}
