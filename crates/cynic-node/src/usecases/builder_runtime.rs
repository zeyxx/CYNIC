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

/// Resolve the webhook/admin HMAC secret.
///
/// Precedence: `CYNIC_FORGEJO_WEBHOOK_SECRET` env first (systemd units,
/// containers), then a secret file (fleet automation without root unit
/// edits): `CYNIC_WEBHOOK_SECRET_FILE` when set, else
/// `<state_root>/identity/webhook.secret` — the same directory that holds
/// the node keys, owned by the same user. Blank or unreadable anywhere
/// yields `None` and the admin endpoints refuse closed; a broken secret
/// file must never fail boot.
pub fn resolve_webhook_secret(state_root: &str) -> Option<String> {
    // `#![forbid(unsafe_code)]`: env access stays in this thin untested
    // shell; every decision lives in the pure function below.
    let env = std::env::var("CYNIC_FORGEJO_WEBHOOK_SECRET").ok();
    let file_path = std::env::var("CYNIC_WEBHOOK_SECRET_FILE")
        .ok()
        .filter(|path| !path.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(state_root)
                .join("identity")
                .join("webhook.secret")
        });
    resolve_webhook_secret_with(env.as_deref(), &file_path)
}

/// Pure secret precedence: env (trimmed, non-blank) wins, then the file,
/// then `None`. Unit-testable without touching process-global env.
pub fn resolve_webhook_secret_with(
    env_secret: Option<&str>,
    file_path: &std::path::Path,
) -> Option<String> {
    if let Some(secret) = env_secret {
        if !secret.trim().is_empty() {
            return Some(secret.trim().to_string());
        }
    }
    webhook_secret_from_file(file_path)
}

/// Read the secret from one file. Pure enough to unit test: trims, refuses
/// blank, swallows I/O errors into `None` (fail-closed downstream).
pub fn webhook_secret_from_file(path: &std::path::Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let secret = raw.trim().to_string();
    if secret.is_empty() {
        None
    } else {
        Some(secret)
    }
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

    let registry_path = node_config
        .build_scheduler
        .as_ref()
        .map(|s| s.registry_path.clone())
        .unwrap_or_else(crate::config::default_runtime_registry_path);

    let transport_uri = node_config
        .build_scheduler
        .as_ref()
        .and_then(|scheduler| scheduler.transport_uri.clone())
        .or_else(|| {
            let adapter = crate::adapters::registry::LocalRegistryAdapter::new(&registry_path);
            adapter
                .repository_transport_uri(&node_config.node_id, &repo_id)
                .ok()
                .flatten()
        })
        .or_else(|| {
            // Fallback to reading the in-tree repository transport if available
            let in_tree_registry = std::path::Path::new("infra/registry.json");
            if in_tree_registry.exists() {
                let adapter =
                    crate::adapters::registry::LocalRegistryAdapter::new(in_tree_registry);
                adapter
                    .repository_transport_uri(&node_config.node_id, &repo_id)
                    .ok()
                    .flatten()
            } else {
                None
            }
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
        webhook_secret: resolve_webhook_secret(
            &node_config
                .agent_workload_state_root
                .to_string_lossy()
                .to_string(),
        ),
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
        binary_link_path: std::env::var("CYNIC_BINARY_LINK_PATH").ok(),
    };

    run_http_server(http_config, store, listen_addr, shutdown)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cynic-secret-test-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("identity")).unwrap();
        dir
    }

    #[test]
    fn a_secret_file_resolves_when_no_env_is_set() {
        let root = unique_dir("file");
        let path = root.join("identity").join("webhook.secret");
        std::fs::write(&path, "file-secret\n").unwrap();
        assert_eq!(
            resolve_webhook_secret_with(None, &path),
            Some("file-secret".to_string())
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn env_wins_over_the_secret_file() {
        let root = unique_dir("env");
        let path = root.join("identity").join("webhook.secret");
        std::fs::write(&path, "file-secret").unwrap();
        assert_eq!(
            resolve_webhook_secret_with(Some("env-secret"), &path),
            Some("env-secret".to_string())
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_blank_env_falls_through_to_the_file() {
        let root = unique_dir("blank-env");
        let path = root.join("identity").join("webhook.secret");
        std::fs::write(&path, "file-secret").unwrap();
        assert_eq!(
            resolve_webhook_secret_with(Some("  "), &path),
            Some("file-secret".to_string())
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_secret_file_yields_none_not_a_boot_failure() {
        let root = unique_dir("missing");
        assert_eq!(
            resolve_webhook_secret_with(None, &root.join("identity").join("webhook.secret")),
            None
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_blank_secret_file_yields_none() {
        let root = unique_dir("blank");
        let path = root.join("identity").join("webhook.secret");
        std::fs::write(&path, "  \n").unwrap();
        assert_eq!(resolve_webhook_secret_with(None, &path), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_default_path_scopes_to_the_state_root_identity_dir() {
        let root = unique_dir("scoped");
        let path = root.join("identity").join("webhook.secret");
        std::fs::write(&path, "scoped-secret").unwrap();
        assert_eq!(
            resolve_webhook_secret_with(None, &path),
            Some("scoped-secret".to_string())
        );
        assert!(path.starts_with(&root));
        let _ = std::fs::remove_dir_all(&root);
    }
}
