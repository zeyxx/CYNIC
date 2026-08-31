use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio_util::sync::CancellationToken;

use crate::adapters::http_node_control::HttpNodeControlAdapter;
use crate::adapters::local_node_enrollment::LocalNodeEnrollmentReader;
use crate::config::NodeControlConfig;
use cynic_protocol::SecurityEvidenceState;

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
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = ticker.tick() => {
                let read_back = reader.read_back(&config.node_id);
                if read_back.state != SecurityEvidenceState::Proven {
                    tracing::warn!(node_id = %config.node_id, unknowns = ?read_back.unknowns, "enrollment renewal blocked by invalid current grant");
                    continue;
                }
                let now = SystemTime::now().duration_since(UNIX_EPOCH)
                    .map_or(0, |duration| duration.as_secs());
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
                    Ok(()) => tracing::info!(node_id = %config.node_id, generation = generation + 1, "machine enrollment renewed and read back"),
                    Err(error) => tracing::warn!(node_id = %config.node_id, %error, "machine enrollment renewal failed"),
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
}
