//! Remote queue observation over SSH — reuses pure `build_queue_plan`.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_build::build_discovery::{
    BUILD_DISCOVERY_REPORT_SCHEMA, BuildDiscoveryReport, HostTriple, Toolchain,
};
use cynic_protocol::branch_graph::{BRANCH_GRAPH_SCHEMA, BranchGraph};
use cynic_protocol::queue::QueuePlan;
use cynic_protocol::retention::{
    RetentionItem, RetentionKind, RetentionPolicy, RetentionPlan, RetentionScope,
    RETENTION_PLAN_SCHEMA,
};

use crate::domain::branch_graph::probe::{
    enrich_with_parents, parse_git_for_each_ref, parse_git_log_graph,
};
use crate::domain::build_artefact::parse::{parse_checkout_list, parse_df_pressure};
use crate::domain::build_discovery::probe::parse_cargo_metadata;
use crate::domain::build_artefact::plan_build_artefacts;
use crate::domain::queue::probe::build_queue_plan;
use cynic_lifecycle::ArgvRunner;
use std::time::Duration;

pub struct RemoteQueueAdapter<R: ArgvRunner> {
    runner: Arc<R>,
    ssh_target: String,
}

impl<R: ArgvRunner> RemoteQueueAdapter<R> {
    pub fn new(runner: Arc<R>, ssh_target: impl Into<String>) -> Result<Self, String> {
        let t = ssh_target.into();
        if t.trim().is_empty() || t.starts_with('-') {
            return Err("invalid_ssh_target".to_string());
        }
        Ok(Self {
            runner,
            ssh_target: t,
        })
    }
    fn ssh(&self, cmd: &[&str]) -> Result<String, String> {
        let mut argv = vec![
            "ssh",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            "-o",
            "StrictHostKeyChecking=yes",
            &self.ssh_target,
        ];
        argv.extend_from_slice(cmd);
        let out = self.runner.run(&argv);
        if out.success() {
            Ok(out.stdout)
        } else {
            Err(out.spawn_error.unwrap_or(out.stderr))
        }
    }
    pub fn observe(&self) -> Result<QueuePlan, String> {
        let bg = self.observe_branch_graph().unwrap_or_else(|_| BranchGraph {
            schema: BRANCH_GRAPH_SCHEMA.to_string(),
            root: "main".into(),
            root_commit: String::new(),
            nodes: vec![],
        });
        let disc = self
            .observe_discovery()
            .unwrap_or_else(|e| BuildDiscoveryReport {
                schema: BUILD_DISCOVERY_REPORT_SCHEMA.to_string(),
                toolchain: Toolchain("stable".into()),
                host_triple: HostTriple("x86_64-unknown-linux-gnu".into()),
                targets: vec![],
                crates: vec![],
                observed_at_unix_s: now_secs(),
                residual_unknowns: vec![format!("discovery_fallback:{e}")],
            });
        let gc = self.observe_gc_plan();
        Ok(build_queue_plan(&bg, &disc, &gc, now_secs()))
    }
    fn observe_branch_graph(&self) -> Result<BranchGraph, String> {
        let root = self
            .ssh(&["git", "rev-parse", "HEAD"])
            .unwrap_or_default()
            .trim()
            .to_string();
        let refs = self
            .ssh(&[
                "git",
                "for-each-ref",
                "--format=%(refname:short)|%(objectname)|%(upstream:short)|%(committerdate:unix)",
                "refs/heads/",
            ])
            .unwrap_or_default();
        let log = self
            .ssh(&["git", "log", "--all", "--pretty=format:%H %P"])
            .unwrap_or_default();
        Ok(enrich_with_parents(
            parse_git_for_each_ref(&refs, &root, now_secs()),
            &parse_git_log_graph(&log),
        ))
    }
    fn observe_discovery(&self) -> Result<BuildDiscoveryReport, String> {
        let json = self.ssh(&cynic_protocol::cargo_metadata::cargo_metadata_argv())?;
        let toolchain = self
            .ssh(&["rustc", "--version"])
            .unwrap_or_else(|_| "stable".into())
            .trim()
            .to_string();
        let vv = self.ssh(&["rustc", "-vV"]).unwrap_or_default();
        let host = vv
            .lines()
            .find(|l| l.starts_with("host:"))
            .map(|l| l["host:".len()..].trim().to_string())
            .unwrap_or_else(|| "x86_64-unknown-linux-gnu".into());
        parse_cargo_metadata(&json, now_secs(), toolchain, host)
    }
    fn observe_gc_plan(&self) -> RetentionPlan {
        let raw_list = self
            .ssh(&["ls", "-1", "/var/lib/cynic/checkouts"])
            .unwrap_or_default();
        let remote_shas = self
            .ssh(&["git", "ls-remote", "origin"])
            .map(|s| {
                s.lines()
                    .filter_map(|l| l.split_whitespace().next().map(|v| v.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut lines = Vec::new();
        for name in raw_list.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
            let path = format!("/var/lib/cynic/checkouts/{name}");
            let sha = self
                .ssh(&["git", "-C", &path, "rev-parse", "HEAD"])
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| name.to_string());
            let size = self
                .ssh(&["du", "-sb", &path])
                .ok()
                .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok())
                .unwrap_or(0);
            let stat = self
                .ssh(&["stat", "-c", "%u:%g %Y", &path])
                .unwrap_or_else(|_| "999:990 0".into());
            let mut parts = stat.split_whitespace();
            let owner = parts.next().unwrap_or("999:990");
            let age = parts
                .next()
                .and_then(|v| v.parse::<u64>().ok())
                .map(|mtime| now_secs().saturating_sub(mtime))
                .unwrap_or(0);
            let flag = if remote_shas.contains(&sha) { "1" } else { "0" };
            lines.push(format!("{path}|{sha}|{size}|{owner}|{age}|{flag}"));
        }
        let checkouts = parse_checkout_list(&lines.join("\n"), &remote_shas);
        let df_k = self
            .ssh(&["df", "-k", "/var/lib/cynic/checkouts"])
            .or_else(|_| self.ssh(&["df", "-k", "/"]))
            .unwrap_or_default();
        let pressure =
            parse_df_pressure(&df_k, checkouts.iter().filter(|c| c.is_root_owned).count());

        // Convert to RetentionObservation
        let items: Vec<RetentionItem> = checkouts
            .iter()
            .map(|co| RetentionItem {
                id: format!("checkout:{}", co.sha.chars().take(8).collect::<String>()),
                path: co.path.clone(),
                size_bytes: co.size_bytes,
                age: co.age_secs.map(Duration::from_secs),
                reason: String::new(),
                requires_privilege: co.is_root_owned,
                argv: vec![],
                is_orphan: Some(co.is_orphan),
                is_root_owned: Some(co.is_root_owned),
                remote_exists: co.remote_exists,
                sha: Some(co.sha.clone()),
                branch: Some(co.branch.clone()),
                owner_uid: Some(co.owner_uid),
                owner_gid: Some(co.owner_gid),
            })
            .collect();

        let obs = cynic_protocol::retention::RetentionObservation {
            schema: cynic_protocol::retention::RETENTION_OBSERVATION_SCHEMA.to_string(),
            timestamp: String::new(),
            items,
            total_reclaimable_bytes: 0,
            pressure: Some(cynic_protocol::retention::PressureSignal {
                use_pct: pressure.use_pct,
                avail_bytes: pressure.avail_bytes,
                is_pressured: pressure.is_pressured,
            }),
        };

        let policy = RetentionPolicy {
            kind: RetentionKind::BuildArtefact,
            max_age: Some(Duration::from_secs(48 * 3600)),
            pressure_trigger: None,
            preserve_count: 0,
            preserve_profiles: vec![],
            grace_period: None,
            scopes: vec![RetentionScope {
                root: "/var/lib/cynic/checkouts".to_string(),
                patterns: vec![],
            }],
        };
        plan_build_artefacts(&obs, &policy)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
