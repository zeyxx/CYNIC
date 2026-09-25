//! Fleet probe orchestration for Dog backend observability.

mod config;
mod worker;

#[cfg(test)]
mod tests;

pub use config::FleetTarget;

use crate::domain::probe::{FleetDetails, ProbeDetails, ProbeError, ProbeResult, ProbeStatus};
use async_trait::async_trait;
pub use cynic_node::verify::{
    ContextResult, check_context_with_bearer, check_identity_with_bearer,
};
use std::time::{Duration, Instant};

#[allow(clippy::match_same_arms)]
pub(crate) fn context_n_ctx_from_result(result: &ContextResult) -> Option<u32> {
    match result {
        ContextResult::Observed { n_ctx } => Some(*n_ctx),
        ContextResult::Unknown | ContextResult::Unreachable => None,
    }
}

/// Probe that HTTP GETs the health endpoint of each configured Dog backend.
/// Reports all results via FleetDetails, not just the first failure.
#[derive(Debug)]
pub struct FleetProbe {
    targets: Vec<FleetTarget>,
    client: reqwest::Client,
}

impl FleetProbe {
    pub fn new(targets: Vec<FleetTarget>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { targets, client }
    }
}

#[async_trait]
impl crate::domain::probe::Probe for FleetProbe {
    fn name(&self) -> &str {
        "fleet"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(30)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();

        if self.targets.is_empty() {
            return Ok(ProbeResult {
                name: "fleet".to_string(),
                status: ProbeStatus::Ok,
                details: ProbeDetails::Fleet(FleetDetails { dogs: Vec::new() }),
                duration_ms: 0,
                timestamp: chrono::Utc::now().timestamp_millis(),
            });
        }

        let futures: Vec<_> = self
            .targets
            .iter()
            .map(|target| {
                let client = self.client.clone();
                let target = target.clone();
                async move { worker::probe_target(client, target).await }
            })
            .collect();

        let results = futures_util::future::join_all(futures).await;
        worker::log_health_findings(&results);

        Ok(ProbeResult {
            name: "fleet".to_string(),
            status: worker::derive_status(&results),
            details: ProbeDetails::Fleet(FleetDetails { dogs: results }),
            duration_ms: start.elapsed().as_millis() as u64,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }
}
