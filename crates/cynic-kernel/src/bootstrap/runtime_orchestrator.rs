use std::collections::HashMap;
use std::sync::Arc;

use crate::bootstrap::entry::KernelEntryFlags;
use cynic_inference::adapters::embedding::auto_recovery::AutoRecoveryEmbedding;
use cynic_inference::application::embedding_runtime::{
    EmbeddingRuntimeConfig, build_initial_embedding_port,
};
use cynic_inference::embedding::EmbeddingPort;
use tokio_util::sync::CancellationToken;

use crate::api;
use crate::backends;
use crate::bootstrap;
use crate::domain;
use crate::infra;
use crate::probe;
use cynic_inference::{SlotSemaphoreMap, SlotTracker};

pub async fn run_runtime(entry_flags: KernelEntryFlags) -> Result<(), Box<dyn std::error::Error>> {
    let node_config = probe::run(entry_flags.force_reprobe).await?;
    klog!("[Ring 0] Omniscience Active. Reality Mapped.");
    klog!(
        "[Ring 0] Host: {} | Compute: {:?} | VRAM: {}GB",
        std::env::consts::OS,
        node_config.compute.backend,
        node_config.compute.vram_gb,
    );

    let kernel_config = bootstrap::config::load_kernel_config_bundle();
    let backends_path = kernel_config.backends_path.clone();
    let storage_config = kernel_config.storage_config.clone();
    let backend_configs = kernel_config.backend_configs.clone();
    let dog_thresholds = Arc::clone(&kernel_config.dog_thresholds);
    let system_contract = Arc::new(std::sync::RwLock::new(kernel_config.system_contract));

    let storage_runtime = bootstrap::storage::connect_storage_runtime(&storage_config).await;
    let storage_port = storage_runtime.storage_port;
    let coord = storage_runtime.coord;
    let reconnector = storage_runtime.reconnector;
    let node_control_store = storage_runtime.node_control;
    let node_enrollment_store = storage_runtime.node_enrollment;

    {
        let configs = backend_configs.clone();
        tokio::spawn(async move {
            backends::health_probe::validate_config(configs).await;
        });
    }

    let slot_tracker = Arc::new(SlotTracker::new());
    let slot_semaphores = Arc::new(SlotSemaphoreMap::new());

    let embedding_runtime = EmbeddingRuntimeConfig::from_env();
    let initial_embed: Arc<dyn EmbeddingPort> =
        build_initial_embedding_port(&embedding_runtime).await;
    let embedding: Arc<AutoRecoveryEmbedding> = Arc::new(AutoRecoveryEmbedding::new(initial_embed));

    let task_health = Arc::new(infra::task_health::TaskHealth::new());
    let shutdown = CancellationToken::new();

    let chain_verified = infra::boot::seed_integrity_chain(&storage_port).await;

    let senses = crate::senses::build_sense_registry();
    if !senses.is_empty() {
        klog!("[senses] {} organ(s) registered", senses.len());
        for s in &senses {
            klog!("[senses]   → {}", s.name());
        }
    }

    let routing_calc = Arc::new(
        infra::routing_calc::RoutingCalculator::with_storage_and_backends(
            Arc::clone(&storage_port),
            &backend_configs,
        ),
    );

    {
        let organ_remediations = infra::config::load_organ_remediations(&backends_path);
        if !organ_remediations.is_empty() {
            klog!(
                "[Ring 2] Organ remediation: {} organs configured for auto-restart",
                organ_remediations.len()
            );
            infra::tasks::spawn_organ_remediation(
                organ_remediations,
                Arc::clone(&storage_port),
                Arc::clone(&task_health),
                shutdown.clone(),
            );
        }
    }

    let usage_tracker = Arc::new(tokio::sync::Mutex::new(
        domain::usage::DogUsageTracker::new(),
    ));
    let api_key = infra::boot::load_rest_api_key().map_err(std::io::Error::other)?;

    let role_keys = api::rest::types::RoleKeyMap::from_environment(api_key.clone())
        .map_err(std::io::Error::other)?;
    let agent_sessions =
        bootstrap::agent_session::load_agent_session_runtime().map_err(std::io::Error::other)?;
    let node_control = bootstrap::node_control::load_node_control_runtime(node_control_store);
    let node_enrollment =
        bootstrap::node_enrollment::load_node_enrollment_runtime(node_enrollment_store)
            .map_err(std::io::Error::other)?;

    let verdict_cache = Arc::new(domain::verdict_cache::VerdictCache::new());
    let metrics = Arc::new(domain::metrics::Metrics::new());

    bootstrap::runtime_state::hydrate_runtime_state(&storage_port, &usage_tracker, &metrics).await;

    let fleet_targets = infra::boot::build_fleet_targets(&HashMap::new(), &HashMap::new());
    let probe_bundle = bootstrap::runtime_state::build_startup_probe_bundle(fleet_targets).await;
    let environment = Arc::clone(&probe_bundle.environment);
    let _ = probe_bundle.probes;

    let runtime_services =
        bootstrap::services::build_runtime_services(&backend_configs, &dog_thresholds);

    let event_tx = runtime_services.event_tx.clone();
    let rest_state = bootstrap::rest_state::build_rest_state(
        bootstrap::rest_state::RestStateInputs {
            agent_sessions,
            node_control,
            node_enrollment,
            storage: Arc::clone(&storage_port),
            coord: Arc::clone(&coord),
            embedding: Arc::clone(&embedding),
            usage: Arc::clone(&usage_tracker),
            verdict_cache: Arc::clone(&verdict_cache),
            task_health: Arc::clone(&task_health),
            metrics: Arc::clone(&metrics),
            api_key,
            role_keys,
            storage_config: storage_config.clone(),
            chain_verified,
            environment: Arc::clone(&environment),
            system_contract: system_contract.clone(),
            backend_configs: backend_configs.clone(),
            senses,
            routing_calc: Arc::clone(&routing_calc),
            slot_semaphores: Arc::clone(&slot_semaphores),
            slot_tracker: Arc::clone(&slot_tracker),
        },
        runtime_services,
    );
    let rest_app = api::rest::router(Arc::clone(&rest_state));

    bootstrap::background::spawn_runtime_background_tasks(
        bootstrap::background::RuntimeBackgroundTaskInputs {
            mcp_mode: entry_flags.mcp_mode,
            embedding_runtime,
            embedding: Arc::clone(&embedding),
            coord: Arc::clone(&coord),
            storage_port: Arc::clone(&storage_port),
            reconnector: Arc::clone(&reconnector),
            usage_tracker: Arc::clone(&usage_tracker),
            metrics: Arc::clone(&metrics),
            task_health: Arc::clone(&task_health),
            event_tx: event_tx.clone(),
            rest_state: Arc::clone(&rest_state),
            environment: Arc::clone(&environment),
            shutdown: shutdown.clone(),
        },
    );

    let rest_addr = std::env::var("CYNIC_REST_ADDR")
        .unwrap_or_else(|_| domain::constants::DEFAULT_REST_ADDR.to_string());
    klog!("[Ring 3] REST API on http://{}", rest_addr);

    bootstrap::runtime_state::emit_boot_observation(&storage_port).await;

    klog!("╔══════════════════════════════════════╗");
    klog!("║   CYNIC SOVEREIGN — ALL SYSTEMS GO   ║");
    klog!("║   REST/WS: http://{} / ws://{}", rest_addr, rest_addr);
    klog!(
        "║   UDS:     unix://{}    ║",
        bootstrap::server::declared_kernel_socket().display()
    );
    klog!("║   Max confidence: phi^-1 = 0.618     ║");
    klog!("╚══════════════════════════════════════╝");

    let _ = bootstrap::server::run_rest_servers(bootstrap::server::RestServeInputs {
        rest_app,
        rest_addr,
        rest_state: Arc::clone(&rest_state),
        usage_tracker: Arc::clone(&usage_tracker),
        task_health: Arc::clone(&task_health),
        storage: Arc::clone(&storage_port),
        shutdown: shutdown.clone(),
    })
    .await?;

    Ok(())
}
