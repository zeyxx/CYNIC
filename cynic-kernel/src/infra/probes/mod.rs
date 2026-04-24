pub mod backup;
pub mod fleet;
pub mod network;
pub mod pressure;
pub mod process;
pub mod resource;
pub mod soma;

pub use backup::BackupProbe;
pub use fleet::{FleetProbe, FleetTarget};
pub use network::NetworkProbe;
pub use pressure::PressureProbe;
pub use process::ProcessProbe;
pub use resource::ResourceProbe;
pub use soma::SomaProbe;

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::domain::probe::{EnvironmentSnapshot, Probe, ProbeDetails, ProbeResult, ProbeStatus};

struct ProbeSlot {
    probe: Arc<dyn Probe>,
    last_fired: Instant,
}

impl std::fmt::Debug for ProbeSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProbeSlot")
            .field("probe", &self.probe.name())
            .field("last_fired", &self.last_fired)
            .finish()
    }
}

pub struct ProbeScheduler {
    probes: Vec<ProbeSlot>,
}

impl std::fmt::Debug for ProbeScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProbeScheduler")
            .field("probes", &self.probes)
            .finish()
    }
}

impl ProbeScheduler {
    pub fn new(probes: Vec<Arc<dyn Probe>>) -> Self {
        let now = Instant::now();
        Self {
            probes: probes
                .into_iter()
                .map(|p| ProbeSlot {
                    probe: p,
                    // Set last_fired to now so probes don't all fire on first tick.
                    // They'll fire after their interval elapses.
                    last_fired: now,
                })
                .collect(),
        }
    }

    /// Run one tick: fire all probes whose interval has elapsed.
    /// Fan-out in parallel. Per-probe timeout: min(2*interval, 30s).
    /// Returns None if no probes were due.
    pub async fn tick(&mut self) -> Option<EnvironmentSnapshot> {
        let now = Instant::now();

        // Collect indices of due probes.
        let due_indices: Vec<usize> = self
            .probes
            .iter()
            .enumerate()
            .filter(|(_, slot)| now.duration_since(slot.last_fired) >= slot.probe.interval())
            .map(|(i, _)| i)
            .collect();

        if due_indices.is_empty() {
            return None;
        }

        // Fire due probes in parallel with per-probe timeout.
        let futures: Vec<_> = due_indices
            .iter()
            .map(|&i| {
                let probe = Arc::clone(&self.probes[i].probe);
                let timeout_dur =
                    std::cmp::min(probe.interval().saturating_mul(2), Duration::from_secs(30));
                async move {
                    match tokio::time::timeout(timeout_dur, probe.sense()).await {
                        Ok(Ok(result)) => result,
                        Ok(Err(e)) => {
                            tracing::error!(
                                probe = probe.name(),
                                error = %e,
                                "probe internal error"
                            );
                            ProbeResult {
                                name: probe.name().to_string(),
                                status: ProbeStatus::Unavailable,
                                details: ProbeDetails::Empty,
                                duration_ms: 0,
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            }
                        }
                        Err(_) => {
                            tracing::warn!(probe = probe.name(), "probe timed out");
                            ProbeResult {
                                name: probe.name().to_string(),
                                status: ProbeStatus::Unavailable,
                                details: ProbeDetails::Empty,
                                duration_ms: timeout_dur.as_millis() as u64,
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            }
                        }
                    }
                }
            })
            .collect();

        let results = futures_util::future::join_all(futures).await;

        // Update last_fired for all due probes.
        let fired_at = Instant::now();
        for &i in &due_indices {
            self.probes[i].last_fired = fired_at;
        }

        let overall = EnvironmentSnapshot::worst_status(&results);
        let timestamp = chrono::Utc::now().to_rfc3339();

        Some(EnvironmentSnapshot {
            timestamp,
            probes: results,
            overall,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::probe::{NullProbe, ProbeError};
    use async_trait::async_trait;

    // ── TestProbe ─────────────────────────────────────────────────────────────

    /// Test double that always returns a fixed, pre-baked `ProbeResult`.
    /// Avoids the need to clone `ProbeError` (which doesn't impl Clone).
    struct TestProbe {
        name: &'static str,
        interval: Duration,
        /// `None` means the probe returns `Err(ProbeError::Internal(...))`.
        result: Option<ProbeResult>,
    }

    #[async_trait]
    impl Probe for TestProbe {
        fn name(&self) -> &str {
            self.name
        }

        fn interval(&self) -> Duration {
            self.interval
        }

        async fn sense(&self) -> Result<ProbeResult, ProbeError> {
            match &self.result {
                Some(r) => Ok(r.clone()),
                None => Err(ProbeError::Internal("test error".into())),
            }
        }
    }

    fn ok_result(name: &str) -> ProbeResult {
        ProbeResult {
            name: name.to_string(),
            status: ProbeStatus::Ok,
            details: ProbeDetails::Empty,
            duration_ms: 0,
            timestamp: "2026-01-01T00:00:00Z".into(),
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn scheduler_fires_due_probes() {
        // interval=0 means the probe is always due.
        let probe = TestProbe {
            name: "test",
            interval: Duration::from_secs(0),
            result: Some(ok_result("test")),
        };
        let mut scheduler = ProbeScheduler::new(vec![Arc::new(probe)]);
        let snap = scheduler.tick().await;
        assert!(snap.is_some());
        assert_eq!(snap.expect("snap must be Some").probes.len(), 1);
    }

    #[tokio::test]
    async fn scheduler_skips_not_due_probes() {
        // NullProbe interval=3600s — not due immediately after construction.
        let mut scheduler = ProbeScheduler::new(vec![Arc::new(NullProbe)]);
        let snap = scheduler.tick().await;
        assert!(snap.is_none());
    }

    #[tokio::test]
    async fn scheduler_handles_probe_error() {
        let faulty = TestProbe {
            name: "faulty",
            interval: Duration::from_secs(0),
            result: None, // will return Err
        };
        let good = TestProbe {
            name: "good",
            interval: Duration::from_secs(0),
            result: Some(ok_result("good")),
        };
        let mut scheduler = ProbeScheduler::new(vec![Arc::new(faulty), Arc::new(good)]);
        let snap = scheduler.tick().await;
        assert!(snap.is_some());
        let snap = snap.expect("snap must be Some");
        // Both probes produce results (faulty → Unavailable, good → Ok).
        assert_eq!(snap.probes.len(), 2);
        let faulty_result = snap
            .probes
            .iter()
            .find(|r| r.name == "faulty")
            .expect("faulty result");
        assert_eq!(faulty_result.status, ProbeStatus::Unavailable);
        let good_result = snap
            .probes
            .iter()
            .find(|r| r.name == "good")
            .expect("good result");
        assert_eq!(good_result.status, ProbeStatus::Ok);
    }
}
