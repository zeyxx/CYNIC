use std::path::PathBuf;

use crate::domain::probe::{Probe, ProbeDetails, ProbeStatus};

use super::BackupProbe;

#[tokio::test]
async fn backup_probe_missing_dir_returns_unavailable() {
    let probe = BackupProbe::new(PathBuf::from("/tmp/nonexistent-cynic-test-99999"));
    let result = probe.sense().await.expect("sense must not return Err");
    assert_eq!(result.status, ProbeStatus::Unavailable);
}

#[tokio::test]
async fn backup_probe_empty_dir_returns_degraded() {
    let tmp = std::env::temp_dir().join("cynic-backup-probe-empty-test");
    tokio::fs::create_dir_all(&tmp)
        .await
        .expect("create tmp dir");

    let probe = BackupProbe::new(tmp.clone());
    let result = probe.sense().await.expect("sense must not return Err");

    // Cleanup before asserting so we don't leave garbage on failure.
    let _ = tokio::fs::remove_dir_all(&tmp).await;

    assert_eq!(result.status, ProbeStatus::Degraded);
}

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
