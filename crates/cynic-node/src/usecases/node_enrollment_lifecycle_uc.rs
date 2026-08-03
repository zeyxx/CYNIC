use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio_util::sync::CancellationToken;

use crate::adapters::http_node_control::HttpNodeControlAdapter;
use crate::adapters::local_node_enrollment::LocalNodeEnrollmentReader;
use crate::config::NodeControlConfig;
use cynic_protocol::SecurityEvidenceState;

pub(crate) async fn run_enrollment_renewal_loop(
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
                if expires_at.saturating_sub(now) > config.enrollment_renewal_window_secs {
                    continue;
                }
                let (Some(grant_id), Some(generation)) = (read_back.grant_id, read_back.generation) else {
                    continue;
                };
                match adapter.renew_enrollment(
                    &config.node_id,
                    &config.enrollment_role,
                    &config.enrollment_seed_id,
                    read_back.granted_capabilities,
                    config.enrollment_request_ttl_secs,
                    &grant_id,
                    generation,
                    &config.enrollment_grant_path,
                ).await {
                    Ok(()) => tracing::info!(node_id = %config.node_id, generation = generation + 1, "machine enrollment renewed and read back"),
                    Err(error) => tracing::warn!(node_id = %config.node_id, %error, "machine enrollment renewal failed"),
                }
            }
        }
    }
}
