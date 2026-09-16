//! Cleanup host agent — observe disk state, reconcile, apply via ArgvRunner.
//!
//! Pattern:
//!   observe() -> CleanupObservation (via ArgvRunner SSH)
//!   reconcile_cleanup() -> CleanupDecision (pure)
//!   apply() -> CleanupProof (via ArgvRunner SSH)

use cynic_protocol::cleanup::*;
use cynic_lifecycle::{ArgvRunner, LocalArgvRunner};

pub struct CleanupAgent<R: ArgvRunner> {
    runner: R,
}

impl CleanupAgent<LocalArgvRunner> {
    pub fn new() -> Self {
        Self {
            runner: LocalArgvRunner,
        }
    }
}

impl<R: ArgvRunner> CleanupAgent<R> {
    pub fn with_runner(runner: R) -> Self {
        Self { runner }
    }

    /// Observe disk state via SSH commands.
    pub fn observe(&self) -> Result<CleanupObservation, String> {
        let df_output = self.runner.run(&["ssh", "cynic-builder-ssh", "df", "-B1", "/"]);
        let parts: Vec<&str> = df_output.stdout.split_whitespace().collect();
        if parts.len() < 10 {
            return Err(format!("unexpected df output: {}", df_output.stdout));
        }
        let total: u64 = parts[8].parse().map_err(|_| format!("parse total: {}", parts[8]))?;
        let used: u64 = parts[9].parse().map_err(|_| format!("parse used: {}", parts[9]))?;
        let available: u64 = parts[10].parse().map_err(|_| format!("parse available: {}", parts[10]))?;
        let disk_used_percent = ((used as f64 / total as f64) * 100.0) as u8;

        let du_output = self.runner.run(&["ssh", "cynic-builder-ssh", "du", "-sh", "/var/lib/cynic/checkouts/*"]);
        let mut items = Vec::new();
        for line in du_output.stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                let path = parts[1].to_string();
                let size = parse_size(parts[0]);
                let target_kind = infer_target_kind(&path);
                items.push(CleanupItem {
                    schema: CLEANUP_ITEM_SCHEMA.to_string(),
                    target_kind,
                    path,
                    size_bytes: Some(size),
                    last_modified_unix_s: None,
                    branch: None,
                    rationale: "Observed on disk".to_string(),
                });
            }
        }

        let reclaimable_bytes = items.iter().filter_map(|i| i.size_bytes).sum();

        Ok(CleanupObservation {
            schema: CLEANUP_OBSERVATION_SCHEMA.to_string(),
            disk_used_percent,
            disk_available_bytes: available,
            disk_total_bytes: total,
            items,
            reclaimable_bytes,
        })
    }

    /// Apply cleanup decision via SSH.
    pub fn apply(&self, decision: &CleanupDecision) -> Result<CleanupProof, String> {
        let mut deleted = Vec::new();
        let mut failed = Vec::new();

        for item in &decision.safe_to_delete {
            let rm_cmd = format!("rm -rf {}", item.path);
            let output = self.runner.run(&["ssh", "cynic-builder-ssh", &rm_cmd]);
            if output.exit_code == Some(0) {
                deleted.push(item.path.clone());
            } else {
                failed.push((item.path.clone(), output.stderr));
            }
        }

        let df_after = self.runner.run(&["ssh", "cynic-builder-ssh", "df", "-B1", "/"]);
        let parts: Vec<&str> = df_after.stdout.split_whitespace().collect();
        let disk_used_percent_after = if parts.len() >= 10 {
            let total: u64 = parts[8].parse().unwrap_or(1);
            let used: u64 = parts[9].parse().unwrap_or(0);
            ((used as f64 / total as f64) * 100.0) as u8
        } else {
            0
        };

        let space_freed_bytes = decision.estimated_savings_bytes;

        Ok(CleanupProof {
            schema: CLEANUP_PROOF_SCHEMA.to_string(),
            deleted,
            failed,
            space_freed_bytes,
            disk_used_percent_after,
        })
    }
}

fn parse_size(s: &str) -> u64 {
    let s = s.trim();
    if let Some(num) = s.strip_suffix('G') {
        num.parse::<f64>().map(|n| (n * 1_000_000_000.0) as u64).unwrap_or(0)
    } else if let Some(num) = s.strip_suffix('M') {
        num.parse::<f64>().map(|n| (n * 1_000_000.0) as u64).unwrap_or(0)
    } else if let Some(num) = s.strip_suffix('K') {
        num.parse::<f64>().map(|n| (n * 1_000.0) as u64).unwrap_or(0)
    } else {
        s.parse::<u64>().unwrap_or(0)
    }
}

fn infer_target_kind(path: &str) -> CleanupTargetKind {
    if path.contains("checkouts/") {
        let name = path.split('/').last().unwrap_or("");
        if name.len() == 40 {
            // SHA-like directory name = incomplete or orphaned
            CleanupTargetKind::IncompleteCheckout
        } else {
            CleanupTargetKind::OrphanedCheckout
        }
    } else {
        CleanupTargetKind::BuildArtifact
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::MapArgvRunner;

    fn fake_runner() -> MapArgvRunner {
        MapArgvRunner {
            responses: vec![
                (vec!["ssh".to_string(), "cynic-builder-ssh".to_string(), "df".to_string(), "-B1".to_string(), "/".to_string()],
                 cynic_lifecycle::ArgvOutput {
                     exit_code: Some(0),
                     stdout: "Filesystem                        Size  Used  Avail Use% Mounted on\n/dev/mapper/pve-vm--105--disk--0   40G   38G     0 100% /".to_string(),
                     stderr: String::new(),
                     spawn_error: None,
                 }),
                (vec!["ssh".to_string(), "cynic-builder-ssh".to_string(), "du".to_string(), "-sh".to_string(), "/var/lib/cynic/checkouts/*".to_string()],
                 cynic_lifecycle::ArgvOutput {
                     exit_code: Some(0),
                     stdout: "2.5G\t/var/lib/cynic/checkouts/abc123\n402M\t/var/lib/cynic/checkouts/def456".to_string(),
                     stderr: String::new(),
                     spawn_error: None,
                 }),
            ],
        }
    }

    #[test]
    fn cleanup_agent_observe() {
        let agent = CleanupAgent::with_runner(fake_runner());
        let obs = agent.observe().unwrap();
        assert_eq!(obs.disk_used_percent, 100);
        assert_eq!(obs.items.len(), 2);
    }

    #[test]
    fn parse_size_gb() {
        assert_eq!(parse_size("2.5G"), 2_500_000_000);
    }

    #[test]
    fn parse_size_mb() {
        assert_eq!(parse_size("402M"), 402_000_000);
    }

    #[test]
    fn infer_target_kind_identifies_sha_dirs() {
        assert_eq!(
            infer_target_kind("/var/lib/cynic/checkouts/36b640d4833e52cbb69f9d03dfc6eb72fd2a2a47"),
            CleanupTargetKind::IncompleteCheckout
        );
    }
}
