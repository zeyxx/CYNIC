use std::net::SocketAddr;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::ports::agent_workload_store_port::AgentWorkloadStorePort;
use crate::usecases::builder_http::{BuilderHttpConfig, run_http_server};

pub struct BuilderRuntimeConfig {
    pub http_listen_addr: SocketAddr,
    pub state_root: String,
    pub workspace_root: String,
    pub node_id: String,
    pub webhook_secret: Option<String>,
}

pub fn builder_runtime_config_from(
    node_config: &config::NodeControlConfig,
) -> BuilderRuntimeConfig {
    BuilderRuntimeConfig {
        http_listen_addr: node_config.builder_listen_addr,
        state_root: node_config
            .agent_workload_state_root
            .to_string_lossy()
            .to_string(),
        workspace_root: node_config.workspace_root.to_string_lossy().to_string(),
        node_id: node_config.node_id.clone(),
        webhook_secret: std::env::var("CYNIC_FORGEJO_WEBHOOK_SECRET").ok(),
    }
}

pub async fn run_builder_runtime(
    config: BuilderRuntimeConfig,
    store: Arc<dyn AgentWorkloadStorePort>,
    shutdown: CancellationToken,
) -> Result<(), String> {
    let http_config = BuilderHttpConfig {
        listen_addr: config.http_listen_addr,
        state_root: config.state_root,
        workspace_root: config.workspace_root,
        node_id: config.node_id,
        webhook_secret: config.webhook_secret,
    };
    run_http_server(http_config, store, config.http_listen_addr, shutdown)
        .await
        .map_err(|e| e.to_string())
}
