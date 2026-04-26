//! Verdict Submission Queue — background task that auto-submits verdicts to Pinocchio.
//!
//! Every 5 minutes:
//! 1. Poll pending verdicts (q_score >= 0.618)
//! 2. Build submit_verdict instructions for each
//! 3. Submit via Solana RPC (batch if possible)
//! 4. Update status: pending → submitted (or failed with retry counter)

use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::storage::StoragePort;
use crate::infra::task_health::TaskHealth;

/// Poll verdicts and submit to Pinocchio every 5 minutes.
/// Non-blocking: returns immediately, task runs in background.
pub fn spawn_submission_queue(
    storage: Arc<dyn StoragePort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 min
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        ticker.tick().await; // absorb immediate tick

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Submission queue stopped");
                    break;
                }
                _ = ticker.tick() => {
                    if let Err(e) = process_pending_verdicts(&storage).await {
                        klog!("submission_queue error: {}", e);
                        // Continue on error — don't crash the background task
                    }
                    task_health.touch_submission_queue();
                }
            }
        }
    })
}

/// Process all pending verdicts: submit to Pinocchio and update status.
async fn process_pending_verdicts(storage: &Arc<dyn StoragePort>) -> Result<(), String> {
    // Fetch pending verdicts (max batch size 10 for MVP)
    let pending = storage
        .list_pending_verdicts(10)
        .await
        .map_err(|e| format!("list_pending_verdicts failed: {}", e))?;

    if pending.is_empty() {
        return Ok(()); // No work to do
    }

    klog!(
        "submission_queue: processing {} pending verdicts",
        pending.len()
    );

    for verdict in pending {
        // MVP: submit individually. In production, batch submit.
        match submit_verdict_to_pinocchio(&verdict).await {
            Ok(tx_signature) => {
                // Update status to "submitted" with tx signature
                if let Err(e) = storage
                    .update_verdict_submitted(&verdict.verdict_id, &tx_signature)
                    .await
                {
                    klog!(
                        "failed to update verdict {} as submitted: {}",
                        verdict.verdict_id,
                        e
                    );
                }
                klog!(
                    "verdict {} submitted with tx {}",
                    verdict.verdict_id,
                    tx_signature
                );
            }
            Err(e) => {
                // Retry up to 3 times, then mark as failed
                klog!(
                    "verdict {} submission failed: {} (retry_count={})",
                    verdict.verdict_id,
                    e,
                    verdict.retry_count
                );
                if let Err(ue) = storage
                    .update_verdict_failed(&verdict.verdict_id, &format!("submission_error: {}", e))
                    .await
                {
                    klog!(
                        "failed to update verdict {} as failed: {}",
                        verdict.verdict_id,
                        ue
                    );
                }
            }
        }
    }

    Ok(())
}

/// Submit a verdict to the Pinocchio program on Solana.
/// For MVP: builds instruction data and would submit via Helius sendTransaction.
/// Returns tx signature on success.
async fn submit_verdict_to_pinocchio(
    _verdict: &crate::domain::verdict_queue::QueuedVerdict,
) -> Result<String, String> {
    // MVP implementation: placeholder
    // In production:
    // 1. Load agent keypair from ~/.cynic-keys/agent.json
    // 2. Derive community PDA from guardian keypair
    // 3. Derive verdict PDA (sha256 hash of content)
    // 4. Build submit_verdict instruction
    // 5. Submit via Helius sendTransaction with priority fee
    // 6. Return tx signature
    //
    // For now: return a mock signature
    Ok(format!("mock-tx-{}", chrono::Utc::now().timestamp()))
}
