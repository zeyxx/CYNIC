#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Contract tests for OrganPort — verifies the trait is object-safe,
//! can be boxed, and the types work as expected.

use async_trait::async_trait;
use chrono::Utc;
use cynic_kernel::domain::organ::{
    Metric, MetricKind, MetricValue, OrganError, OrganHealth, OrganPort, OrganSnapshot,
};
use std::time::Duration;

struct StubOrgan;

#[async_trait]
impl OrganPort for StubOrgan {
    fn name(&self) -> &str {
        "stub"
    }

    async fn health(&self) -> OrganHealth {
        OrganHealth::Alive
    }

    async fn freshness(&self) -> Result<Duration, OrganError> {
        Ok(Duration::from_secs(60))
    }

    async fn snapshot(&self) -> Result<OrganSnapshot, OrganError> {
        Ok(OrganSnapshot {
            taken_at: Utc::now(),
            metrics: vec![
                Metric {
                    key: "test_counter".to_string(),
                    value: MetricValue::I64(42),
                    kind: MetricKind::Counter,
                    unit: Some("count".to_string()),
                },
                Metric {
                    key: "test_gauge".to_string(),
                    value: MetricValue::F64(0.95),
                    kind: MetricKind::Gauge,
                    unit: Some("%".to_string()),
                },
            ],
        })
    }
}

#[tokio::test]
async fn trait_is_object_safe_and_boxable() {
    let organ: Box<dyn OrganPort> = Box::new(StubOrgan);
    assert_eq!(organ.name(), "stub");
    assert!(matches!(organ.health().await, OrganHealth::Alive));
}

#[tokio::test]
async fn snapshot_returns_typed_metrics() {
    let organ = StubOrgan;
    let snap = organ.snapshot().await.unwrap();
    assert_eq!(snap.metrics.len(), 2);
    assert!(matches!(snap.metrics[0].kind, MetricKind::Counter));
    assert!(matches!(snap.metrics[1].kind, MetricKind::Gauge));
    assert!(matches!(snap.metrics[0].value, MetricValue::I64(42)));
}

#[tokio::test]
async fn freshness_returns_duration() {
    let organ = StubOrgan;
    let fresh = organ.freshness().await.unwrap();
    assert_eq!(fresh.as_secs(), 60);
}

#[tokio::test]
async fn health_degraded_carries_reason() {
    struct DegradedOrgan;

    #[async_trait]
    impl OrganPort for DegradedOrgan {
        fn name(&self) -> &str {
            "degraded"
        }
        async fn health(&self) -> OrganHealth {
            OrganHealth::Degraded {
                reason: "db locked".to_string(),
            }
        }
        async fn freshness(&self) -> Result<Duration, OrganError> {
            Err(OrganError::Unavailable("not reachable".to_string()))
        }
        async fn snapshot(&self) -> Result<OrganSnapshot, OrganError> {
            Err(OrganError::ReadFailed("timeout".to_string()))
        }
    }

    let organ = DegradedOrgan;
    match organ.health().await {
        OrganHealth::Degraded { reason } => assert_eq!(reason, "db locked"),
        _ => panic!("expected Degraded"),
    }
    assert!(organ.freshness().await.is_err());
    assert!(organ.snapshot().await.is_err());
}

#[tokio::test]
async fn organ_error_display() {
    let e = OrganError::Unavailable("gone".to_string());
    assert_eq!(format!("{e}"), "organ unavailable: gone");
    let e = OrganError::ReadFailed("io".to_string());
    assert_eq!(format!("{e}"), "organ read failed: io");
}
