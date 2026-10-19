//! Remote build artefact observation over SSH.
//!
//! Reuses the pure `parse_checkout_list` / `parse_df_pressure` parsers shared
//! with `LocalBuildArtefactAdapter`; the transport (SSH) is fully injected
//! via `ArgvRunner` so unit tests inject `MapArgvRunner` returning canned
//! `ls`/`du`/`df`/`git ls-remote` stdout.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use crate::domain::build_artefact::parse::{parse_checkout_list, parse_df_pressure};
use crate::domain::build_artefact::{
    plan_build_artefacts, RetentionError, RetentionExecutor, RetentionItem,
    RetentionItemExecution, RetentionObservation, RetentionPlan, RetentionPlanner,
    RetentionPolicy, RetentionResult, RetentionObserver, RETENTION_OBSERVATION_SCHEMA,
    RETENTION_RESULT_SCHEMA,
};
use crate::domain::disk::probe::{parse_df_filesystems, parse_lsblk_disks};
use cynic_lifecycle::ArgvRunner;
use cynic_protocol::retention::PressureSignal;
use cynic_types::paths::CynicPaths;

pub struct RemoteBuildArtefactAdapter<R: ArgvRunner> {
    runner: Arc<R>,
    ssh_target: String,
}

impl<R: ArgvRunner> RemoteBuildArtefactAdapter<R> {
    pub fn new(runner: Arc<R>, ssh_target: impl Into<String>) -> Result<Self, String> {
        let ssh_target = ssh_target.into();
        if ssh_target.trim().is_empty() || ssh_target.starts_with('-') {
            return Err("invalid_ssh_target".to_string());
        }
        Ok(Self { runner, ssh_target })
    }

    fn ssh_run(&self, remote_argv: &[&str]) -> Result<String, String> {
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
                "remote_build_artefact_failed: {}",
                out.spawn_error.clone().unwrap_or(out.stderr)
            ))
        }
    }
}

impl<R: ArgvRunner> RetentionObserver for RemoteBuildArtefactAdapter<R> {
    fn observe(&self) -> Result<RetentionObservation, RetentionError> {
        let checkouts_dir = CynicPaths::state_dir()
            .join("checkouts")
            .display()
            .to_string();
        let timestamp = DateTime::<Utc>::from(SystemTime::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let physical_disks = self
            .ssh_run(&[
                "lsblk",
                "-b",
                "-J",
                "-o",
                "NAME,SIZE,TYPE,ROTA,VENDOR,MODEL,SERIAL,WWN,MOUNTPOINT",
            ])
            .map(|s| parse_lsblk_disks(&s))
            .unwrap_or_default();

        let filesystems = self
            .ssh_run(&[
                "df",
                "-h",
                "--output=source,fstype,size,used,avail,pcent,target",
            ])
            .map(|s| parse_df_filesystems(&s))
            .unwrap_or_default();

        let raw_list = self
            .ssh_run(&["ls", "-1", &checkouts_dir])
            .unwrap_or_default();
        let mut lines = Vec::new();
        let remote_shas = self
            .ssh_run(&["git", "ls-remote", "origin"])
            .map(|s| {
                s.lines()
                    .filter_map(|l| l.split_whitespace().next().map(|v| v.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for name in raw_list.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
            let path = format!("{checkouts_dir}/{name}");
            let sha = self
                .ssh_run(&["git", "-C", &path, "rev-parse", "HEAD"])
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| name.to_string());
            let size = self
                .ssh_run(&["du", "-sb", &path])
                .ok()
                .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok())
                .unwrap_or(0);
            let stat = self
                .ssh_run(&["stat", "-c", "%u:%g %Y", &path])
                .unwrap_or_else(|_| "999:990 0".to_string());
            let mut stat_parts = stat.split_whitespace();
            let owner = stat_parts.next().unwrap_or("999:990");
            let mtime_str = stat_parts.next().unwrap_or("0");
            let mtime: u64 = mtime_str.parse().unwrap_or(0);
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let age = now.saturating_sub(mtime);
            let remote_exists = if remote_shas.contains(&sha) { "1" } else { "0" };
            lines.push(format!("{path}|{sha}|{size}|{owner}|{age}|{remote_exists}"));
        }

        let raw = lines.join("\n");
        let checkouts = parse_checkout_list(&raw, &remote_shas);
        let total_checkout_bytes: u64 = checkouts.iter().map(|c| c.size_bytes).sum();

        let df_k = self
            .ssh_run(&["df", "-k", &checkouts_dir])
            .or_else(|_| self.ssh_run(&["df", "-k", "/"]))
            .unwrap_or_default();
        let root_owned = checkouts.iter().filter(|c| c.is_root_owned).count();
        let pressure = parse_df_pressure(&df_k, root_owned);

        let items: Vec<RetentionItem> = checkouts
            .iter()
            .map(|co| RetentionItem {
                id: format!("checkout:{}", co.sha.chars().take(8).collect::<String>()),
                path: co.path.clone(),
                size_bytes: co.size_bytes,
                age: co.age_secs.map(std::time::Duration::from_secs),
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

        let _ = physical_disks;
        let _ = filesystems;
        let _ = total_checkout_bytes;

        Ok(RetentionObservation {
            schema: RETENTION_OBSERVATION_SCHEMA.to_string(),
            timestamp,
            items,
            total_reclaimable_bytes: 0,
            pressure: Some(PressureSignal {
                use_pct: pressure.use_pct,
                avail_bytes: pressure.avail_bytes,
                is_pressured: pressure.is_pressured,
            }),
        })
    }
}

impl<R: ArgvRunner> RetentionPlanner for RemoteBuildArtefactAdapter<R> {
    fn plan(&self, obs: &RetentionObservation, policy: &RetentionPolicy) -> RetentionPlan {
        plan_build_artefacts(obs, policy)
    }
}

impl<R: ArgvRunner> RetentionExecutor for RemoteBuildArtefactAdapter<R> {
    fn execute(&self, plan: &RetentionPlan) -> Result<RetentionResult, RetentionError> {
        let timestamp = DateTime::<Utc>::from(SystemTime::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let mut items_executed = Vec::new();
        let mut total_reclaimed_bytes = 0u64;
        let mut success = true;

        for item in &plan.items {
            let exec = execute_gc_item(item);
            if exec.success {
                total_reclaimed_bytes += item.size_bytes;
            } else {
                success = false;
            }
            items_executed.push(exec);
        }

        Ok(RetentionResult {
            schema: RETENTION_RESULT_SCHEMA.to_string(),
            timestamp,
            success,
            items_executed,
            total_reclaimed_bytes,
        })
    }
}

fn execute_gc_item(item: &RetentionItem) -> RetentionItemExecution {
    if item.path.is_empty() {
        return RetentionItemExecution {
            item_id: item.id.clone(),
            success: false,
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: "empty path".to_string(),
        };
    }
    // For remote execution, we'd need to SSH. For now, return a stub.
    // The actual execution happens via the local adapter.
    // NOTE: no argv is built here — when remote execution lands it must
    // go through `crate::domain::build_artefact::gc_argv`, never inline.
    RetentionItemExecution {
        item_id: item.id.clone(),
        success: false,
        exit_code: None,
        stdout_tail: String::new(),
        stderr_tail: "remote execution not yet implemented".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::{ArgvOutput, MapArgvRunner};
    use std::sync::Arc;

    fn runner_with(map: Vec<(Vec<String>, ArgvOutput)>) -> MapArgvRunner {
        MapArgvRunner { responses: map }
    }

    #[test]
    fn rejects_invalid_target() {
        let r = Arc::new(runner_with(vec![]));
        assert!(RemoteBuildArtefactAdapter::new(r, "").is_err());
        let r2 = Arc::new(runner_with(vec![]));
        assert!(RemoteBuildArtefactAdapter::new(r2, "--evil").is_err());
    }

    #[test]
    fn observes_empty_when_no_checkouts() {
        let base = runner_with(vec![(
            vec!["ssh".to_string()],
            ArgvOutput {
                exit_code: Some(0),
                stdout: "".to_string(),
                stderr: "".to_string(),
                spawn_error: None,
            },
        )]);
        let adapter = RemoteBuildArtefactAdapter::new(Arc::new(base), "cynic@10.0.0.1").unwrap();
        let obs = adapter.observe().unwrap();
        assert_eq!(obs.items.len(), 0);
    }
}
