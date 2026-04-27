//! HermesXReader — reads Hermes X organ dataset (JSONL + filesystem).
//! Second OrganPort impl. Validates the generic bag pattern holds for N>1.
//! All filesystem I/O runs inside spawn_blocking to avoid stalling tokio.

use crate::domain::organ::{
    Metric, MetricKind, MetricValue, OrganError, OrganHealth, OrganPort, OrganSnapshot,
};
use async_trait::async_trait;
use chrono::Utc;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug)]
pub struct HermesXReader {
    /// Path to ~/.cynic/organs/hermes/x/
    organ_dir: PathBuf,
}

impl HermesXReader {
    pub fn new(organ_dir: PathBuf) -> Self {
        Self { organ_dir }
    }

    fn dataset_path(&self) -> PathBuf {
        self.organ_dir.join("dataset.jsonl")
    }

    fn captures_dir(&self) -> PathBuf {
        self.organ_dir.join("captures")
    }
}

#[async_trait]
impl OrganPort for HermesXReader {
    fn name(&self) -> &str {
        "hermes-x"
    }

    async fn health(&self) -> OrganHealth {
        let organ_dir = self.organ_dir.clone();
        let result = tokio::task::spawn_blocking(move || {
            if !organ_dir.exists() {
                return OrganHealth::Dead {
                    reason: "organ dir not found".to_string(),
                };
            }
            let dataset = organ_dir.join("dataset.jsonl");
            if !dataset.exists() {
                return OrganHealth::Degraded {
                    reason: "dataset.jsonl missing".to_string(),
                };
            }
            // Check if dataset has been written to recently (< 24h)
            match std::fs::metadata(&dataset) {
                Ok(meta) => match meta.modified() {
                    Ok(modified) => {
                        let age = modified.elapsed().unwrap_or(Duration::from_secs(u64::MAX));
                        if age > Duration::from_secs(24 * 3600) {
                            OrganHealth::Degraded {
                                reason: format!("dataset stale ({}h old)", age.as_secs() / 3600),
                            }
                        } else {
                            OrganHealth::Alive
                        }
                    }
                    Err(e) => OrganHealth::Degraded {
                        reason: format!("cannot read mtime: {e}"),
                    },
                },
                Err(e) => OrganHealth::Dead {
                    reason: format!("cannot stat dataset: {e}"),
                },
            }
        })
        .await;

        // K14: spawn failure = assume degraded
        result.unwrap_or(OrganHealth::Degraded {
            reason: "spawn_blocking panicked".to_string(),
        })
    }

    async fn freshness(&self) -> Result<Duration, OrganError> {
        let dataset = self.dataset_path();
        tokio::task::spawn_blocking(move || {
            let meta = std::fs::metadata(&dataset)
                .map_err(|e| OrganError::Unavailable(format!("dataset.jsonl: {e}")))?;
            let modified = meta
                .modified()
                .map_err(|e| OrganError::ReadFailed(format!("mtime: {e}")))?;
            Ok(modified.elapsed().unwrap_or(Duration::ZERO))
        })
        .await
        .map_err(|e| OrganError::ReadFailed(format!("spawn_blocking: {e}")))?
    }

    async fn snapshot(&self) -> Result<OrganSnapshot, OrganError> {
        let dataset = self.dataset_path();
        let captures_dir = self.captures_dir();

        tokio::task::spawn_blocking(move || {
            // Count captures (raw intercept files)
            let captures_count = std::fs::read_dir(&captures_dir)
                .map(|entries| entries.count() as i64)
                .unwrap_or(0);

            // Parse dataset.jsonl — line-by-line, extract aggregate metrics
            let file = std::fs::File::open(&dataset)
                .map_err(|e| OrganError::Unavailable(format!("dataset.jsonl: {e}")))?;
            let reader = BufReader::new(file);

            let mut tweets_total: i64 = 0;
            let mut authors = std::collections::HashSet::new();
            let mut signal_sum: f64 = 0.0;
            let mut coordinated_count: i64 = 0;
            let mut latest_ts = String::new();

            for line in reader.lines() {
                let line = line.map_err(|e| OrganError::ReadFailed(format!("read line: {e}")))?;
                if line.is_empty() {
                    continue;
                }
                // Lightweight parse — extract only what we need via serde_json::Value
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&line) {
                    tweets_total += 1;
                    if let Some(author) = obj.get("author_screen_name").and_then(|v| v.as_str()) {
                        authors.insert(author.to_string());
                    }
                    if let Some(score) = obj.get("signal_score").and_then(|v| v.as_f64()) {
                        signal_sum += score;
                    }
                    if obj
                        .get("is_coordinated")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        coordinated_count += 1;
                    }
                    if let Some(ts) = obj
                        .get("capture_ts")
                        .and_then(|v| v.as_str())
                        .filter(|ts| *ts > latest_ts.as_str())
                    {
                        latest_ts = ts.to_string();
                    }
                }
            }

            let avg_signal = if tweets_total > 0 {
                signal_sum / tweets_total as f64
            } else {
                0.0
            };

            let dataset_bytes = std::fs::metadata(&dataset)
                .map(|m| m.len() as i64)
                .unwrap_or(0);

            Ok(OrganSnapshot {
                taken_at: Utc::now(),
                metrics: vec![
                    Metric {
                        key: "tweets_total".into(),
                        value: MetricValue::I64(tweets_total),
                        kind: MetricKind::Counter,
                        unit: Some("count".into()),
                    },
                    Metric {
                        key: "unique_authors".into(),
                        value: MetricValue::I64(authors.len() as i64),
                        kind: MetricKind::Gauge,
                        unit: Some("count".into()),
                    },
                    Metric {
                        key: "avg_signal_score".into(),
                        value: MetricValue::F64((avg_signal * 100.0).round() / 100.0),
                        kind: MetricKind::Gauge,
                        unit: None,
                    },
                    Metric {
                        key: "coordinated_count".into(),
                        value: MetricValue::I64(coordinated_count),
                        kind: MetricKind::Counter,
                        unit: Some("count".into()),
                    },
                    Metric {
                        key: "captures_total".into(),
                        value: MetricValue::I64(captures_count),
                        kind: MetricKind::Counter,
                        unit: Some("count".into()),
                    },
                    Metric {
                        key: "dataset_bytes".into(),
                        value: MetricValue::I64(dataset_bytes),
                        kind: MetricKind::Gauge,
                        unit: Some("bytes".into()),
                    },
                ],
            })
        })
        .await
        .map_err(|e| OrganError::ReadFailed(format!("spawn_blocking: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_dead_when_dir_missing() {
        let reader = HermesXReader::new(PathBuf::from("/tmp/nonexistent_hermes_x_test"));
        assert!(matches!(reader.health().await, OrganHealth::Dead { .. }));
    }

    #[tokio::test]
    async fn freshness_fails_when_dir_missing() {
        let reader = HermesXReader::new(PathBuf::from("/tmp/nonexistent_hermes_x_test"));
        assert!(reader.freshness().await.is_err());
    }

    #[tokio::test]
    async fn snapshot_fails_when_dir_missing() {
        let reader = HermesXReader::new(PathBuf::from("/tmp/nonexistent_hermes_x_test"));
        assert!(reader.snapshot().await.is_err());
    }
}
