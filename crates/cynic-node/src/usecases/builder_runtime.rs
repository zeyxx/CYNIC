//! Builder runtime — point d'entrée pour le nœud builder.
//!
//! Activé uniquement si le rôle déclaré dans le registry est `build_runner`.
//! Ce module respecte le pattern fractal CYNIC :
//!   - desired state : rôle déclaré dans registry
//!   - observe : réception webhook forgejo + observation fédérée
//!   - reconcile : orchestrateur admission + grant
//!   - bounded : --yes/--apply gate
//!   - read-back : preuve signée + distribution artifacts

use std::net::SocketAddr;
use std::sync::Arc;

use cynic_protocol::{AgentWorkloadDelivery, AgentWorkloadDeliveryStatus};
use tokio_util::sync::CancellationToken;

use crate::config::NodeControlConfig;
use crate::usecases::build_submission_orchestrator_uc::orchestrate_build_submission;
use crate::usecases::builder_http::{self, BuilderHttpConfig, BuilderHttpState};

/// Configuration du builder runtime.
#[derive(Debug, Clone)]
pub struct BuilderRuntimeConfig {
    /// Adresse d'écoute HTTP pour les webhooks forgejo.
    pub listen_addr: SocketAddr,
    /// Chemin vers la clé de signature du nœud.
    pub signing_key_path: std::path::PathBuf,
    /// Chemin vers le registry.
    pub registry_path: String,
    /// Racine du state local.
    pub state_root: std::path::PathBuf,
    /// ID du nœud.
    pub node_id: String,
}

/// Lance le builder runtime.
pub async fn run_builder_runtime(
    config: BuilderRuntimeConfig,
    node_control: &NodeControlConfig,
    shutdown: CancellationToken,
) -> Result<(), String> {
    tracing::info!(
        node_id = %config.node_id,
        listen_addr = %config.listen_addr,
        "builder runtime starting"
    );

    let http_config = BuilderHttpConfig {
        node_id: config.node_id.clone(),
        signing_key_path: config.signing_key_path.clone(),
        state_root: config.state_root.clone(),
        registry_path: config.registry_path.clone(),
    };

    builder_http::run_http_server(http_config, config.listen_addr, shutdown).await
}
