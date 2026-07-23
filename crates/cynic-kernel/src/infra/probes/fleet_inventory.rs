//! Fleet inventory probe adapter.
//!
//! Ownership of provider-facing fleet inventory collection lives in `cynic-node`.
//! The kernel keeps only a thin probe adapter so the existing probe scheduler and
//! lifecycle diagnostics can consume the same shaped evidence.

use crate::domain::probe::{
    FleetInventoryDetails, ProbeDetails, ProbeError, ProbeResult, ProbeStatus,
};
use async_trait::async_trait;
use cynic_host::runtime_registry::DEFAULT_REGISTRY_PATH;
use cynic_host::runtime_registry::resolve_runtime_registry_path;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct FleetInventoryProbe {
    registry_path: PathBuf,
}

pub fn build_default_fleet_inventory_probe() -> FleetInventoryProbe {
    let fallback = PathBuf::from(DEFAULT_REGISTRY_PATH);
    let registry_path = resolve_runtime_registry_path(None).unwrap_or(fallback);
    FleetInventoryProbe { registry_path }
}

#[async_trait]
impl crate::domain::probe::Probe for FleetInventoryProbe {
    fn name(&self) -> &str {
        "fleet_inventory"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();
        let details =
            cynic_node::fleet_inventory::build_fleet_inventory_details(&self.registry_path)
                .await
                .map_err(ProbeError::Internal)
                .and_then(convert_details)?;

        let total = details.nodes.len();
        let provider_errors = details
            .summary
            .providers
            .iter()
            .filter(|provider| provider.status == "degraded")
            .count();
        let missing = details.summary.missing_nodes;

        let status = if details.summary.providers.is_empty() {
            if details.summary.declared_nodes == 0 {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Degraded
            }
        } else if provider_errors > 0 || missing > 0 {
            ProbeStatus::Degraded
        } else if total == 0 {
            ProbeStatus::Unavailable
        } else {
            ProbeStatus::Ok
        };

        Ok(ProbeResult {
            name: "fleet_inventory".to_string(),
            status,
            details: ProbeDetails::FleetInventory(details),
            duration_ms: start.elapsed().as_millis() as u64,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }
}

fn convert_details(
    details: cynic_node::fleet_inventory::FleetInventoryDetails,
) -> Result<FleetInventoryDetails, ProbeError> {
    let value = serde_json::to_value(details).map_err(|error| {
        ProbeError::Internal(format!("fleet inventory serialization failed: {error}"))
    })?;
    serde_json::from_value(value)
        .map_err(|error| ProbeError::Internal(format!("fleet inventory shape mismatch: {error}")))
}
