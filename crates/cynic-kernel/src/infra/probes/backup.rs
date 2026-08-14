//! BackupProbe — backup freshness and health sensing.
//! Scans a directory for `.surql.gz` files and reports age, size, and count.

mod scanner;

#[cfg(test)]
mod tests;

use crate::domain::probe::{BackupDetails, ProbeDetails, ProbeError, ProbeResult, ProbeStatus};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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

        let scan_result = scanner::scan_backup_dir(backup_dir).await;

        let duration_ms = start.elapsed().as_millis() as u64;
        let timestamp = chrono::Utc::now().timestamp_millis();

        match scan_result {
            Ok(scanner::ScanResult::NotFound) => Ok(ProbeResult {
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
            Ok(scanner::ScanResult::Denied) => Ok(ProbeResult {
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
            Ok(scanner::ScanResult::OtherError) => Ok(ProbeResult {
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
            Ok(scanner::ScanResult::Empty) => Ok(ProbeResult {
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
            Ok(scanner::ScanResult::Found {
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
