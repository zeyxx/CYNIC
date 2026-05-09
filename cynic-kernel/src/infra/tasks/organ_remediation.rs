//! Organ silence → auto-remediation (C4/E1).
//!
//! Checks `last_observation_per_source()` every 60s.
//! When an organ has been silent beyond its threshold, attempts restart via
//! `ssh_restart()`. Reuses `RecoveryTracker` for cooldown + retry bounding.
//!
//! Trigger: silence (no observations), NOT circuit-breaker open.
//! Independent from Dog remediation loop.

use std::sync::Arc;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::infra::config::{BackendRemediation, OrganRemediation};
use crate::infra::task_health::TaskHealth;

/// Organ remediation loop — restarts silent organs via ssh_restart().
/// Independent from Dog remediation (different trigger: silence, not circuit open).
pub fn spawn_organ_remediation(
    organ_configs: Vec<OrganRemediation>,
    storage: Arc<dyn crate::domain::storage::StoragePort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let organ_count = organ_configs.len();
    let handle = tokio::spawn(async move {
        let tracker = crate::infra::remediation::RecoveryTracker::new();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick — don't fire immediately at boot

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Organ remediation stopped");
                    break;
                }
                _ = interval.tick() => {
                    let sources = match tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        storage.last_observation_per_source(),
                    ).await {
                        Ok(Ok(s)) => s,
                        Ok(Err(e)) => {
                            tracing::debug!(error = %e, "organ remediation: storage query failed");
                            task_health.touch_remediation();
                            continue;
                        }
                        Err(_) => {
                            tracing::debug!("organ remediation: storage query timed out (10s)");
                            task_health.touch_remediation();
                            continue;
                        }
                    };
                    let now = chrono::Utc::now();

                    for organ_config in &organ_configs {
                        let silence_secs = sources
                            .iter()
                            .find(|(source, _, _)| source == &organ_config.source)
                            .and_then(|(_, last_at, _)| {
                                chrono::DateTime::parse_from_rfc3339(last_at).ok()
                            })
                            .map(|t| {
                                (now - t.with_timezone(&chrono::Utc))
                                    .num_seconds()
                                    .max(0) as u64
                            })
                            .unwrap_or(u64::MAX); // K14: unknown = assume degraded

                        if silence_secs > organ_config.silence_threshold_secs {
                            // Build a compat BackendRemediation to reuse RecoveryTracker
                            let compat = BackendRemediation {
                                node: organ_config.node.clone(),
                                restart_command: organ_config.restart_command.clone(),
                                max_retries: organ_config.max_retries,
                                cooldown_secs: organ_config.cooldown_secs,
                            };
                            if tracker.should_restart(&organ_config.source, &compat) {
                                let silence_display = if silence_secs == u64::MAX {
                                    "∞ (never seen)".to_string()
                                } else {
                                    format!("{silence_secs}s")
                                };
                                klog!(
                                    "[Remediation] Organ '{}' silent for {} (threshold {}s), attempting restart on {}",
                                    organ_config.source,
                                    silence_display,
                                    organ_config.silence_threshold_secs,
                                    organ_config.node,
                                );
                                let node = organ_config.node.clone();
                                let cmd = organ_config.restart_command.clone();
                                match tokio::time::timeout(
                                    std::time::Duration::from_secs(15),
                                    tokio::task::spawn_blocking(move || {
                                        crate::infra::remediation::ssh_restart(&node, &cmd)
                                    }),
                                )
                                .await
                                {
                                    Ok(Ok(Ok(output))) => {
                                        klog!(
                                            "[Remediation] Organ '{}' restart OK: {}",
                                            organ_config.source,
                                            output.trim()
                                        );
                                    }
                                    Ok(Ok(Err(e))) => {
                                        klog!(
                                            "[Remediation] Organ '{}' restart failed: {}",
                                            organ_config.source,
                                            e
                                        );
                                    }
                                    Ok(Err(e)) => {
                                        klog!(
                                            "[Remediation] Organ '{}' restart task panicked: {}",
                                            organ_config.source,
                                            e
                                        );
                                    }
                                    Err(_) => {
                                        klog!(
                                            "[Remediation] Organ '{}' restart timed out (15s)",
                                            organ_config.source
                                        );
                                    }
                                }
                                tracker.record_attempt(
                                    &organ_config.source,
                                    organ_config.max_retries,
                                );
                            }
                        }
                    }
                    task_health.touch_remediation();
                }
            }
        }
    });
    klog!(
        "[Ring 2] Organ remediation started ({} organs configured)",
        organ_count
    );
    handle
}
