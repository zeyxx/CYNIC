//! BackupProbe — backup freshness and health sensing.
//! Scans a directory for `.surql.gz` files and reports age, size, and count.

use crate::domain::probe::{BackupDetails, ProbeDetails, ProbeError, ProbeResult, ProbeStatus};
use async_trait::async_trait;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Probe that inspects a backup directory for `.surql.gz` files.
#[derive(Debug)]
pub struct BackupProbe {
    backup_dir: PathBuf,
}

impl BackupProbe {
    pub fn new(backup_dir: PathBuf) -> Self {
        Self { backup_dir }
    }
}

#[async_trait]
impl crate::domain::probe::Probe for BackupProbe {
    fn name(&self) -> &str {
        "backup"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(3600)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();
        let backup_dir = self.backup_dir.clone();
        let backup_dir_str = backup_dir.display().to_string();

        let scan_result = scan_backup_dir(backup_dir).await;

        let duration_ms = start.elapsed().as_millis() as u64;
        let timestamp = chrono::Utc::now().to_rfc3339();

        match scan_result {
            Ok(ScanResult::NotFound) => Ok(ProbeResult {
                name: "backup".to_string(),
                status: ProbeStatus::Unavailable,
                details: ProbeDetails::Backup(BackupDetails {
                    last_backup_age_hours: None,
                    last_backup_size_mb: None,
                    backup_count: None,
                    backup_dir: backup_dir_str,
                }),
                duration_ms,
                timestamp,
            }),
            Ok(ScanResult::Denied) => Ok(ProbeResult {
                name: "backup".to_string(),
                status: ProbeStatus::Denied,
                details: ProbeDetails::Backup(BackupDetails {
                    last_backup_age_hours: None,
                    last_backup_size_mb: None,
                    backup_count: None,
                    backup_dir: backup_dir_str,
                }),
                duration_ms,
                timestamp,
            }),
            Ok(ScanResult::OtherError) => Ok(ProbeResult {
                name: "backup".to_string(),
                status: ProbeStatus::Unavailable,
                details: ProbeDetails::Backup(BackupDetails {
                    last_backup_age_hours: None,
                    last_backup_size_mb: None,
                    backup_count: None,
                    backup_dir: backup_dir_str,
                }),
                duration_ms,
                timestamp,
            }),
            Ok(ScanResult::Empty) => Ok(ProbeResult {
                name: "backup".to_string(),
                status: ProbeStatus::Degraded,
                details: ProbeDetails::Backup(BackupDetails {
                    last_backup_age_hours: None,
                    last_backup_size_mb: None,
                    backup_count: Some(0),
                    backup_dir: backup_dir_str,
                }),
                duration_ms,
                timestamp,
            }),
            Ok(ScanResult::Found {
                last_backup_age_hours,
                last_backup_size_mb,
                backup_count,
            }) => Ok(ProbeResult {
                name: "backup".to_string(),
                status: ProbeStatus::Ok,
                details: ProbeDetails::Backup(BackupDetails {
                    last_backup_age_hours: Some(last_backup_age_hours),
                    last_backup_size_mb: Some(last_backup_size_mb),
                    backup_count: Some(backup_count),
                    backup_dir: backup_dir_str,
                }),
                duration_ms,
                timestamp,
            }),
            Err(e) => Err(ProbeError::Internal(format!("backup scan bug: {e}"))),
        }
    }
}

// ─── Internal scan logic ──────────────────────────────────────────────────────

enum ScanResult {
    NotFound,
    Denied,
    OtherError,
    Empty,
    Found {
        last_backup_age_hours: f64,
        last_backup_size_mb: f64,
        backup_count: u32,
    },
}

async fn scan_backup_dir(dir: PathBuf) -> Result<ScanResult, ProbeError> {
    let mut read_dir = match tokio::fs::read_dir(&dir).await {
        Ok(rd) => rd,
        Err(e) => {
            return Ok(match e.kind() {
                io::ErrorKind::NotFound => ScanResult::NotFound,
                io::ErrorKind::PermissionDenied => ScanResult::Denied,
                _ => ScanResult::OtherError,
            });
        }
    };

    let mut latest_modified: Option<SystemTime> = None;
    let mut latest_size_bytes: u64 = 0;
    let mut backup_count: u32 = 0;

    loop {
        let entry = match read_dir.next_entry().await {
            Ok(Some(e)) => e,
            Ok(None) => break,
            Err(_) => continue,
        };

        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if !name.ends_with(".surql.gz") {
            continue;
        }

        let Ok(meta) = entry.metadata().await else {
            continue;
        };

        if !meta.is_file() {
            continue;
        }

        backup_count += 1;
        let modified = meta.modified().unwrap_or(UNIX_EPOCH);
        let size = meta.len();

        match latest_modified {
            None => {
                latest_modified = Some(modified);
                latest_size_bytes = size;
            }
            Some(prev) if modified > prev => {
                latest_modified = Some(modified);
                latest_size_bytes = size;
            }
            _ => {}
        }
    }

    if backup_count == 0 {
        return Ok(ScanResult::Empty);
    }

    let age_secs = latest_modified
        .and_then(|m| SystemTime::now().duration_since(m).ok())
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let last_backup_age_hours = age_secs / 3600.0;
    let last_backup_size_mb = latest_size_bytes as f64 / (1024.0 * 1024.0);

    Ok(ScanResult::Found {
        last_backup_age_hours,
        last_backup_size_mb,
        backup_count,
    })
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::probe::{Probe, ProbeDetails};

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn backup_probe_missing_dir_returns_unavailable() {
        let probe = BackupProbe::new(PathBuf::from("/tmp/nonexistent-cynic-test-99999"));
        let result = probe.sense().await.expect("sense must not return Err");
        assert_eq!(result.status, ProbeStatus::Unavailable);
    }

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn backup_probe_empty_dir_returns_degraded() {
        let tmp = std::env::temp_dir().join("cynic-backup-probe-empty-test");
        tokio::fs::create_dir_all(&tmp)
            .await
            .expect("create tmp dir");

        let probe = BackupProbe::new(tmp.clone());
        let result = probe.sense().await.expect("sense must not return Err");

        // Cleanup before asserting so we don't leave garbage on failure
        let _ = tokio::fs::remove_dir_all(&tmp).await;

        assert_eq!(result.status, ProbeStatus::Degraded);
    }

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn backup_probe_with_file_returns_ok() {
        let tmp = std::env::temp_dir().join("cynic-backup-probe-file-test");
        tokio::fs::create_dir_all(&tmp)
            .await
            .expect("create tmp dir");
        tokio::fs::write(tmp.join("test.surql.gz"), b"fake backup data")
            .await
            .expect("write test file");

        let probe = BackupProbe::new(tmp.clone());
        let result = probe.sense().await.expect("sense must not return Err");

        let _ = tokio::fs::remove_dir_all(&tmp).await;

        assert_eq!(result.status, ProbeStatus::Ok);
        match result.details {
            ProbeDetails::Backup(ref b) => {
                assert_eq!(b.backup_count, Some(1));
            }
            _ => panic!("expected Backup details"),
        }
    }
}
