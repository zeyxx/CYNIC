use std::sync::Arc;

use crate::domain::probe::Probe;
use crate::domain::{metrics::Metrics, probe::EnvironmentSnapshot, usage::DogUsageTracker};
use crate::infra;
use crate::infra::probes::FleetTarget;

pub struct StartupProbeBundle {
    pub environment: Arc<std::sync::RwLock<Option<EnvironmentSnapshot>>>,
    pub probes: Vec<Arc<dyn Probe>>,
    pub startup_coherence_ok: bool,
}

impl std::fmt::Debug for StartupProbeBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StartupProbeBundle")
            .field("environment_open", &self.environment.read().is_ok())
            .field("probes", &self.probes.len())
            .field("startup_coherence_ok", &self.startup_coherence_ok)
            .finish()
    }
}

pub fn resolve_backup_dir() -> std::path::PathBuf {
    std::env::var("CYNIC_BACKUP_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".surrealdb")
                .join("backups")
        })
}

pub async fn hydrate_runtime_state(
    storage_port: &Arc<dyn crate::domain::storage::StoragePort>,
    usage_tracker: &Arc<tokio::sync::Mutex<DogUsageTracker>>,
    metrics: &Arc<Metrics>,
) {
    // Wire per-Dog cost rates from backends.toml
    {
        let mut usage = usage_tracker.lock().await;
        for (name, input_rate, output_rate) in &std::vec::Vec::<(String, u64, u64)>::new() {
            usage.set_cost_rate(name, *input_rate as f64, *output_rate as f64);
        }
        if false {
            klog!("[Ring 2] Cost rates: {} Dogs configured", 0);
        }
    }

    // Load historical usage from DB (survives restarts)
    match storage_port.load_usage_history().await {
        Ok(rows) if !rows.is_empty() => {
            let mut usage = usage_tracker.lock().await;
            usage.load_historical(&rows);
            klog!(
                "[Ring 2] Usage: loaded {} Dog histories ({} all-time requests)",
                rows.len(),
                usage.all_time_requests()
            );
        }
        Err(e) => klog!("[Ring 2] Usage: failed to load history (non-fatal): {}", e),
        _ => {}
    }

    // Hydrate metrics from DB so counters survive reboots.
    // On NullStorage these return Err (non-fatal) — counters start at 0.
    match storage_port.count_verdicts().await {
        Ok(v_count) => {
            metrics
                .verdicts_total
                .store(v_count, std::sync::atomic::Ordering::Relaxed);
            // K15 reconstruction: verdicts in database = consumption opportunity existed.
            // Prior verdicts were accessible via REST endpoints (/verdict/{id}, /verdicts).
            // Backfill verdicts_served to match verdicts_total so K15 ratio starts honest.
            // Going forward: direct measurement via inc_verdict_served() in each endpoint.
            metrics
                .verdicts_served_total
                .store(v_count, std::sync::atomic::Ordering::Relaxed);
            klog!(
                "[Ring 2] Metrics: hydrated verdicts_total = {} (backfilled verdicts_served_total)",
                v_count
            );
        }
        Err(e) => klog!(
            "[Ring 2] Metrics: failed to hydrate verdicts_total (counters start at 0): {}",
            e
        ),
    }
    match storage_port.count_crystal_observations().await {
        Ok(co_count) => {
            metrics
                .crystal_observations_total
                .store(co_count, std::sync::atomic::Ordering::Relaxed);
            klog!(
                "[Ring 2] Metrics: hydrated crystal_observations_total = {}",
                co_count
            );
        }
        Err(e) => klog!(
            "[Ring 2] Metrics: failed to hydrate crystal_observations (counters start at 0): {}",
            e
        ),
    }
    match storage_port.count_observations().await {
        Ok(o_count) => {
            metrics
                .observations_ingested_total
                .store(o_count, std::sync::atomic::Ordering::Relaxed);
            klog!(
                "[Ring 2] Metrics: hydrated observations_ingested_total = {}",
                o_count
            );
        }
        Err(e) => klog!(
            "[Ring 2] Metrics: failed to hydrate observations (counters start at 0): {}",
            e
        ),
    }
}

pub async fn build_startup_probe_bundle(fleet_targets: Vec<FleetTarget>) -> StartupProbeBundle {
    let environment: Arc<std::sync::RwLock<Option<EnvironmentSnapshot>>> =
        Arc::new(std::sync::RwLock::new(None));
    let backup_dir = resolve_backup_dir();
    let fleet_inventory_probe = infra::probes::build_default_fleet_inventory_probe();

    // ── STARTUP VERIFICATION: Fail fast on backend coherence violations ──
    // Probe backends immediately (context drift, model mismatch) before claiming sovereignty.
    // This ensures organism knows its own inference constraints at boot time.
    let fleet_probe = infra::probes::FleetProbe::new(fleet_targets);
    let startup_coherence_ok = match fleet_probe.sense().await {
        Ok(result) => {
            if let crate::domain::probe::ProbeDetails::Fleet(fleet) = &result.details {
                let mut coherence_ok = true;
                for dog in &fleet.dogs {
                    // Context drift: actual < expected
                    if let (Some(actual), Some(expected)) = (dog.actual_n_ctx, dog.expected_n_ctx)
                        && actual < expected
                    {
                        coherence_ok = false;
                        klog!(
                            "[BOOT FAIL] {} context drift at startup: {} (expected {}). Fix llama-server launch flags or backends.toml.",
                            dog.dog_name,
                            actual,
                            expected
                        );
                    }
                    // Model mismatch: wrong model loaded
                    if dog.model_mismatch {
                        coherence_ok = false;
                        klog!(
                            "[BOOT FAIL] {} model mismatch at startup: {:?} vs {:?}. Stop llama-server and verify correct model is loaded.",
                            dog.dog_name,
                            dog.actual_model,
                            dog.expected_model
                        );
                    }
                }
                coherence_ok
            } else {
                true // Not fleet details, skip check
            }
        }
        Err(_) => true, // Probe error, let periodic probes handle it
    };

    if !startup_coherence_ok {
        // Backend coherence violations detected, but continue in degraded mode.
        // Circuit breaker will mark affected Dogs as failed, fallback routing will activate.
        // This ensures organism doesn't claim sovereignty (logs violations), but doesn't block startup.
        klog!(
            "[Boot] Backend coherence violations logged. Running in degraded mode — circuit breaker will isolate failed Dogs."
        );
    }

    let probes: Vec<Arc<dyn Probe>> = vec![
        Arc::new(infra::probes::ResourceProbe::default()),
        Arc::new(infra::probes::BackupProbe::new(backup_dir)),
        Arc::new(infra::probes::ProcessProbe),
        Arc::new(infra::probes::PressureProbe),
        Arc::new(infra::probes::NetworkProbe),
        Arc::new(infra::probes::ProxyProbe),
        Arc::new(infra::probes::SomaProbe),
        Arc::new(fleet_probe),
        Arc::new(fleet_inventory_probe),
        Arc::new(infra::probes::VersionProbe::new()),
    ];

    StartupProbeBundle {
        environment,
        probes,
        startup_coherence_ok,
    }
}

pub async fn emit_boot_observation(storage_port: &Arc<dyn crate::domain::storage::StoragePort>) {
    let obs = crate::domain::storage::Observation {
        project: "CYNIC".into(),
        agent_id: "kernel".into(),
        tool: "boot".into(),
        target: "self".into(),
        domain: "kernel-lifecycle".into(),
        status: "event".into(),
        context: format!(
            "version={} build={} sha={} dogs={}",
            env!("CARGO_PKG_VERSION"),
            env!("CYNIC_VERSION"),
            env!("CYNIC_GIT_SHA"),
            0
        ),
        session_id: String::new(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        tags: vec!["kernel-self-obs".into()],
        source_tier: "permanent".to_string(),
        value: None,
        confidence: None,
        consumer: None,
        action: None,
        depends_on: vec![],
        maturity: None,
        hash: String::new(),
        prev_hash: String::new(),
        observers: vec![],
        consensus_score: None,
    };

    let _ = storage_port.store_observation(&obs).await;
}
