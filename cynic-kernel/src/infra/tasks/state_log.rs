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
use crate::domain::organ::{Metric, MetricKind, MetricValue, OrganHealth};
use crate::domain::state_log::{
    DogSnapshot, GENESIS_HASH, OrganAuditSnapshot, OrganSnapshot, ResourceSnapshot, StateBlock,
    SystemSnapshot,
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

    let organ_audits = capture_organ_audits(state).await;

    // Organs — last observation per source (agent_id)
    let organs = match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        state.storage.last_observation_per_source(),
    )
    .await
    {
        Ok(Ok(sources)) => {
            tracing::info!(
                source_count = sources.len(),
                "state_log: organ sources loaded"
            );
            let now_ts = chrono::Utc::now();
            sources
                .into_iter()
                .map(|(source, last_at, total)| {
                    let silence = chrono::DateTime::parse_from_rfc3339(&last_at)
                        .map(|t| {
                            (now_ts - t.with_timezone(&chrono::Utc))
                                .num_seconds()
                                .max(0) as u64
                        })
                        .unwrap_or(u64::MAX);
                    OrganSnapshot {
                        source,
                        last_observation: last_at,
                        total_observations: total,
                        silence_secs: silence,
                    }
                })
                .collect()
        }
        Ok(Err(e)) => {
            tracing::warn!("state_log: organ query failed: {e}");
            vec![]
        }
        Err(_) => {
            tracing::warn!("state_log: organ query timed out (3s)");
            vec![]
        }
    };

    StateBlock::new(
        seq,
        prev_hash.to_string(),
        dogs,
        system,
        resource,
        organs,
        organ_audits,
    )
}

async fn capture_organ_audits(state: &AppState) -> Vec<OrganAuditSnapshot> {
    let mut audits = Vec::new();

    for organ in &state.senses {
        let name = organ.name().to_string();
        let health = organ.health().await;
        let (health_label, health_reason) = match health {
            OrganHealth::Alive => ("alive".to_string(), None),
            OrganHealth::Degraded { reason } => ("degraded".to_string(), Some(reason)),
            OrganHealth::Dead { reason } => ("dead".to_string(), Some(reason)),
        };

        let mut anomalies = Vec::new();
        if health_label != "alive" {
            anomalies.push(health_label.clone());
        }

        let freshness_secs = match tokio::time::timeout(
            std::time::Duration::from_secs(3),
            organ.freshness(),
        )
        .await
        {
            Ok(Ok(age)) => {
                let secs = age.as_secs();
                if secs > 8 * 3600 {
                    anomalies.push("stale".to_string());
                }
                Some(secs)
            }
            Ok(Err(e)) => {
                anomalies.push(format!("freshness_error:{e}"));
                None
            }
            Err(_) => {
                anomalies.push("freshness_timeout".to_string());
                None
            }
        };

        let (metrics, snapshot_error) =
            match tokio::time::timeout(std::time::Duration::from_secs(5), organ.snapshot()).await {
                Ok(Ok(snapshot)) => (snapshot.metrics, None),
                Ok(Err(e)) => (Vec::new(), Some(format!("snapshot_error:{e}"))),
                Err(_) => (Vec::new(), Some("snapshot_timeout".to_string())),
            };

        if let Some(error) = snapshot_error {
            anomalies.push(error);
        }
        if metrics.is_empty() {
            anomalies.push("no_metrics".to_string());
        }

        let counter_count = metrics
            .iter()
            .filter(|m| matches!(m.kind, MetricKind::Counter))
            .count();
        let gauge_count = metrics
            .iter()
            .filter(|m| matches!(m.kind, MetricKind::Gauge))
            .count();
        let metrics_hash = hash_metrics(&metrics);

        audits.push(OrganAuditSnapshot {
            organ: name,
            health: health_label,
            health_reason,
            freshness_secs,
            metric_count: metrics.len(),
            counter_count,
            gauge_count,
            metrics_hash,
            anomalies,
        });
    }

    audits
}

fn hash_metrics(metrics: &[Metric]) -> String {
    let mut canonical: Vec<serde_json::Value> = metrics
        .iter()
        .map(|m| {
            serde_json::json!({
                "key": &m.key,
                "value": metric_value_json(&m.value),
                "kind": match m.kind {
                    MetricKind::Counter => "counter",
                    MetricKind::Gauge => "gauge",
                },
                "unit": &m.unit,
            })
        })
        .collect();
    canonical.sort_by(|a, b| {
        a.get("key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .cmp(b.get("key").and_then(|v| v.as_str()).unwrap_or(""))
    });

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(serde_json::Value::Array(canonical).to_string().as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}

fn metric_value_json(value: &MetricValue) -> serde_json::Value {
    match value {
        MetricValue::F64(v) => serde_json::json!(v),
        MetricValue::I64(v) => serde_json::json!(v),
        MetricValue::Str(v) => serde_json::json!(v),
        MetricValue::Bool(v) => serde_json::json!(v),
    }
}
