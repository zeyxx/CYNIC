//! State log background task — periodic organism state snapshots.
//!
//! Every 5 minutes, captures Dogs, system health, crystals, and resources
//! into a hash-chained StateBlock. Append-only, tamper-evident.

use std::sync::Arc;

use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};
use tokio_util::sync::CancellationToken;

use crate::api::rest::AppState;
use crate::domain::health_gate::count_healthy_dogs;
use crate::domain::state_log::{
    DogSnapshot, GENESIS_HASH, ResourceSnapshot, StateBlock, SystemSnapshot,
};
use crate::infra::task_health::TaskHealth;

/// Snapshot interval — 60 seconds.
/// Forensics needs sub-minute resolution; 60s = ~1440 blocks/day (~3 MB).
const SNAPSHOT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

pub fn spawn_state_log(
    rest_state: Arc<AppState>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = interval(SNAPSHOT_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        ticker.tick().await; // skip first tick (let system boot)

        // Seed chain from last persisted block
        let mut prev_hash = match rest_state.storage.last_state_block().await {
            Ok(Some(block)) => {
                klog!(
                    "[StateLog] Resuming chain at seq={} hash={}...",
                    block.seq,
                    &block.hash[..16]
                );
                block.hash
            }
            Ok(None) => {
                klog!("[StateLog] Starting new chain from genesis");
                GENESIS_HASH.to_string()
            }
            Err(e) => {
                tracing::warn!("StateLog: failed to load last block: {e} — starting from genesis");
                GENESIS_HASH.to_string()
            }
        };

        let mut seq = match rest_state.storage.last_state_block().await {
            Ok(Some(block)) => block.seq + 1,
            _ => 0,
        };

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] State log stopped");
                    break;
                }
                _ = ticker.tick() => {
                    let block = capture_snapshot(&rest_state, seq, &prev_hash).await;
                    match rest_state.storage.store_state_block(&block).await {
                        Ok(()) => {
                            tracing::info!(
                                seq = block.seq,
                                status = %block.system.status,
                                dogs = block.system.healthy_dogs,
                                hash = &block.hash[..16],
                                "state block recorded"
                            );
                            prev_hash = block.hash;
                            seq += 1;
                            task_health.touch_state_log();
                        }
                        Err(e) => {
                            tracing::warn!("StateLog: failed to store block seq={seq}: {e}");
                        }
                    }
                }
            }
        }
    })
}

async fn capture_snapshot(state: &AppState, seq: u64, prev_hash: &str) -> StateBlock {
    let judge = state.judge.load_full();

    // Dogs
    let dog_health = judge.dog_health_detailed();
    let dog_quality = judge.dog_quality_snapshot();
    let quality_map: std::collections::HashMap<String, _> = dog_quality.into_iter().collect();

    let dogs: Vec<DogSnapshot> = dog_health
        .into_iter()
        .map(|(id, circuit, _failures, reason, _open_secs)| {
            let (success_rate, mean_latency, total_failures) =
                if let Some(stats) = quality_map.get(&id) {
                    (
                        stats.json_valid_rate(),
                        stats.mean_latency_ms(),
                        stats.total_calls - stats.success_count,
                    )
                } else {
                    (0.0, 0.0, 0)
                };
            DogSnapshot {
                id,
                circuit,
                success_rate: (success_rate * 1000.0).round() / 1000.0,
                mean_latency_ms: (mean_latency * 10.0).round() / 10.0,
                failures: total_failures,
                last_failure_reason: reason.map(|r| r.as_str().to_string()),
            }
        })
        .collect();

    // System
    let basic_health = judge.dog_health();
    let (healthy_dogs, total_dogs) = count_healthy_dogs(&basic_health);
    let (verdict_count, total_tokens) = {
        let usage = state.usage.lock().await;
        (usage.all_time_requests(), usage.total_tokens())
    };
    let (crystals_forming, crystals_crystallized) = match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        state.storage.list_crystals(200),
    )
    .await
    {
        Ok(Ok(crystals)) => {
            let forming = crystals
                .iter()
                .filter(|c| c.state == crate::domain::ccm::CrystalState::Forming)
                .count();
            let crystallized = crystals.len() - forming;
            (forming, crystallized)
        }
        _ => (0, 0),
    };

    let system = SystemSnapshot {
        status: crate::domain::health_gate::system_health_status(
            healthy_dogs,
            total_dogs,
            true,
            false,
            false,
        )
        .0
        .to_string(),
        healthy_dogs,
        total_dogs,
        verdict_count,
        total_tokens,
        crystals_forming,
        crystals_crystallized,
    };

    // Resources (from environment probe if available)
    let resource = {
        let env_guard = state.environment.read().unwrap_or_else(|e| e.into_inner());
        let mut cpu = 0.0f64;
        let mut mem = 0.0f64;
        let mut disk = 0.0f64;
        let mut uptime = 0u64;
        if let Some(env) = env_guard.as_ref() {
            for probe in &env.probes {
                if let crate::domain::probe::ProbeDetails::Resource(r) = &probe.details {
                    cpu = r.cpu_usage_percent.unwrap_or(0.0) as f64;
                    mem = r.memory_used_gb.unwrap_or(0.0);
                    disk = r.disk_available_gb.unwrap_or(0.0);
                    uptime = r.uptime_seconds.unwrap_or(0);
                }
            }
        }
        ResourceSnapshot {
            cpu_pct: (cpu * 10.0).round() / 10.0,
            memory_used_gb: (mem * 100.0).round() / 100.0,
            disk_avail_gb: (disk * 100.0).round() / 100.0,
            uptime_secs: uptime,
        }
    };

    StateBlock::new(seq, prev_hash.to_string(), dogs, system, resource)
}
