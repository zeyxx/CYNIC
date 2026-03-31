//! FleetProbe — HTTP health probes to Dog backend endpoints.
//! Reads health_url from backends.toml (via constructor) and probes each.
//! Reports ALL dog states (not just first failure). Checks HTTP status, not just connection.

use crate::domain::probe::{
    DogHealthDetails, FleetDetails, ProbeDetails, ProbeError, ProbeResult, ProbeStatus,
};
use async_trait::async_trait;
use std::time::{Duration, Instant};

#[cfg(test)]
use crate::domain::probe::Probe;

/// A Dog endpoint to health-check.
#[derive(Debug, Clone)]
pub struct FleetTarget {
    pub dog_name: String,
    pub health_url: String,
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
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }

        // Probe all targets in parallel.
        let futures: Vec<_> = self
            .targets
            .iter()
            .map(|t| {
                let client = self.client.clone();
                let url = t.health_url.clone();
                let name = t.dog_name.clone();
                async move {
                    let probe_start = Instant::now();
                    // Check HTTP status code, not just connection success.
                    let reachable = match client.get(&url).send().await {
                        Ok(resp) => resp.status().is_success(),
                        Err(_) => false,
                    };
                    let latency_ms = Some(probe_start.elapsed().as_millis() as u64);
                    DogHealthDetails {
                        dog_name: name,
                        reachable,
                        latency_ms,
                    }
                }
            })
            .collect();

        let results = futures_util::future::join_all(futures).await;

        let duration_ms = start.elapsed().as_millis() as u64;
        let timestamp = chrono::Utc::now().to_rfc3339();

        let reachable_count = results.iter().filter(|r| r.reachable).count();
        let total = results.len();

        let status = if reachable_count == total {
            ProbeStatus::Ok
        } else if reachable_count == 0 {
            ProbeStatus::Unavailable
        } else {
            ProbeStatus::Degraded
        };

        // Log unreachable dogs (state-transition logging is a follow-up).
        for r in &results {
            if !r.reachable {
                tracing::warn!(
                    dog = %r.dog_name,
                    latency_ms = ?r.latency_ms,
                    "fleet probe: Dog unreachable"
                );
            }
        }

        Ok(ProbeResult {
            name: "fleet".to_string(),
            status,
            details: ProbeDetails::Fleet(FleetDetails { dogs: results }),
            duration_ms,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn fleet_probe_no_targets_returns_ok() {
        let probe = FleetProbe::new(vec![]);
        let result = probe.sense().await.expect("should not error");
        assert_eq!(result.status, ProbeStatus::Ok);
    }

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn fleet_probe_unreachable_target() {
        let probe = FleetProbe::new(vec![FleetTarget {
            dog_name: "test-dog".into(),
            health_url: "http://127.0.0.1:19999/health".into(),
        }]);
        let result = probe.sense().await.expect("should not error");
        assert_eq!(result.status, ProbeStatus::Unavailable);
        match result.details {
            ProbeDetails::Fleet(ref f) => {
                assert_eq!(f.dogs.len(), 1);
                assert_eq!(f.dogs[0].dog_name, "test-dog");
                assert!(!f.dogs[0].reachable);
            }
            _ => panic!("expected Fleet details"),
        }
    }
}
