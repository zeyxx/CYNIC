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
    if let Ok(secret) = std::env::var("CYNIC_FORGEJO_WEBHOOK_SECRET") {
        if !secret.trim().is_empty() {
            return Some(secret.trim().to_string());
        }
    }
    let path = std::env::var("CYNIC_WEBHOOK_SECRET_FILE")
        .ok()
        .filter(|path| !path.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(state_root)
                .join("identity")
                .join("webhook.secret")
        });
    webhook_secret_from_file(&path)
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
    use std::sync::{Mutex, MutexGuard};

    // Env vars are process-global: serialize every test that touches them.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        kept_env: Option<String>,
        kept_file: Option<String>,
    }

    impl EnvGuard {
        fn hold() -> (MutexGuard<'static, ()>, Self) {
            let guard = ENV_LOCK.lock().unwrap();
            let kept = Self {
                kept_env: std::env::var("CYNIC_FORGEJO_WEBHOOK_SECRET").ok(),
                kept_file: std::env::var("CYNIC_WEBHOOK_SECRET_FILE").ok(),
            };
            std::env::remove_var("CYNIC_FORGEJO_WEBHOOK_SECRET");
            std::env::remove_var("CYNIC_WEBHOOK_SECRET_FILE");
            (guard, kept)
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.kept_env {
                Some(value) => std::env::set_var("CYNIC_FORGEJO_WEBHOOK_SECRET", value),
                None => std::env::remove_var("CYNIC_FORGEJO_WEBHOOK_SECRET"),
            }
            match &self.kept_file {
                Some(value) => std::env::set_var("CYNIC_WEBHOOK_SECRET_FILE", value),
                None => std::env::remove_var("CYNIC_WEBHOOK_SECRET_FILE"),
            }
        }
    }

    fn unique_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cynic-secret-test-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("identity")).unwrap();
        dir
    }

    #[test]
    fn a_secret_file_resolves_when_no_env_is_set() {
        let (_lock, _guard) = EnvGuard::hold();
        let root = unique_dir("file");
        std::fs::write(
            root.join("identity").join("webhook.secret"),
            "file-secret\n",
        )
        .unwrap();
        assert_eq!(
            resolve_webhook_secret(&root.to_string_lossy()),
            Some("file-secret".to_string())
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn env_wins_over_the_secret_file() {
        let (_lock, _guard) = EnvGuard::hold();
        let root = unique_dir("env");
        std::fs::write(root.join("identity").join("webhook.secret"), "file-secret").unwrap();
        std::env::set_var("CYNIC_FORGEJO_WEBHOOK_SECRET", "env-secret");
        assert_eq!(
            resolve_webhook_secret(&root.to_string_lossy()),
            Some("env-secret".to_string())
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_secret_file_yields_none_not_a_boot_failure() {
        let (_lock, _guard) = EnvGuard::hold();
        let root = unique_dir("missing");
        assert_eq!(resolve_webhook_secret(&root.to_string_lossy()), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_blank_secret_file_yields_none() {
        let (_lock, _guard) = EnvGuard::hold();
        let root = unique_dir("blank");
        std::fs::write(root.join("identity").join("webhook.secret"), "  \n").unwrap();
        assert_eq!(resolve_webhook_secret(&root.to_string_lossy()), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_explicit_secret_file_path_is_honored() {
        let (_lock, _guard) = EnvGuard::hold();
        let root = unique_dir("explicit");
        let custom = root.join("custom.secret");
        std::fs::write(&custom, "custom-secret").unwrap();
        std::env::set_var("CYNIC_WEBHOOK_SECRET_FILE", &custom);
        assert_eq!(
            resolve_webhook_secret("/nonexistent-root"),
            Some("custom-secret".to_string())
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
