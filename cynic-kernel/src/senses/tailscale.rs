//! TailscaleReader — fleet nervous system via `tailscale status --json`.
//! Async subprocess call (not spawn_blocking — tokio::process is already async).
//! Filters funnel-ingress-node peers (Tailscale infra, not our fleet).

use crate::domain::organ::{
    Metric, MetricKind, MetricValue, OrganError, OrganHealth, OrganPort, OrganSnapshot,
};
use async_trait::async_trait;
use chrono::Utc;
use std::time::Duration;

#[derive(Debug)]
pub struct TailscaleReader {
    /// Peer hostnames to track (our fleet, not all Tailscale peers)
    fleet_nodes: Vec<String>,
}

impl TailscaleReader {
    pub fn new(fleet_nodes: Vec<String>) -> Self {
        Self { fleet_nodes }
    }

    /// Parse `tailscale status --json` output into metrics.
    /// Extracted for testability — unit tests pass mock JSON.
    fn parse_status(json: &str, fleet_nodes: &[String]) -> Result<Vec<Metric>, OrganError> {
        let v: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| OrganError::ReadFailed(format!("JSON parse: {e}")))?;

        let peers = v
            .get("Peer")
            .and_then(|p| p.as_object())
            .ok_or_else(|| OrganError::ReadFailed("missing Peer object".into()))?;

        let mut online_count = 0i64;
        let mut ssh_ready_count = 0i64;
        let mut fleet_total = 0i64;
        let mut metrics = Vec::new();

        for peer in peers.values() {
            let hostname = peer.get("HostName").and_then(|h| h.as_str()).unwrap_or("");

            // Skip funnel-ingress-node (Tailscale infra, 22 of 29 peers observed)
            if hostname == "funnel-ingress-node" {
                continue;
            }

            // Only track fleet nodes if specified
            if !fleet_nodes.is_empty() && !fleet_nodes.iter().any(|n| n == hostname) {
                continue;
            }

            fleet_total += 1;
            let is_online = peer
                .get("Online")
                .and_then(|o| o.as_bool())
                .unwrap_or(false);
            if is_online {
                online_count += 1;
            }

            // SSH ready = online + SSH field true
            let has_ssh = peer.get("SSH").and_then(|s| s.as_bool()).unwrap_or(false);
            if is_online && has_ssh {
                ssh_ready_count += 1;
            }

            // Per-node online status for fleet awareness
            metrics.push(Metric {
                key: format!("node_{hostname}_online"),
                value: MetricValue::Bool(is_online),
                kind: MetricKind::Gauge,
                unit: None,
            });
        }

        // Aggregate metrics first
        metrics.insert(
            0,
            Metric {
                key: "nodes_online".into(),
                value: MetricValue::I64(online_count),
                kind: MetricKind::Gauge,
                unit: Some("count".into()),
            },
        );
        metrics.insert(
            1,
            Metric {
                key: "nodes_total".into(),
                value: MetricValue::I64(fleet_total),
                kind: MetricKind::Gauge,
                unit: Some("count".into()),
            },
        );
        metrics.insert(
            2,
            Metric {
                key: "nodes_ssh_ready".into(),
                value: MetricValue::I64(ssh_ready_count),
                kind: MetricKind::Gauge,
                unit: Some("count".into()),
            },
        );

        Ok(metrics)
    }

    /// Run `tailscale status --json` with a 5s timeout.
    async fn run_tailscale_cmd() -> Result<String, OrganError> {
        let output = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::process::Command::new("tailscale")
                .args(["status", "--json"])
                .output(),
        )
        .await
        .map_err(|e| OrganError::ReadFailed(format!("tailscale status timed out (5s): {e}")))?
        .map_err(|e| OrganError::ReadFailed(format!("tailscale exec: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OrganError::ReadFailed(format!(
                "tailscale status exit {}: {stderr}",
                output.status
            )));
        }

        String::from_utf8(output.stdout).map_err(|e| OrganError::ReadFailed(format!("UTF-8: {e}")))
    }
}

#[async_trait]
impl OrganPort for TailscaleReader {
    fn name(&self) -> &str {
        "tailscale"
    }

    async fn health(&self) -> OrganHealth {
        match Self::run_tailscale_cmd().await {
            Ok(json) => match Self::parse_status(&json, &self.fleet_nodes) {
                Ok(_) => OrganHealth::Alive,
                Err(OrganError::ReadFailed(reason)) => OrganHealth::Degraded { reason },
                Err(OrganError::Unavailable(reason)) => OrganHealth::Dead { reason },
            },
            Err(OrganError::ReadFailed(reason)) => OrganHealth::Degraded { reason },
            Err(OrganError::Unavailable(reason)) => OrganHealth::Dead { reason },
        }
    }

    async fn freshness(&self) -> Result<Duration, OrganError> {
        // Tailscale status is always live — freshness = 0.
        // Unlike RTK (reads historical DB) or Hermes (reads cached files),
        // tailscale status probes the daemon's current state.
        Ok(Duration::ZERO)
    }

    async fn snapshot(&self) -> Result<OrganSnapshot, OrganError> {
        let json = Self::run_tailscale_cmd().await?;
        let metrics = Self::parse_status(&json, &self.fleet_nodes)?;
        Ok(OrganSnapshot {
            taken_at: Utc::now(),
            metrics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOCK_STATUS: &str = r#"{
        "Self": {"HostName": "cynic-core"},
        "Peer": {
            "abc": {"HostName": "cynic-gpu", "Online": true, "SSH": true, "OS": "windows"},
            "def": {"HostName": "kairos", "Online": false, "SSH": true, "OS": "linux"},
            "ghi": {"HostName": "funnel-ingress-node", "Online": true}
        }
    }"#;

    #[test]
    fn parse_filters_funnel_ingress() {
        let metrics =
            TailscaleReader::parse_status(MOCK_STATUS, &["cynic-gpu".into(), "kairos".into()])
                .unwrap();
        assert!(metrics.iter().all(|m| !m.key.contains("funnel")));
    }

    #[test]
    fn parse_counts_online_nodes() {
        let metrics =
            TailscaleReader::parse_status(MOCK_STATUS, &["cynic-gpu".into(), "kairos".into()])
                .unwrap();
        let online = metrics.iter().find(|m| m.key == "nodes_online").unwrap();
        match &online.value {
            MetricValue::I64(v) => assert_eq!(*v, 1), // only cynic-gpu is online
            other => panic!("expected I64, got {other:?}"),
        }
    }

    #[test]
    fn parse_per_node_status() {
        let metrics =
            TailscaleReader::parse_status(MOCK_STATUS, &["cynic-gpu".into(), "kairos".into()])
                .unwrap();
        let gpu = metrics
            .iter()
            .find(|m| m.key == "node_cynic-gpu_online")
            .unwrap();
        match &gpu.value {
            MetricValue::Bool(v) => assert!(*v),
            other => panic!("expected Bool, got {other:?}"),
        }
        let kairos = metrics
            .iter()
            .find(|m| m.key == "node_kairos_online")
            .unwrap();
        match &kairos.value {
            MetricValue::Bool(v) => assert!(!*v),
            other => panic!("expected Bool, got {other:?}"),
        }
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let result = TailscaleReader::parse_status("not json", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_fleet_tracks_all_non_funnel() {
        let metrics = TailscaleReader::parse_status(MOCK_STATUS, &[]).unwrap();
        let total = metrics.iter().find(|m| m.key == "nodes_total").unwrap();
        match &total.value {
            MetricValue::I64(v) => assert_eq!(*v, 2), // cynic-gpu + kairos (no funnel)
            other => panic!("expected I64, got {other:?}"),
        }
    }
}
