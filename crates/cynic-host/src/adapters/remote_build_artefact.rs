//! Remote build artefact observation over SSH.
//!
//! Reuses the pure `parse_checkout_list` / `parse_df_pressure` parsers shared
//! with `LocalBuildArtefactAdapter`; the transport (SSH) is fully injected
//! via `ArgvRunner` so unit tests inject `MapArgvRunner` returning canned
//! `ls`/`du`/`df`/`git ls-remote` stdout.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use crate::domain::build_artefact::port::BuildArtefactObservationPort;
use crate::domain::build_artefact::probe::{build_gc_plan, parse_checkout_list, parse_df_pressure};
use crate::domain::build_artefact::{BuildArtefactGcPlan, BuildArtefactReport};
use crate::domain::disk::probe::{parse_df_filesystems, parse_lsblk_disks};
use cynic_build::build_artefact::BUILD_ARTEFACT_REPORT_SCHEMA;
use cynic_lifecycle::ArgvRunner;
use cynic_protocol::disk::bytes_to_human;

const DEFAULT_CHECKOUTS_BASE: &str = "/var/lib/cynic/checkouts";

pub struct RemoteBuildArtefactAdapter<R: ArgvRunner> {
    runner: Arc<R>,
    ssh_target: String,
    checkouts_base: String,
}

impl<R: ArgvRunner> RemoteBuildArtefactAdapter<R> {
    pub fn new(runner: Arc<R>, ssh_target: impl Into<String>) -> Result<Self, String> {
        let ssh_target = ssh_target.into();
        if ssh_target.trim().is_empty() || ssh_target.starts_with('-') {
            return Err("invalid_ssh_target".to_string());
        }
        Ok(Self {
            runner,
            ssh_target,
            checkouts_base: DEFAULT_CHECKOUTS_BASE.to_string(),
        })
    }

    /// Bind the adapter to an explicit checkout base observed over SSH.
    ///
    /// The default keeps the production placement, so existing call sites are
    /// unchanged; the override is what lets a test prove the base reaches the
    /// argv instead of a constant leaking past the port.
    pub fn with_checkouts_base(mut self, base: impl Into<String>) -> Self {
        self.checkouts_base = base.into();
        self
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

impl<R: ArgvRunner> BuildArtefactObservationPort for RemoteBuildArtefactAdapter<R> {
    fn observe_build_artefacts(&self) -> Result<BuildArtefactReport, String> {
        let timestamp = DateTime::<Utc>::from(SystemTime::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        // Physical + filesystems best-effort (empty on failure, not fatal)
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

        // Checkouts: list + per-dir meta via one ssh shell
        let raw_list = self
            .ssh_run(&["ls", "-1", &self.checkouts_base])
            .unwrap_or_default();
        // For each checkout name, fetch sha/size/owner/age via ssh
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
            let path = format!("{}/{name}", self.checkouts_base);
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
            .ssh_run(&["df", "-k", &self.checkouts_base])
            .or_else(|_| self.ssh_run(&["df", "-k", "/"]))
            .unwrap_or_default();
        let root_owned = checkouts.iter().filter(|c| c.is_root_owned).count();
        let pressure = parse_df_pressure(&df_k, root_owned);

        Ok(BuildArtefactReport {
            schema: BUILD_ARTEFACT_REPORT_SCHEMA.to_string(),
            timestamp,
            physical_disks,
            filesystems,
            checkouts,
            total_checkout_bytes,
            total_checkout_human: bytes_to_human(total_checkout_bytes),
            target_bytes: 0,
            target_human: "0B".to_string(),
            pressure,
            residual_unknowns: vec![],
        })
    }

    fn plan_gc(&self, report: &BuildArtefactReport) -> BuildArtefactGcPlan {
        build_gc_plan(report)
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
        let report = adapter.observe_build_artefacts().unwrap();
        assert_eq!(report.checkouts.len(), 0);
    }

    /// The override must reach the remote argv, not sit beside it: the
    /// `ls`/`df` lines the adapter builds are read back from the injected
    /// runner, so a base that failed to thread through would show up here as
    /// the production constant instead of the override.
    #[test]
    fn override_checkouts_base_reaches_remote_argv() {
        use std::sync::Mutex;

        #[derive(Default)]
        struct RecordingRunner {
            seen: Mutex<Vec<Vec<String>>>,
        }

        impl cynic_lifecycle::ArgvRunner for RecordingRunner {
            fn run(&self, argv: &[&str]) -> cynic_protocol::argv::ArgvOutput {
                self.seen
                    .lock()
                    .expect("recording lock")
                    .push(argv.iter().map(|s| s.to_string()).collect());
                cynic_protocol::argv::ArgvOutput {
                    exit_code: Some(0),
                    stdout: String::new(),
                    stderr: String::new(),
                    spawn_error: None,
                }
            }
        }

        let runner = Arc::new(RecordingRunner::default());
        let adapter = RemoteBuildArtefactAdapter::new(runner.clone(), "cynic@10.0.0.1")
            .unwrap()
            .with_checkouts_base("/srv/overridden/checkouts");
        let report = adapter.observe_build_artefacts().unwrap();
        assert_eq!(report.checkouts.len(), 0);

        let seen = runner.seen.lock().expect("recording lock").clone();
        let rendered: Vec<String> = seen.iter().map(|argv| argv.join(" ")).collect();
        assert!(
            rendered
                .iter()
                .any(|argv| argv.contains("ls -1 /srv/overridden/checkouts")),
            "override base never reached the ls argv: {rendered:?}"
        );
        assert!(
            rendered
                .iter()
                .any(|argv| argv.contains("df -k /srv/overridden/checkouts")),
            "override base never reached the df argv: {rendered:?}"
        );
        assert!(
            !rendered
                .iter()
                .any(|argv| argv.contains("/var/lib/cynic/checkouts")),
            "the production constant leaked into the argv despite the override: {rendered:?}"
        );
    }
}
