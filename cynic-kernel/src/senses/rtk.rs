//! RtkReader — reads RTK's history.db (SQLite) in read-only mode.
//! All SQLite operations run inside spawn_blocking to avoid stalling tokio.

use crate::domain::organ::{
    Metric, MetricKind, MetricValue, OrganError, OrganHealth, OrganPort, OrganSnapshot,
};
use async_trait::async_trait;
use chrono::Utc;
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug)]
pub struct RtkReader {
    db_path: PathBuf,
    project_root: String,
}

impl RtkReader {
    pub fn new(db_path: PathBuf, project_root: String) -> Self {
        Self {
            db_path,
            project_root,
        }
    }

    /// Open DB read-only. Called inside spawn_blocking only.
    fn open_db(path: &PathBuf) -> Result<Connection, OrganError> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| OrganError::ReadFailed(format!("sqlite open: {e}")))?;

        conn.busy_timeout(Duration::from_secs(1))
            .map_err(|e| OrganError::ReadFailed(format!("busy_timeout: {e}")))?;

        Ok(conn)
    }
}

#[async_trait]
impl OrganPort for RtkReader {
    fn name(&self) -> &str {
        "rtk"
    }

    async fn health(&self) -> OrganHealth {
        let path = self.db_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            if !path.exists() {
                return OrganHealth::Dead {
                    reason: "history.db not found".to_string(),
                };
            }
            match Self::open_db(&path) {
                Ok(_) => OrganHealth::Alive,
                Err(OrganError::ReadFailed(reason)) => OrganHealth::Degraded { reason },
                Err(OrganError::Unavailable(reason)) => OrganHealth::Dead { reason },
            }
        })
        .await;

        // K14: spawn failure = assume degraded (safe default, never optimistic)
        match result {
            Ok(h) => h,
            Err(_) => OrganHealth::Degraded {
                reason: "spawn_blocking panicked".to_string(),
            },
        }
    }

    async fn freshness(&self) -> Result<Duration, OrganError> {
        let path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = Self::open_db(&path)?;
            let ts: String = conn
                .query_row("SELECT MAX(timestamp) FROM commands", [], |row| row.get(0))
                .map_err(|e| OrganError::ReadFailed(format!("freshness query: {e}")))?;

            let parsed = chrono::DateTime::parse_from_rfc3339(&ts)
                .map_err(|e| OrganError::ReadFailed(format!("timestamp parse: {e}")))?;

            let age = Utc::now()
                .signed_duration_since(parsed)
                .to_std()
                // K14: future timestamp = zero age (safe default — not optimistic, just minimal)
                .unwrap_or(Duration::ZERO);

            Ok(age)
        })
        .await
        .map_err(|e| OrganError::ReadFailed(format!("spawn_blocking: {e}")))?
    }

    async fn snapshot(&self) -> Result<OrganSnapshot, OrganError> {
        let path = self.db_path.clone();
        let project_filter = format!("%{}%", self.project_root);

        tokio::task::spawn_blocking(move || {
            let conn = Self::open_db(&path)?;

            let (cmd_count, input_tok, output_tok, saved_tok, avg_pct, exec_ms): (
                i64,
                i64,
                i64,
                i64,
                f64,
                i64,
            ) = conn
                .query_row(
                    "SELECT COUNT(*), \
                            COALESCE(SUM(input_tokens), 0), \
                            COALESCE(SUM(output_tokens), 0), \
                            COALESCE(SUM(saved_tokens), 0), \
                            COALESCE(AVG(savings_pct), 0.0), \
                            COALESCE(SUM(exec_time_ms), 0) \
                     FROM commands WHERE project_path LIKE ?1",
                    [&project_filter],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .map_err(|e| OrganError::ReadFailed(format!("snapshot query: {e}")))?;

            let parse_failures: i64 = conn
                .query_row("SELECT COUNT(*) FROM parse_failures", [], |row| row.get(0))
                .map_err(|e| OrganError::ReadFailed(format!("parse_failures query: {e}")))?;

            Ok(OrganSnapshot {
                taken_at: Utc::now(),
                metrics: vec![
                    Metric {
                        key: "commands_total".into(),
                        value: MetricValue::I64(cmd_count),
                        kind: MetricKind::Counter,
                        unit: Some("count".into()),
                    },
                    Metric {
                        key: "tokens_input".into(),
                        value: MetricValue::I64(input_tok),
                        kind: MetricKind::Counter,
                        unit: Some("tokens".into()),
                    },
                    Metric {
                        key: "tokens_output".into(),
                        value: MetricValue::I64(output_tok),
                        kind: MetricKind::Counter,
                        unit: Some("tokens".into()),
                    },
                    Metric {
                        key: "tokens_saved".into(),
                        value: MetricValue::I64(saved_tok),
                        kind: MetricKind::Counter,
                        unit: Some("tokens".into()),
                    },
                    Metric {
                        key: "savings_pct".into(),
                        value: MetricValue::F64(avg_pct),
                        kind: MetricKind::Gauge,
                        unit: Some("%".into()),
                    },
                    Metric {
                        key: "exec_time_total".into(),
                        value: MetricValue::I64(exec_ms),
                        kind: MetricKind::Counter,
                        unit: Some("ms".into()),
                    },
                    Metric {
                        key: "parse_failures".into(),
                        value: MetricValue::I64(parse_failures),
                        kind: MetricKind::Counter,
                        unit: Some("count".into()),
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
    async fn health_dead_when_db_missing() {
        let reader = RtkReader::new(
            PathBuf::from("/tmp/nonexistent_rtk_test.db"),
            "/tmp".to_string(),
        );
        assert!(matches!(reader.health().await, OrganHealth::Dead { .. }));
    }

    #[tokio::test]
    async fn freshness_fails_when_db_missing() {
        let reader = RtkReader::new(
            PathBuf::from("/tmp/nonexistent_rtk_test.db"),
            "/tmp".to_string(),
        );
        assert!(reader.freshness().await.is_err());
    }

    #[tokio::test]
    async fn snapshot_fails_when_db_missing() {
        let reader = RtkReader::new(
            PathBuf::from("/tmp/nonexistent_rtk_test.db"),
            "/tmp".to_string(),
        );
        assert!(reader.snapshot().await.is_err());
    }
}
