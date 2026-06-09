#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration test for HermesXReader against real organ data.
//! #[ignore] — requires Hermes X organ at ~/.cynic/organs/hermes/x/

use cynic_kernel::domain::organ::{MetricKind, MetricValue, OrganHealth, OrganPort};
use cynic_kernel::senses::hermes_x::HermesXReader;
use std::path::PathBuf;

fn hermes_x_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".cynic/organs/hermes/x")
}

#[tokio::test]
#[ignore]
async fn hermes_x_health_alive_with_real_data() {
    let dir = hermes_x_dir();
    if !dir.exists() {
        // Organ dir not found — skip silently (test is #[ignore] anyway)
        return;
    }
    let reader = HermesXReader::new(dir);
    let health = reader.health().await;
    // Alive or Degraded (stale) — both are valid for existing data
    assert!(
        !matches!(health, OrganHealth::Dead { .. }),
        "expected Alive or Degraded, got Dead"
    );
}

#[tokio::test]
#[ignore]
async fn hermes_x_snapshot_returns_6_metrics() {
    let dir = hermes_x_dir();
    if !dir.exists() {
        return;
    }
    let reader = HermesXReader::new(dir);
    let snap = reader.snapshot().await.unwrap();

    assert_eq!(snap.metrics.len(), 6);

    let keys: Vec<&str> = snap.metrics.iter().map(|m| m.key.as_str()).collect();
    assert!(keys.contains(&"tweets_total"));
    assert!(keys.contains(&"unique_authors"));
    assert!(keys.contains(&"avg_signal_score"));
    assert!(keys.contains(&"coordinated_count"));
    assert!(keys.contains(&"captures_total"));
    assert!(keys.contains(&"dataset_bytes"));

    // tweets_total should be > 0 (we know there's ~1791 tweets)
    let tweets = snap
        .metrics
        .iter()
        .find(|m| m.key == "tweets_total")
        .unwrap();
    match tweets.value {
        MetricValue::I64(v) => assert!(v > 0, "tweets_total should be positive: {v}"),
        _ => panic!("tweets_total should be I64"),
    }
    assert!(matches!(tweets.kind, MetricKind::Counter));

    // unique_authors should be a Gauge
    let authors = snap
        .metrics
        .iter()
        .find(|m| m.key == "unique_authors")
        .unwrap();
    assert!(matches!(authors.kind, MetricKind::Gauge));
}

#[tokio::test]
#[ignore]
async fn hermes_x_readonly_preserves_data() {
    let dir = hermes_x_dir();
    if !dir.exists() {
        return;
    }
    let reader = HermesXReader::new(dir);

    let before = reader.snapshot().await.unwrap();
    let count_before = before
        .metrics
        .iter()
        .find(|m| m.key == "tweets_total")
        .map(|m| match m.value {
            MetricValue::I64(v) => v,
            _ => -1,
        })
        .unwrap();

    // 5 rapid reads
    for _ in 0..5 {
        let _ = reader.snapshot().await;
    }

    let after = reader.snapshot().await.unwrap();
    let count_after = after
        .metrics
        .iter()
        .find(|m| m.key == "tweets_total")
        .map(|m| match m.value {
            MetricValue::I64(v) => v,
            _ => -1,
        })
        .unwrap();

    assert_eq!(
        count_before, count_after,
        "read-only access should not change data"
    );
}
