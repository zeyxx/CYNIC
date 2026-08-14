use std::sync::Arc;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::domain::events::KernelEvent;
use crate::domain::storage::StoragePort;
use crate::infra::task_health::TaskHealth;

pub const NIGHTSHIFT_REPORT_SCHEMA_V1: &str = "cynic.nightshift_report.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NightshiftReport {
    pub schema: &'static str,
    pub round_number: u64,
    pub timestamp: String,
    pub expired_leases_swept: usize,
    pub state_entries_verified: usize,
    pub poh_checkpoint_digest: String,
}

pub fn execute_nightshift_round(
    round_number: u64,
    expired_leases: usize,
    state_entries: usize,
) -> NightshiftReport {
    let timestamp = Utc::now().to_rfc3339();

    let mut hasher = Sha256::new();
    hasher.update(round_number.to_be_bytes());
    hasher.update(timestamp.as_bytes());
    hasher.update(expired_leases.to_be_bytes());
    hasher.update(state_entries.to_be_bytes());
    let poh_checkpoint_digest = hex::encode(hasher.finalize());

    NightshiftReport {
        schema: NIGHTSHIFT_REPORT_SCHEMA_V1,
        round_number,
        timestamp,
        expired_leases_swept: expired_leases,
        state_entries_verified: state_entries,
        poh_checkpoint_digest,
    }
}

pub(crate) fn spawn_nightshift(
    _storage: Arc<dyn StoragePort>,
    event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut round = 0u64;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    break;
                }
                _ = interval.tick() => {
                    round += 1;
                    task_health.mark_alive();

                    let report = execute_nightshift_round(round, 0, 0);
                    let _ = event_tx.send(KernelEvent::StateTransition {
                        state: format!("nightshift_round_{round}_{}", &report.poh_checkpoint_digest[..8]),
                    });
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nightshift_round_execution_and_digest() {
        let r1 = execute_nightshift_round(1, 5, 120);
        assert_eq!(r1.round_number, 1);
        assert_eq!(r1.expired_leases_swept, 5);
        assert_eq!(r1.state_entries_verified, 120);
        assert_eq!(r1.poh_checkpoint_digest.len(), 64);
        assert_eq!(r1.schema, NIGHTSHIFT_REPORT_SCHEMA_V1);
    }
}
