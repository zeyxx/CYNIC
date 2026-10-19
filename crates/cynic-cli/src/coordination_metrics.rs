//! Pure coordination-metrics domain logic (FCIS: no I/O).
//!
//! A coordination metrics projection aggregates three observables:
//!   (a) the number of currently-active claims,
//!   (b) agent response time derived from coordination-log timestamps,
//!   (c) a merge rate derived from git/PR counts.
//!
//! This module owns types, validation, and in-memory transformations only.
//! File I/O, git invocation, and VCS-port calls live in the adapter surface
//! (`coordination_surface`).

use serde::{Deserialize, Serialize};

use crate::coordination_log::CoordinationEntry;

pub const COORDINATION_METRICS_SCHEMA: &str = "cynic.coordination_metrics.v1";

/// Response-time statistics derived from coordination-log timestamps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseTimeStats {
    /// Number of timestamp deltas sampled.
    pub samples: usize,
    pub min_seconds: Option<f64>,
    pub max_seconds: Option<f64>,
    pub avg_seconds: Option<f64>,
}

impl ResponseTimeStats {
    pub fn empty() -> Self {
        Self {
            samples: 0,
            min_seconds: None,
            max_seconds: None,
            avg_seconds: None,
        }
    }
}

/// Merge-rate statistics derived from git/PR counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeRateStats {
    pub merged: usize,
    pub total: usize,
    pub rate: f64,
}

impl MergeRateStats {
    pub fn from_counts(merged: usize, total: usize) -> Self {
        let rate = if total == 0 {
            0.0
        } else {
            merged as f64 / total as f64
        };
        Self {
            merged,
            total,
            rate,
        }
    }
}

/// Unified coordination metrics projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoordinationMetrics {
    pub schema: String,
    pub active_claims: usize,
    pub agent_response_time: ResponseTimeStats,
    pub merge_rate: MergeRateStats,
}

impl CoordinationMetrics {
    pub fn new(
        active_claims: usize,
        agent_response_time: ResponseTimeStats,
        merge_rate: MergeRateStats,
    ) -> Self {
        Self {
            schema: COORDINATION_METRICS_SCHEMA.to_string(),
            active_claims,
            agent_response_time,
            merge_rate,
        }
    }
}

/// Pure count of active claims from a claims document.
///
/// A claim counts as active when its status is `"active"` and the current
/// time is strictly before `started_at_unix_s + ttl_seconds`. Mirrors the
/// liveness rule used by the claim protocol (expired-but-still-"active"
/// entries are ghosts, not members of the active set).
pub fn count_active_claims(claims: &[serde_json::Value], now_unix_s: u64) -> usize {
    claims
        .iter()
        .filter(|claim| {
            let status = claim.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if status != "active" {
                return false;
            }
            let started = claim
                .get("temporal")
                .and_then(|t| t.get("started_at_unix_s"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let ttl = claim
                .get("temporal")
                .and_then(|t| t.get("ttl_seconds"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            now_unix_s < started.saturating_add(ttl)
        })
        .count()
}

/// Extract the `claims` array from a claims document, defaulting to empty.
pub fn claims_from_document(doc: &serde_json::Value) -> Vec<serde_json::Value> {
    doc.get("claims")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Pure response-time computation from coordination-log entries.
///
/// Entries are sorted by timestamp; deltas between consecutive entries are
/// sampled. Returns empty stats when fewer than two entries are present.
pub fn response_time_from_entries(entries: &[CoordinationEntry]) -> ResponseTimeStats {
    if entries.len() < 2 {
        return ResponseTimeStats::empty();
    }
    let mut sorted: Vec<&CoordinationEntry> = entries.iter().collect();
    sorted.sort_by_key(|e| e.ts_unix_s);

    let mut deltas: Vec<f64> = Vec::with_capacity(sorted.len().saturating_sub(1));
    for pair in sorted.windows(2) {
        let delta = pair[1].ts_unix_s.saturating_sub(pair[0].ts_unix_s) as f64;
        deltas.push(delta);
    }
    if deltas.is_empty() {
        return ResponseTimeStats::empty();
    }
    let min = deltas.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = deltas.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let sum: f64 = deltas.iter().sum();
    let avg = sum / deltas.len() as f64;
    ResponseTimeStats {
        samples: deltas.len(),
        min_seconds: Some(min),
        max_seconds: Some(max),
        avg_seconds: Some(avg),
    }
}

/// Pure merge-rate computation from a merged/total pair.
pub fn merge_rate_from_counts(merged: usize, total: usize) -> MergeRateStats {
    MergeRateStats::from_counts(merged, total)
}

/// Assemble a full metrics projection from its three components.
pub fn build_metrics(
    active_claims: usize,
    agent_response_time: ResponseTimeStats,
    merge_rate: MergeRateStats,
) -> CoordinationMetrics {
    CoordinationMetrics::new(active_claims, agent_response_time, merge_rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(ts: u64, agent: &str) -> CoordinationEntry {
        CoordinationEntry::new(agent, "info", "body", vec![], ts)
    }

    #[test]
    fn count_active_claims_filters_ghosts() {
        let now = 1_700_000_100u64;
        let claims = vec![
            serde_json::json!({"status":"active","temporal":{"started_at_unix_s": now-50, "ttl_seconds": 1000}}),
            serde_json::json!({"status":"active","temporal":{"started_at_unix_s": now-2000, "ttl_seconds": 1000}}), // expired ghost
            serde_json::json!({"status":"released","temporal":{"started_at_unix_s": now-50, "ttl_seconds": 1000}}),
        ];
        assert_eq!(count_active_claims(&claims, now), 1);
    }

    #[test]
    fn response_time_empty_for_single_entry() {
        let entries = vec![entry(10, "a")];
        let stats = response_time_from_entries(&entries);
        assert_eq!(stats.samples, 0);
        assert!(stats.avg_seconds.is_none());
    }

    #[test]
    fn response_time_computes_deltas() {
        let entries = vec![entry(0, "a"), entry(5, "b"), entry(15, "a")];
        let stats = response_time_from_entries(&entries);
        assert_eq!(stats.samples, 2);
        assert_eq!(stats.min_seconds, Some(5.0));
        assert_eq!(stats.max_seconds, Some(10.0));
        assert_eq!(stats.avg_seconds, Some(7.5));
    }

    #[test]
    fn merge_rate_handles_zero_total() {
        let m = merge_rate_from_counts(0, 0);
        assert_eq!(m.rate, 0.0);
        let m2 = merge_rate_from_counts(3, 4);
        assert!((m2.rate - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn build_metrics_assembles() {
        let m = build_metrics(
            2,
            ResponseTimeStats::empty(),
            MergeRateStats::from_counts(1, 2),
        );
        assert_eq!(m.active_claims, 2);
        assert_eq!(m.schema, COORDINATION_METRICS_SCHEMA);
    }
}
