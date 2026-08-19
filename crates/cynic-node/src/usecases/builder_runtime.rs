use std::net::SocketAddr;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::adapters::ed25519_build_workload_grant::Ed25519BuildWorkloadGrantSigner;
use crate::adapters::http_build_workload_submission::HttpBuildWorkloadSubmission;
use crate::config::NodeControlConfig;
use crate::ports::build_workload_grant_port::BuildWorkloadGrantSignerPort;
use crate::ports::build_workload_submission_port::BuildWorkloadSubmissionPort;
use crate::usecases::builder_http::{self, BuilderHttpConfig};

#[derive(Debug, Clone)]
pub struct BuilderRuntimeConfig {
    pub listen_addr: SocketAddr,
    pub signing_key_path: std::path::PathBuf,
    pub registry_path: String,
    pub state_root: std::path::PathBuf,
    pub workspace_root: String,
    pub node_id: String,
    pub kernel_url: String,
}

pub async fn run_builder_runtime(
    config: BuilderRuntimeConfig,
    _node_control: &NodeControlConfig,
    shutdown: CancellationToken,
) -> Result<(), String> {
    let signer = Ed25519BuildWorkloadGrantSigner::load_hex_file(&config.signing_key_path)
        .map_err(|e| format!("load signing key: {e}"))?;
    let signer_arc: Arc<dyn BuildWorkloadGrantSignerPort> = Arc::new(signer);

    let submitter = HttpBuildWorkloadSubmission::new(&config.kernel_url);
    let submitter_arc: Arc<dyn BuildWorkloadSubmissionPort> = Arc::new(submitter);

    let http_config = BuilderHttpConfig {
        node_id: config.node_id,
        signing_key_path: config.signing_key_path,
        state_root: config.state_root,
        registry_path: config.registry_path,
        workspace_root: config.workspace_root,
    };

    builder_http::run_http_server(http_config, signer_arc, submitter_arc, config.listen_addr, shutdown).await
}
