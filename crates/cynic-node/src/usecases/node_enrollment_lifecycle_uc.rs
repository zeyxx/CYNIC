use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::adapters::http_node_control::HttpNodeControlAdapter;
use crate::adapters::local_node_enrollment::LocalNodeEnrollmentReader;
use crate::config::NodeControlConfig;
use cynic_protocol::SecurityEvidenceState;

/// Persistent telemetry status for enrollment renewal attempts.
/// Closes measurement gap #2: surfaces countable attempts, successes, failures and last error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentRenewalStatus {
    pub schema: String,
    pub node_id: String,
    pub attempts_count: u64,
    pub successes_count: u64,
    pub failures_count: u64,
    pub last_attempt_unix_s: Option<u64>,
    pub last_success_unix_s: Option<u64>,
    pub last_error: Option<String>,
}

impl NodeEnrollmentRenewalStatus {
    pub fn new(node_id: &str) -> Self {
        Self {
            schema: "cynic.node_enrollment.renewal_status.v1".to_string(),
            node_id: node_id.to_string(),
            attempts_count: 0,
            successes_count: 0,
            failures_count: 0,
            last_attempt_unix_s: None,
            last_success_unix_s: None,
            last_error: None,
        }
    }

    pub fn record_success(&mut self, now_unix_s: u64) {
        self.attempts_count += 1;
        self.successes_count += 1;
        self.last_attempt_unix_s = Some(now_unix_s);
        self.last_success_unix_s = Some(now_unix_s);
        self.last_error = None;
    }

    pub fn record_failure(&mut self, now_unix_s: u64, error: String) {
        self.attempts_count += 1;
        self.failures_count += 1;
        self.last_attempt_unix_s = Some(now_unix_s);
        self.last_error = Some(error);
    }
}

pub fn renewal_status_path(grant_path: &Path) -> PathBuf {
    grant_path.with_file_name("renewal-status.json")
}

pub fn save_renewal_status(path: &Path, status: &NodeEnrollmentRenewalStatus) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let bytes = serde_json::to_vec_pretty(status)
        .map_err(|e| format!("serialize renewal status failed: {e}"))?;
    std::fs::write(path, bytes)
        .map_err(|e| format!("write renewal status failed: {e}"))
}

pub fn load_renewal_status(path: &Path) -> Option<NodeEnrollmentRenewalStatus> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Pure L2 renewal decision (step 4): renew exactly when the grant's
/// remaining life has entered the configured renewal window.
///
/// Law: `remaining <= window` → due; saturating arithmetic keeps a clock jump
/// past expiry (`now > expires`) due as well — an expired grant always asks
/// for renewal instead of underflowing into false freshness.
pub fn enrollment_renewal_due(expires_at_unix_s: u64, now_unix_s: u64, window_secs: u64) -> bool {
    expires_at_unix_s.saturating_sub(now_unix_s) <= window_secs
}

pub async fn run_enrollment_renewal_loop(
    adapter: HttpNodeControlAdapter,
    reader: LocalNodeEnrollmentReader,
    config: NodeControlConfig,
    shutdown: CancellationToken,
) {
    let check_interval = Duration::from_secs(config.enrollment_renewal_window_secs.clamp(10, 300));
    let mut ticker = tokio::time::interval(check_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let status_path = renewal_status_path(&config.enrollment_grant_path);
    let mut status = load_renewal_status(&status_path)
        .unwrap_or_else(|| NodeEnrollmentRenewalStatus::new(&config.node_id));

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = ticker.tick() => {
                let read_back = reader.read_back(&config.node_id);
                let now = SystemTime::now().duration_since(UNIX_EPOCH)
                    .map_or(0, |duration| duration.as_secs());

                if read_back.state != SecurityEvidenceState::Proven {
                    tracing::warn!(node_id = %config.node_id, unknowns = ?read_back.unknowns, "enrollment renewal blocked by invalid current grant");
                    status.record_failure(now, format!("blocked_invalid_grant:{:?}", read_back.unknowns));
                    let _ = save_renewal_status(&status_path, &status);
                    continue;
                }
                let Some(expires_at) = read_back.expires_at_unix_s else { continue; };
                if !enrollment_renewal_due(
                    expires_at,
                    now,
                    config.enrollment_renewal_window_secs,
                ) {
                    continue;
                }
                let (Some(grant_id), Some(generation)) = (read_back.grant_id, read_back.generation) else {
                    continue;
                };
                match adapter.renew_enrollment(crate::adapters::http_node_control::RenewEnrollmentParams {
                    node_id: &config.node_id,
                    role: &config.enrollment_role,
                    seed_id: &config.enrollment_seed_id,
                    requested_capabilities: read_back.granted_capabilities,
                    request_ttl_secs: config.enrollment_request_ttl_secs,
                    current_grant_id: &grant_id,
                    expected_generation: generation,
                    bundle_path: &config.enrollment_grant_path,
                }).await {
                    Ok(()) => {
                        tracing::info!(node_id = %config.node_id, generation = generation + 1, "machine enrollment renewed and read back");
                        status.record_success(now);
                        let _ = save_renewal_status(&status_path, &status);
                    }
                    Err(error) => {
                        tracing::warn!(node_id = %config.node_id, %error, "machine enrollment renewal failed");
                        status.record_failure(now, error);
                        let _ = save_renewal_status(&status_path, &status);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renewal_not_due_while_remaining_life_exceeds_window() {
        assert!(!enrollment_renewal_due(10_000, 5_000, 1_000));
    }

    #[test]
    fn renewal_due_at_window_boundary_and_inside() {
        assert!(enrollment_renewal_due(6_000, 5_000, 1_000));
        assert!(enrollment_renewal_due(5_500, 5_000, 1_000));
    }

    #[test]
    fn expired_grant_is_always_due_no_underflow_false_freshness() {
        assert!(enrollment_renewal_due(4_000, 9_999, 1_000));
        assert!(enrollment_renewal_due(0, u64::MAX, 300));
    }

    #[test]
    fn zero_window_renews_only_when_already_expired_or_exact() {
        assert!(!enrollment_renewal_due(6_000, 5_000, 0));
        assert!(enrollment_renewal_due(5_000, 5_000, 0));
    }

    #[test]
    fn test_renewal_status_record_and_serialization() {
        let mut status = NodeEnrollmentRenewalStatus::new("compute-01");
        assert_eq!(status.attempts_count, 0);

        status.record_failure(100, "connection_refused".to_string());
        assert_eq!(status.attempts_count, 1);
        assert_eq!(status.failures_count, 1);
        assert_eq!(status.last_error.as_deref(), Some("connection_refused"));

        status.record_success(200);
        assert_eq!(status.attempts_count, 2);
        assert_eq!(status.successes_count, 1);
        assert_eq!(status.last_error, None);
        assert_eq!(status.last_success_unix_s, Some(200));

        let serialized = serde_json::to_string(&status).unwrap();
        let deserialized: NodeEnrollmentRenewalStatus = serde_json::from_str(&serialized).unwrap();
        assert_eq!(status, deserialized);
    }
}
