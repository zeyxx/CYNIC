//! VersionProbe — supply chain self-awareness.
//! Tracks running versions and checks latest upstream tags for selected dependencies.

mod collect;
mod config;

#[cfg(test)]
mod tests;

use async_trait::async_trait;
use std::time::{Duration, Instant};

use crate::domain::probe::{
    DependencyVersion, ProbeDetails, ProbeError, ProbeResult, ProbeStatus, VersionDetails,
};
use collect::collect_dependencies;
use config::TRACKED_DEPENDENCIES;

#[derive(Debug)]
pub struct VersionProbe {
    client: reqwest::Client,
}

impl Default for VersionProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionProbe {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .user_agent("cynic-core/version-probe")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }
}

#[async_trait]
impl crate::domain::probe::Probe for VersionProbe {
    fn name(&self) -> &str {
        "version"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(3600)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();
        let mut dependencies = Vec::new();

        // Kernel version (compile-time).
        dependencies.push(DependencyVersion {
            name: "cynic-core".to_string(),
            running: Some(env!("CARGO_PKG_VERSION").to_string()),
            latest: None,
            behind: false,
            versions_behind: 0,
            source: "zeyxx/CYNIC".to_string(),
        });

        let (tracked, behind_count, unknown_count) =
            collect_dependencies(&self.client, TRACKED_DEPENDENCIES).await;
        dependencies.extend(tracked);

        let status = if unknown_count > 0 {
            ProbeStatus::Degraded
        } else {
            ProbeStatus::Ok
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        Ok(ProbeResult {
            name: self.name().to_string(),
            status,
            details: ProbeDetails::Version(VersionDetails {
                dependencies,
                behind_count,
                unknown_count,
            }),
            duration_ms,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }
}
