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

use tokio_util::sync::CancellationToken;

use crate::adapters::ed25519_build_workload_grant::Ed25519BuildWorkloadGrantSigner;
use crate::config::NodeControlConfig;
use crate::ports::build_workload_grant_port::BuildWorkloadGrantSignerPort;
use crate::usecases::builder_http::{self, BuilderHttpConfig};

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
    /// Racine du workspace (pour les builds).
    pub workspace_root: String,
    /// ID du nœud.
    pub node_id: String,
}

/// Lance le builder runtime.
pub async fn run_builder_runtime(
    config: BuilderRuntimeConfig,
    _node_control: &NodeControlConfig,
    shutdown: CancellationToken,
) -> Result<(), String> {
    tracing::info!(
        node_id = %config.node_id,
        listen_addr = %config.listen_addr,
        "builder runtime starting"
    );

    // Initialize the ed25519 signer
    let signer = Ed25519BuildWorkloadGrantSigner::load_hex_file(&config.signing_key_path)
        .map_err(|e| format!("failed to load signing key: {e}"))?;
    let signer_arc: Arc<dyn BuildWorkloadGrantSignerPort> = Arc::new(signer);

    let http_config = BuilderHttpConfig {
        node_id: config.node_id,
        signing_key_path: config.signing_key_path,
        state_root: config.state_root,
        registry_path: config.registry_path.to_string_lossy().to_string(),
        workspace_root: config.workspace_root.to_string_lossy().to_string(),
    };

    builder_http::run_http_server(http_config, signer_arc, config.listen_addr, shutdown).await
}
