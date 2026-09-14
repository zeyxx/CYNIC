use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::ports::agent_workload_store_port::AgentWorkloadStorePort;
use crate::usecases::builder_http::{BuilderHttpConfig, run_http_server};

#[derive(Debug, Clone)]
pub struct BuilderRuntimeConfig {
    pub http_listen_addr: std::net::SocketAddr,
    pub state_root: String,
    pub workspace_root: String,
    pub node_id: String,
    pub agent_id: String,
    pub webhook_secret: Option<String>,
    /// Registry-declared repository the webhook builds.
    pub repository_id: String,
    /// Secret-free source transport URI for that repository.
    pub transport_uri: String,
}

pub(crate) fn builder_runtime_config_from(
    node_config: &config::NodeControlConfig,
) -> BuilderRuntimeConfig {
    // Repository identity comes from the declared build-scheduler config;
    // the transport URI is derived from the registry-declared repository
    // id, never hardcoded per machine.
    let repo_id = node_config
        .build_scheduler
        .as_ref()
        .map(|scheduler| scheduler.repository_id.clone())
        .unwrap_or_else(crate::config::default_build_repository_id);

    let transport_uri = node_config
        .build_scheduler
        .as_ref()
        .and_then(|scheduler| scheduler.transport_uri.clone())
        .or_else(|| {
            let registry_path = node_config
                .build_scheduler
                .as_ref()
                .map(|s| s.registry_path.clone())
                .unwrap_or_else(crate::config::default_runtime_registry_path);
            let adapter = crate::adapters::registry::LocalRegistryAdapter::new(registry_path);
            adapter
                .repository_transport_uri(&node_config.node_id, &repo_id)
                .ok()
                .flatten()
        })
        .or_else(|| std::env::var("CYNIC_GIT_TRANSPORT_URI").ok())
        .unwrap_or_default();

    BuilderRuntimeConfig {
        http_listen_addr: node_config.builder_listen_addr,
        state_root: node_config
            .agent_workload_state_root
            .to_string_lossy()
            .to_string(),
        workspace_root: node_config.workspace_root.to_string_lossy().to_string(),
        node_id: node_config.node_id.clone(),
        agent_id: node_config.enrollment_seed_id.clone(),
        webhook_secret: std::env::var("CYNIC_FORGEJO_WEBHOOK_SECRET").ok(),
        repository_id: repo_id,
        transport_uri,
    }
}

pub async fn run_builder_runtime(
    config: BuilderRuntimeConfig,
    store: Arc<dyn AgentWorkloadStorePort>,
    shutdown: CancellationToken,
) -> Result<(), String> {
    let listen_addr = config.http_listen_addr;
    let http_config = BuilderHttpConfig {
        listen_addr,
        state_root: config.state_root,
        workspace_root: config.workspace_root,
        node_id: config.node_id,
        agent_id: config.agent_id,
        webhook_secret: config.webhook_secret,
        repository_id: config.repository_id,
        transport_uri: config.transport_uri,
    };

    run_http_server(http_config, store, listen_addr, shutdown)
        .await
        .map_err(|e| e.to_string())
}
