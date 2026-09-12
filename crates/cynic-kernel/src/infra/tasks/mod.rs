//! Background task spawns — extracted from main.rs for composition root clarity.
//!
//! Each function takes its dependencies as parameters and spawns a tokio task.
//! Returns `JoinHandle<()>` — caller can collect into a `JoinSet` for
//! structured concurrency.
//! All tasks respect the CancellationToken for graceful shutdown.
//! All `.await` inside spawns are wrapped in `tokio::time::timeout()`.

mod convergence_consumer;
mod governance;
mod lifecycle;
mod nightshift;
mod organ_remediation;
mod runtime_loops;
mod state_log;
mod submission_queue;

pub use convergence_consumer::spawn_convergence_consumer;
pub use governance::spawn_governance_queue;
pub use lifecycle::{
    flush_usage_on_shutdown, spawn_auto_remediation, spawn_backfill, spawn_coord_expiry,
    spawn_introspection, spawn_pattern_analyzer, spawn_session_summarizer, spawn_usage_flush,
    spawn_usage_flush_with_organ,
};

pub fn spawn_signal_handler(
    shutdown: tokio_util::sync::CancellationToken,
) -> tokio::task::JoinHandle<()> {
    lifecycle::spawn_signal_handler(shutdown)
}

pub fn spawn_remediation_watcher(
    configs: std::collections::HashMap<String, crate::infra::config::BackendRemediation>,
    breakers: Vec<std::sync::Arc<dyn crate::domain::health_gate::HealthGate>>,
    task_health: std::sync::Arc<crate::infra::task_health::TaskHealth>,
    shutdown: tokio_util::sync::CancellationToken,
) -> tokio::task::JoinHandle<()> {
    lifecycle::spawn_remediation_watcher(configs, breakers, task_health, shutdown)
}

pub fn spawn_organ_remediation(
    organ_configs: Vec<crate::infra::config::OrganRemediation>,
    storage: std::sync::Arc<dyn crate::domain::storage::StoragePort>,
    task_health: std::sync::Arc<crate::infra::task_health::TaskHealth>,
    shutdown: tokio_util::sync::CancellationToken,
) -> tokio::task::JoinHandle<()> {
    organ_remediation::spawn_organ_remediation(organ_configs, storage, task_health, shutdown)
}
// DORMANT: nightshift (K15 violation — no consumer for dev domain crystals)
// Re-enable in Phase 2+ when temporal compounding has an acting consumer.
/*
pub use runtime_loops::{
    spawn_crystal_challenge_loop, spawn_discovery_loop, spawn_dog_heartbeat_loop,
    spawn_dog_perf_flush_loop, spawn_dog_ttl_checker, spawn_event_consumer,
    spawn_pattern_healing_alerter, spawn_probe_scheduler, spawn_service_ttl_checker,
    spawn_storage_metrics_emitter,
};
*/
pub use state_log::spawn_state_log;
// DORMANT: pinocchio submission queue disabled 2026-05-19 (0 verdicts onchain).
// Rewire when onchain anchoring is needed.
