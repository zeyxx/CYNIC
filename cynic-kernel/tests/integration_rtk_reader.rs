#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration test for RtkReader against real history.db.
//! #[ignore] — requires RTK installed with data.

use cynic_kernel::domain::organ::{MetricKind, MetricValue, OrganHealth, OrganPort};
use cynic_kernel::senses::rtk::RtkReader;
use std::path::PathBuf;

fn rtk_db_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join("rtk/history.db")
}

#[tokio::test]
#[ignore]
async fn rtk_health_alive_with_real_db() {
    let db = rtk_db_path();
    if !db.exists() {
        eprintln!("SKIP: RTK history.db not found at {}", db.display());
        return;
    }
    let reader = RtkReader::new(db, "/home/user/Bureau/CYNIC".to_string());
    assert!(matches!(reader.health().await, OrganHealth::Alive));
}

#[tokio::test]
#[ignore]
async fn rtk_freshness_returns_reasonable_duration() {
    let db = rtk_db_path();
    if !db.exists() {
        return;
    }
    let reader = RtkReader::new(db, "/home/user/Bureau/CYNIC".to_string());
    let fresh = reader.freshness().await.unwrap();
    assert!(
        fresh.as_secs() < 7 * 24 * 3600,
        "freshness too old: {:?}",
        fresh
    );
}

#[tokio::test]
#[ignore]
async fn rtk_snapshot_returns_7_metrics() {
    let db = rtk_db_path();
    if !db.exists() {
        return;
    }
    let reader = RtkReader::new(db, "/home/user/Bureau/CYNIC".to_string());
    let snap = reader.snapshot().await.unwrap();

    assert_eq!(snap.metrics.len(), 7);

    let keys: Vec<&str> = snap.metrics.iter().map(|m| m.key.as_str()).collect();
    assert!(keys.contains(&"commands_total"));
    assert!(keys.contains(&"tokens_saved"));
    assert!(keys.contains(&"savings_pct"));
    assert!(keys.contains(&"parse_failures"));

    // tokens_saved should be > 0 (we know there's 48M+ saved for CYNIC)
    let saved = snap
        .metrics
        .iter()
        .find(|m| m.key == "tokens_saved")
        .unwrap();
    match saved.value {
        MetricValue::I64(v) => assert!(v > 0, "tokens_saved should be positive: {v}"),
        _ => panic!("tokens_saved should be I64"),
    }
    assert!(matches!(saved.kind, MetricKind::Counter));

    // savings_pct should be a gauge between 0 and 100
    let pct = snap
        .metrics
        .iter()
        .find(|m| m.key == "savings_pct")
        .unwrap();
    match pct.value {
        MetricValue::F64(v) => assert!((0.0..=100.0).contains(&v), "savings_pct out of range: {v}"),
        _ => panic!("savings_pct should be F64"),
    }
    assert!(matches!(pct.kind, MetricKind::Gauge));
}

#[tokio::test]
#[ignore]
async fn rtk_readonly_preserves_data_across_reads() {
    let db = rtk_db_path();
    if !db.exists() {
        return;
    }
    let reader = RtkReader::new(db, "/home/user/Bureau/CYNIC".to_string());

    let before = reader.snapshot().await.unwrap();
    let count_before = before
        .metrics
        .iter()
        .find(|m| m.key == "commands_total")
        .map(|m| match m.value {
            MetricValue::I64(v) => v,
            _ => -1,
        })
        .unwrap();

    for _ in 0..10 {
        let _ = reader.snapshot().await;
    }

    let after = reader.snapshot().await.unwrap();
    let count_after = after
        .metrics
        .iter()
        .find(|m| m.key == "commands_total")
        .map(|m| match m.value {
            MetricValue::I64(v) => v,
            _ => -1,
        })
        .unwrap();

    assert_eq!(
        count_before, count_after,
        "read-only access should not change data"
    );
    assert_eq!(after.metrics.len(), 7);
}
