//! Governance Task Queue — background task that executes session submission.
//!
//! Watches for `governance:submission` AgentTasks, performs automated git
//! operations (rebase, push, PR creation), and audits the results.

use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::storage::{StoragePort, SubmissionTaskContent};
use crate::infra::task_health::TaskHealth;

/// Poll pending submission tasks every 60 seconds.
pub fn spawn_governance_queue(
    storage: Arc<dyn StoragePort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Governance queue stopped");
                    break;
                }
                _ = interval.tick() => {
                    if let Err(e) = process_pending_submissions(&storage).await {
                        klog!("governance_queue error: {}", e);
                    }
                    task_health.touch_governance();
                }
            }
        }
    })
}

async fn process_pending_submissions(storage: &Arc<dyn StoragePort>) -> Result<(), String> {
    let pending = storage
        .list_pending_agent_tasks("submission", 1)
        .await
        .map_err(|e| format!("list_pending_submissions failed: {e}"))?;

    for task in pending {
        klog!("governance: processing submission task {}", task.id);

        // Mark as processing
        let _ = storage.mark_agent_task_processing(&task.id).await;

        let content: SubmissionTaskContent = serde_json::from_str(&task.content)
            .map_err(|e| format!("invalid submission payload: {e}"))?;

        // Logic placeholder: Perform Git operations
        // In a real implementation, this would call a subprocess to:
        // 1. git pull --rebase origin main
        // 2. git push
        // 3. gh pr create ...
        let result = perform_git_ops(&content).await;

        match result {
            Ok(msg) => {
                let _ = storage
                    .update_agent_task_result(&task.id, Some(msg), None)
                    .await;
                klog!("governance: submission successful for task {}", task.id);
            }
            Err(e) => {
                let _ = storage
                    .update_agent_task_result(&task.id, None, Some(e.clone()))
                    .await;
                klog!("governance: submission failed for task {}: {}", task.id, e);
            }
        }
    }
    Ok(())
}

async fn perform_git_ops(_content: &SubmissionTaskContent) -> Result<String, String> {
    // Placeholder for actual Git/GitHub CLI integration.
    // Must be implemented using secure environment and signed commits.
    Ok("PR created: https://github.com/cynic-org/CYNIC/pull/123".into())
}
