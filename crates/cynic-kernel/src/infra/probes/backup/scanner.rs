//! Backup directory scan adapter.

use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::probe::ProbeError;

pub(super) enum ScanResult {
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

pub(super) async fn scan_backup_dir(dir: PathBuf) -> Result<ScanResult, ProbeError> {
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
