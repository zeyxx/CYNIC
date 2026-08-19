use std::net::SocketAddr;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::ports::agent_workload_store_port::AgentWorkloadStorePort;
use crate::usecases::builder_http::{BuilderHttpConfig, run_http_server};
use crate::config;

pub struct BuilderRuntimeConfig {
    pub http_listen_addr: SocketAddr,
    pub state_root: String,
}

pub fn builder_runtime_config_from(node_config: &config::NodeControlConfig) -> BuilderRuntimeConfig {
    BuilderRuntimeConfig {
        http_listen_addr: node_config.builder_listen_addr,
        state_root: node_config.agent_workload_state_root.to_string_lossy().to_string(),
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
    };
    run_http_server(http_config, store, config.http_listen_addr, shutdown)
        .await
        .map_err(|e| e.to_string())
}
