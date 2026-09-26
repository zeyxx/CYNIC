use std::fmt;
use std::sync::Arc;

use crate::api;
use crate::bootstrap::services::RuntimeServices;
use crate::domain;
use crate::infra;
use cynic_inference::adapters::embedding::auto_recovery::AutoRecoveryEmbedding;
use cynic_inference::embedding::EmbeddingPort;
use cynic_inference::{SlotSemaphoreMap, SlotTracker};

pub struct RestStateInputs {
    pub storage: Arc<dyn domain::storage::StoragePort>,
    pub coord: Arc<dyn domain::coord::CoordPort>,
    pub embedding: Arc<AutoRecoveryEmbedding>,
    pub usage: Arc<tokio::sync::Mutex<domain::usage::DogUsageTracker>>,
    pub verdict_cache: Arc<domain::verdict_cache::VerdictCache>,
    pub task_health: Arc<infra::task_health::TaskHealth>,
    pub metrics: Arc<domain::metrics::Metrics>,
    pub api_key: Option<String>,
    pub role_keys: api::rest::types::RoleKeyMap,
    pub storage_config: infra::config::StorageConfig,
    pub chain_verified: bool,
    pub environment: Arc<std::sync::RwLock<Option<domain::probe::EnvironmentSnapshot>>>,
    pub system_contract: Arc<std::sync::RwLock<domain::contract::SystemContract>>,
    pub backend_configs: Vec<infra::config::BackendConfig>,
    pub senses: Vec<Arc<dyn domain::organ::OrganPort>>,
    pub routing_calc: Arc<infra::routing_calc::RoutingCalculator>,
    pub slot_semaphores: Arc<SlotSemaphoreMap>,
    pub slot_tracker: Arc<SlotTracker>,
}

impl fmt::Debug for RestStateInputs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RestStateInputs")
            .field("storage", &"Arc<dyn StoragePort>")
            .field("coord", &"Arc<dyn CoordPort>")
            .field("embedding", &"Arc<AutoRecoveryEmbedding>")
            .field("usage", &"Arc<Mutex<DogUsageTracker>>")
            .field("verdict_cache", &self.verdict_cache)
            .field("task_health", &self.task_health)
            .field("metrics", &self.metrics)
            .field("has_api_key", &self.api_key.is_some())
            .field(
                "role_keys_present",
                &[
                    ("cortex", self.role_keys.cortex.is_some()),
                    ("organ", self.role_keys.organ.is_some()),
                    ("internal", self.role_keys.internal.is_some()),
                    ("legacy", self.role_keys.legacy.is_some()),
                ],
            )
            .field("storage_config", &self.storage_config)
            .field("chain_verified", &self.chain_verified)
            .field("environment", &"Arc<RwLock<Option<EnvironmentSnapshot>>>")
            .field("system_contract", &"Arc<RwLock<SystemContract>>")
            .field("backend_config_count", &self.backend_configs.len())
            .field("senses_count", &self.senses.len())
            .field("routing_calc", &"Arc<RoutingCalculator>")
            .field("slot_semaphores", &self.slot_semaphores)
            .field("slot_tracker", &self.slot_tracker)
            .finish()
    }
}

pub fn build_rest_state(
    inputs: RestStateInputs,
    runtime_services: RuntimeServices,
) -> Arc<api::rest::AppState> {
    Arc::new(api::rest::AppState {
        storage: inputs.storage,
        coord: inputs.coord,
        embedding: inputs.embedding as Arc<dyn EmbeddingPort>,
        usage: inputs.usage,
        verdict_cache: inputs.verdict_cache,
        task_health: inputs.task_health,
        metrics: inputs.metrics,
        api_key: inputs.api_key,
        role_keys: inputs.role_keys,
        storage_info: api::rest::StorageInfo {
            namespace: inputs.storage_config.namespace,
            database: inputs.storage_config.database,
        },
        rate_limiter: api::rest::PerIpRateLimiter::new(30),
        judge_limiter: api::rest::PerIpRateLimiter::new(10),
        ready_cache: api::rest::ReadyCache::new(),
        bg_semaphore: Arc::new(tokio::sync::Semaphore::new(
            domain::constants::BG_SEMAPHORE_PERMITS,
        )),
        bg_tasks: tokio_util::task::TaskTracker::new(),
        sse_semaphore: Arc::new(tokio::sync::Semaphore::new(
            domain::constants::SSE_SEMAPHORE_PERMITS,
        )),
        introspection_alerts: Arc::new(std::sync::RwLock::new(Vec::new())),
        event_tx: runtime_services.event_tx,
        chain_verified: std::sync::atomic::AtomicBool::new(inputs.chain_verified),
        environment: inputs.environment,
        registered_dogs: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        registered_services: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        system_contract: inputs.system_contract,
        backend_configs: inputs.backend_configs,
        enricher: runtime_services.enricher,
        senses: inputs.senses,
        domain_curations: runtime_services.domain_curations,
        domain_router: runtime_services.domain_router,
        routing_calc: inputs.routing_calc,
        dog_perf_collector: runtime_services.dog_perf_collector,
        slot_semaphores: inputs.slot_semaphores,
        slot_tracker: inputs.slot_tracker,
        proxy_targets: runtime_services.proxy_targets,
        mail: runtime_services.mail,
        node_registry: Arc::new(api::websocket::NodeRegistry::new(120)),
        zones: runtime_services.zones,
        producer_heartbeats: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        overdue_task_alerted: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        temporal_clock: std::sync::Arc::new(crate::temporal_clock::TemporalClock::new()),
    })
}
