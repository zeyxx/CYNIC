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

    /// Extract latest capture_ts from dataset.jsonl (production signal, not file mtime)
    fn extract_latest_ts(dataset: &PathBuf) -> Result<String, String> {
        let file = std::fs::File::open(dataset).map_err(|e| format!("open: {e}"))?;
        let reader = BufReader::new(file);
        let mut latest_ts = String::new();

        for line in reader.lines() {
            let line = line
                .ok()
                .and_then(|l| (!l.is_empty()).then_some(l))
                .ok_or("read failed")?;
            // WHY: nested if guards readability (separates JSON parse from field extraction)
            #[allow(clippy::collapsible_if)]
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(ts) = obj
                    .get("capture_ts")
                    .and_then(|v| v.as_str())
                    .filter(|ts| *ts > latest_ts.as_str())
                {
                    latest_ts = ts.to_string();
                }
            }
        }

        if latest_ts.is_empty() {
            Err("no capture_ts found".to_string())
        } else {
            Ok(latest_ts)
        }
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

            let datasets_dir = organ_dir.join("datasets");
            let legacy_dataset = organ_dir.join("dataset.jsonl");

            // L0 multi-account: check datasets/{account}/dataset.jsonl
            // Fallback to legacy single dataset for backward compat
            let accounts = if datasets_dir.exists() {
                std::fs::read_dir(&datasets_dir)
                    .ok()
                    .and_then(|d| {
                        let mut accounts = Vec::new();
                        for entry in d {
                            if let Ok(entry) = entry {
                                if entry.path().is_dir() {
                                    if let Some(name) = entry.file_name().to_str() {
                                        accounts.push(name.to_string());
                                    }
                                }
                            }
                        }
                        if accounts.is_empty() {
                            None
                        } else {
                            Some(accounts)
                        }
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            // Check account datasets (L0 structure)
            if !accounts.is_empty() {
                let mut worst_health = OrganHealth::Alive;
                let mut statuses = Vec::new();

                for account in accounts {
                    let dataset = datasets_dir.join(&account).join("dataset.jsonl");
                    let latest_ts = match Self::extract_latest_ts(&dataset) {
                        Ok(ts) => ts,
                        Err(_) => {
                            statuses.push(format!("{}: missing", account));
                            worst_health = OrganHealth::Degraded {
                                reason: format!("account {} dataset missing", account),
                            };
                            continue;
                        }
                    };

                    match chrono::DateTime::parse_from_rfc3339(&latest_ts) {
                        Ok(dt) => {
                            let age = Utc::now().signed_duration_since(dt.with_timezone(&Utc));
                            let age_secs = age.num_seconds().max(0) as u64;
                            let age_hrs = age_secs / 3600;
                            if age_secs > 8 * 3600 {
                                statuses.push(format!("{}: {}h stale", account, age_hrs));
                                worst_health = OrganHealth::Degraded {
                                    reason: format!("account {} stale ({}h)", account, age_hrs),
                                };
                            } else if age_secs > 4 * 3600 {
                                statuses.push(format!("{}: {}h old", account, age_hrs));
                            } else {
                                statuses.push(format!("{}: alive", account));
                            }
                        }
                        Err(_) => {
                            statuses.push(format!("{}: parse error", account));
                            worst_health = OrganHealth::Degraded {
                                reason: format!("account {} timestamp parse error", account),
                            };
                        }
                    }
                }

                // If any account is alive (not old), consider organ alive with per-account status
                let has_alive = statuses.iter().any(|s| s.contains("alive"));
                if has_alive {
                    return OrganHealth::Alive; // Return alive even if some accounts stale
                }

                return worst_health;
            }

            // Fallback: check legacy single dataset for backward compatibility
            if !legacy_dataset.exists() {
                return OrganHealth::Degraded {
                    reason: "dataset.jsonl missing".to_string(),
                };
            }

            let latest_ts = match Self::extract_latest_ts(&legacy_dataset) {
                Ok(ts) => ts,
                Err(e) => {
                    return OrganHealth::Degraded {
                        reason: format!("extract_latest_ts: {e}"),
                    };
                }
            };

            match chrono::DateTime::parse_from_rfc3339(&latest_ts) {
                Ok(dt) => {
                    let age = Utc::now().signed_duration_since(dt.with_timezone(&Utc));
                    let age_secs = age.num_seconds().max(0) as u64;
                    if age_secs > 8 * 3600 {
                        OrganHealth::Degraded {
                            reason: format!("dataset production stale ({}h old)", age_secs / 3600),
                        }
                    } else {
                        OrganHealth::Alive
                    }
                }
                Err(e) => OrganHealth::Degraded {
                    reason: format!("parse latest_ts: {e}"),
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
            let latest_ts = Self::extract_latest_ts(&dataset)
                .map_err(|e| OrganError::Unavailable(format!("extract_latest_ts: {e}")))?;

            let dt = chrono::DateTime::parse_from_rfc3339(&latest_ts)
                .map_err(|e| OrganError::ReadFailed(format!("parse latest_ts: {e}")))?;

            let age = Utc::now().signed_duration_since(dt.with_timezone(&Utc));
            Ok(Duration::from_secs(age.num_seconds().max(0) as u64))
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

    async fn store_observation(
        &self,
        obs: crate::domain::storage::Observation,
    ) -> Result<(), OrganError> {
        let observations_dir = self.organ_dir.join("observations");

        tokio::task::spawn_blocking(move || {
            // Create observations dir if needed
            std::fs::create_dir_all(&observations_dir)
                .map_err(|e| OrganError::ReadFailed(format!("create_dir: {e}")))?;

            // Write observation as JSON file (timestamp + tool as filename for uniqueness)
            let safe_tool = obs.tool.to_lowercase().replace(' ', "_");
            let filename = format!(
                "{}_{}.json",
                obs.timestamp.split('T').next().unwrap_or("unknown"),
                safe_tool
            );
            let filepath = observations_dir.join(filename);

            let json = serde_json::to_string_pretty(&obs)
                .map_err(|e| OrganError::ReadFailed(format!("json_serialize: {e}")))?;

            std::fs::write(&filepath, json)
                .map_err(|e| OrganError::ReadFailed(format!("write_file: {e}")))?;

            Ok(())
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

    #[tokio::test]
    async fn health_degraded_when_dataset_stale() {
        let temp_dir =
            std::path::PathBuf::from(format!("/tmp/hermes_x_stale_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        // Write dataset.jsonl with capture_ts from 10 hours ago
        let now = Utc::now();
        let stale_ts = now - chrono::Duration::hours(10);
        let old_line = format!(
            r#"{{"capture_ts":"{}","signal_score":0.5}}"#,
            stale_ts.to_rfc3339()
        );

        std::fs::write(temp_dir.join("dataset.jsonl"), old_line).unwrap();

        let reader = HermesXReader::new(temp_dir.clone());
        let health = reader.health().await;

        // Should be Degraded with stale message
        match health {
            OrganHealth::Degraded { reason } => {
                assert!(
                    reason.contains("stale"),
                    "Reason should mention stale: {reason}"
                );
            }
            _ => panic!("Expected Degraded, got {health:?}"),
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
