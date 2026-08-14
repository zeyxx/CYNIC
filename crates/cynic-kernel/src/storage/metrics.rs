//! Storage metrics primitives for Surreal-backed persistence.

use std::collections::HashMap;
use std::sync::{Arc, RwLock, atomic::AtomicU64, atomic::Ordering};

/// Slow query threshold in milliseconds.
pub(crate) const SLOW_QUERY_MS: u64 = 100;

#[derive(Debug, Clone)]
pub struct TableOpMetrics {
    pub table: String,
    pub operation: String,
    pub count: u64,
    pub errors: u64,
    pub total_latency_us: u64,
    pub slow_count: u64,
    pub min_latency_us: u64,
    pub max_latency_us: u64,
}

#[derive(Debug)]
pub struct StorageMetrics {
    pub queries: AtomicU64,
    pub errors: AtomicU64,
    pub total_latency_us: AtomicU64,
    pub slow_queries: AtomicU64,
    pub started_at: std::time::Instant,
    pub table_op_metrics: Arc<RwLock<HashMap<String, TableOpMetrics>>>,
}

impl StorageMetrics {
    pub(crate) fn new() -> Self {
        Self {
            queries: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            slow_queries: AtomicU64::new(0),
            started_at: std::time::Instant::now(),
            table_op_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub(crate) fn record_table_op(&self, table: &str, op: &str, latency_us: u64, is_error: bool) {
        let key = format!("{table}#{op}");

        if let Ok(mut metrics) = self.table_op_metrics.write() {
            let entry = metrics.entry(key).or_insert_with(|| TableOpMetrics {
                table: table.to_string(),
                operation: op.to_string(),
                count: 0,
                errors: 0,
                total_latency_us: 0,
                slow_count: 0,
                min_latency_us: u64::MAX,
                max_latency_us: 0,
            });

            entry.count += 1;
            if is_error {
                entry.errors += 1;
            }
            entry.total_latency_us += latency_us;
            if latency_us > (SLOW_QUERY_MS * 1000) {
                entry.slow_count += 1;
            }
            entry.min_latency_us = entry.min_latency_us.min(latency_us);
            entry.max_latency_us = entry.max_latency_us.max(latency_us);
        }
    }

    pub fn snapshot(&self) -> StorageMetricsSnapshot {
        let queries = self.queries.load(Ordering::Relaxed);
        let total_us = self.total_latency_us.load(Ordering::Relaxed);

        StorageMetricsSnapshot {
            queries,
            errors: self.errors.load(Ordering::Relaxed),
            slow_queries: self.slow_queries.load(Ordering::Relaxed),
            avg_latency_ms: total_us
                .checked_div(queries)
                .map_or(0.0, |avg| avg as f64 / 1000.0),
            uptime_secs: self.started_at.elapsed().as_secs(),
        }
    }

    pub fn snapshot_table_ops(&self) -> Vec<TableOpMetrics> {
        self.table_op_metrics
            .read()
            .ok()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn reset_table_ops(&self) {
        if let Ok(mut m) = self.table_op_metrics.write() {
            m.clear();
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct StorageMetricsSnapshot {
    pub queries: u64,
    pub errors: u64,
    pub slow_queries: u64,
    pub avg_latency_ms: f64,
    pub uptime_secs: u64,
}
