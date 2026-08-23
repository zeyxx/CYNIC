//! Remote disk-space observation over SSH.
//!
//! `RemoteDiskAdapter` implements [`DiskObservationPort`] by shelling out to
//! `df` on a remote host through an injectable [`ArgvRunner`]. It reuses the
//! pure [`parse_df_filesystems`] parser shared with [`LocalDiskAdapter`], so the
//! two adapters never diverge on column layout. The transport (SSH invocation)
//! is fully injected: unit tests inject a `MapArgvRunner` returning canned `df`
//! stdout, avoiding a real `cynic` binary on PATH.
//!
//! Known-cache / large-file enumeration is intentionally localhost-specific
//! (it walks `$HOME`, `/tmp`, `/var/log`, ...). For a remote node we surface
//! only the portable filesystem table from `df`; richer per-node detail is left
//! to that node's own `cynic runtime disk` run (which reports the full
//! `DiskReport`). This keeps the adapter minimal and portable.

use std::time::SystemTime;

use chrono::{DateTime, Utc};

use crate::domain::disk::probe::parse_df_filesystems;
use crate::domain::disk::{
    DiskCleanupPlan, DiskReport, DiskObservationPort,
};
use crate::update_lifecycle::ArgvRunner;

use std::sync::Arc;

/// Adapter that observes disk usage on a remote host via SSH.
///
/// The SSH transport is provided through the injected [`ArgvRunner`], so this
/// adapter is safe to unit test without a live `cynic` binary or a real SSH key.
pub struct RemoteDiskAdapter<R: ArgvRunner> {
    runner: Arc<R>,
    ssh_target: String,
}

impl<R: ArgvRunner> RemoteDiskAdapter<R> {
    /// Build a remote adapter for the given SSH target (e.g. `cynic@10.0.0.7`).
    /// Returns `Err` for an empty or option-like target, mirroring
    /// [`crate::adapters::proxmox_substrate::ProxmoxSshSubstrateAdapter::new`].
    pub fn new(runner: Arc<R>, ssh_target: impl Into<String>) -> Result<Self, String> {
        let ssh_target = ssh_target.into();
        if ssh_target.trim().is_empty() || ssh_target.starts_with('-') {
            return Err("invalid_ssh_target".to_string());
        }
        Ok(Self {
            runner,
            ssh_target,
        })
    }

    /// Build the argv for `ssh <target> df -h --output=...`.
    fn df_argv(&self) -> Vec<&str> {
        [
            "ssh",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            "-o",
            "StrictHostKeyChecking=yes",
            &self.ssh_target,
            "df",
            "-h",
            "--output=source,fstype,size,used,avail,pcent,target",
        ]
        .iter()
        .copied()
        .collect()
    }

    fn run_df(&self) -> Result<String, String> {
        let argv = self.df_argv();
        let output = self.runner.run(&argv);
        if output.success() {
            Ok(output.stdout)
        } else {
            Err(format!(
                "remote_df_failed: {}",
                output.spawn_error.clone().unwrap_or(output.stderr)
            ))
        }
    }
}

impl<R: ArgvRunner> DiskObservationPort for RemoteDiskAdapter<R> {
    fn observe_disk(&self) -> Result<DiskReport, String> {
        let timestamp = DateTime::<Utc>::from(SystemTime::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let stdout = self.run_df()?;
        let filesystems = parse_df_filesystems(&stdout);

        // Remote nodes cannot walk local caches / large-file trees portably;
        // leave them empty so the report stays truthful and conservative.
        Ok(DiskReport {
            schema: "cynic.host_disk_report.v1".to_string(),
            timestamp,
            filesystems,
            known_caches: Vec::new(),
            large_files: Vec::new(),
        })
    }

    fn plan_cleanup(&self, report: &DiskReport) -> DiskCleanupPlan {
        crate::domain::disk::probe::build_cleanup_plan(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};

    /// Sample `df -h --output=...` stdout (Linux) returning two real + one tmpfs.
    const SAMPLE_DF: &str = "\
Filesystem     Type  Size  Used Avail Use% Mounted on
/dev/sda2      ext4   50G   30G   20G  60% /
tmpfs          tmpfs  3.9G     0  3.9G   0% /dev
/dev/sdb1      xfs    20G   18G  2.0G  90% /mnt/data
";

    fn stub_runner(stdout: &str) -> MapArgvRunner {
        // Match any argv that *starts with* the SSH/df prefix. MapArgvRunner
        // does prefix matching on the collected key.
        let key: Vec<String> = ["ssh"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        MapArgvRunner {
            responses: vec![(
                key,
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: stdout.to_string(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        }
    }

    #[test]
    fn parses_remote_df_output() {
        let runner = Arc::new(stub_runner(SAMPLE_DF));
        // The MapArgvRunner prefix-matches on `["ssh"]`, so any ssh argv hits it.
        let adapter = RemoteDiskAdapter::new(runner, "cynic@10.0.0.7").unwrap();

        let report = adapter.observe_disk().expect("observe should succeed");
        assert_eq!(report.schema, "cynic.host_disk_report.v1");
        // tmpfs + /dev/* backed store + xfs mount -> 3 entries
        assert_eq!(report.filesystems.len(), 3);
        assert_eq!(report.filesystems[0].source, "/dev/sda2");
        assert_eq!(report.filesystems[0].fstype, "ext4");
        assert_eq!(report.filesystems[0].use_pct, "60%");
        assert_eq!(report.filesystems[1].source, "tmpfs");
        assert_eq!(report.filesystems[2].fstype, "xfs");
        assert!(report.known_caches.is_empty());
        assert!(report.large_files.is_empty());
    }

    #[test]
    fn rejects_invalid_ssh_target() {
        let runner = Arc::new(stub_runner(SAMPLE_DF));
        assert!(RemoteDiskAdapter::new(runner, "").is_err());
        let runner2 = Arc::new(stub_runner(SAMPLE_DF));
        assert!(RemoteDiskAdapter::new(runner2, "--bad").is_err());
    }

    #[test]
    fn plan_cleanup_delegates_to_pure_builder() {
        let runner = Arc::new(stub_runner(SAMPLE_DF));
        let adapter = RemoteDiskAdapter::new(runner, "cynic@10.0.0.7").unwrap();
        let report = adapter.observe_disk().unwrap();
        let plan = adapter.plan_cleanup(&report);
        // No caches observed remotely → no reclaimable items.
        assert_eq!(plan.items.len(), 0);
        assert_eq!(plan.total_reclaimable_bytes, 0);
    }

    #[test]
    fn surfaces_ssh_failure() {
        // Runner returns failure (non-zero exit).
        let runner = MapArgvRunner {
            responses: vec![(
                vec!["ssh".to_string()],
                ArgvOutput {
                    exit_code: Some(255),
                    stdout: String::new(),
                    stderr: "Permission denied".to_string(),
                    spawn_error: None,
                },
            )],
        };
        let adapter =
            RemoteDiskAdapter::new(Arc::new(runner), "cynic@10.0.0.7").unwrap();
        let result = adapter.observe_disk();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("remote_df_failed"));
    }
}
