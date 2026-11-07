use std::path::PathBuf;

use cynic_types::paths::CynicPaths;
use serde_json::Value;

pub const DEFAULT_NODE_PUBLIC_KEY: &str = "workload-identity-unavailable";

#[derive(Debug, Clone)]
pub struct RuntimeFederationConfig {
    pub kernel_ws_url: String,
    pub api_key: String,
    pub node_public_key: String,
    pub node_identity: String,
    pub queue_dir: PathBuf,
    /// Absolute path of the workload Ed25519 signing key (None = unsigned handshake).
    pub signing_key_path: Option<PathBuf>,
    /// Absolute path of the ML-DSA-44 workload signing key (None = classical
    /// signature only, no post-quantum half).
    pub pqc_signing_key_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct FallbackVerdict {
    pub verdict: Value,
    pub signature: String,
    pub queued_at: i64,
}

pub fn kernel_websocket_url(kernel_http_url: &str) -> String {
    kernel_http_url
        .replace("http://", "ws://")
        .replace("https://", "wss://")
        + "/events/ws"
}

pub fn default_queue_dir(home: Option<PathBuf>) -> PathBuf {
    if std::env::var("CYNIC_ALLOW_LOCAL_DEV_BUILD").is_ok() {
        return home
            .map(|path| path.join(".cynic/node"))
            .unwrap_or_else(|| PathBuf::from("/tmp/cynic/node"));
    }

    if let Ok(state_root) = std::env::var("CYNIC_STATE_ROOT") {
        return PathBuf::from(state_root).join("node");
    }

    match home {
        Some(path) if !path.starts_with("/home/") => path.join("node"),
        _ => CynicPaths::state_dir().join("node"),
    }
}

pub fn deterministic_fallback_verdict(stimulus_id: &str, now_secs: i64) -> FallbackVerdict {
    FallbackVerdict {
        verdict: serde_json::json!({
            "schema": "cynic.runtime_federation.blocked_verdict.v1",
            "id": format!("verd_{stimulus_id}"),
            "status": "blocked_unsigned",
            "mutates": false,
            "blockers": ["runtime_federation_workload_signer_unavailable"],
            "unknowns": ["stimulus_outcome_unobserved"],
            "timestamp": now_secs
        }),
        signature: String::new(),
        queued_at: now_secs,
    }
}

#[cfg(test)]
mod tests {
    use super::{default_queue_dir, deterministic_fallback_verdict, kernel_websocket_url};
    use std::path::PathBuf;

    #[test]
    fn kernel_url_maps_http_to_websocket_endpoint() {
        assert_eq!(
            kernel_websocket_url("http://127.0.0.1:3030"),
            "ws://127.0.0.1:3030/events/ws"
        );
        assert_eq!(
            kernel_websocket_url("https://kernel.example"),
            "wss://kernel.example/events/ws"
        );
    }

    #[test]
    fn queue_dir_is_derived_without_reading_environment() {
        // `default_queue_dir()` checks `CYNIC_ALLOW_LOCAL_DEV_BUILD`, then
        // `CYNIC_STATE_ROOT`, then the explicit home / `CynicPaths::state_dir()`
        // (`CYNIC_STATE_DIR`, else `/var/lib/cynic`). Clear every ambient
        // override so the derivation is tested, not the CI/user environment.
        let _env_lock = crate::test_env::env_lock();
        let _local_dev = crate::test_env::EnvOverride::clear("CYNIC_ALLOW_LOCAL_DEV_BUILD");
        let _state_root = crate::test_env::EnvOverride::clear("CYNIC_STATE_ROOT");
        let _state_dir = crate::test_env::EnvOverride::clear("CYNIC_STATE_DIR");

        // An explicit home (not under `/home/`) is joined with `node`.
        assert_eq!(
            default_queue_dir(Some(PathBuf::from("/tmp/home"))),
            PathBuf::from("/tmp/home/node")
        );
        // An explicit system-style root is honored as-is.
        assert_eq!(
            default_queue_dir(Some(PathBuf::from("/var/lib/cynic"))),
            PathBuf::from("/var/lib/cynic/node")
        );
        // No home: fall back to the canonical state root (`/var/lib/cynic`
        // once `CYNIC_STATE_DIR` is cleared), regardless of the current user.
        assert_eq!(
            default_queue_dir(None),
            PathBuf::from("/var/lib/cynic/node")
        );

        // `CYNIC_STATE_ROOT` overrides both an explicit home and the default.
        let _pinned_root = crate::test_env::EnvOverride::set("CYNIC_STATE_ROOT", "/tmp/state-root");
        assert_eq!(
            default_queue_dir(Some(PathBuf::from("/tmp/home"))),
            PathBuf::from("/tmp/state-root/node")
        );
        assert_eq!(
            default_queue_dir(None),
            PathBuf::from("/tmp/state-root/node")
        );
    }

    #[test]
    fn fallback_verdict_shape_is_stable() {
        let verdict = deterministic_fallback_verdict("stim-1", 42);
        assert_eq!(verdict.verdict["id"], "verd_stim-1");
        assert_eq!(verdict.verdict["status"], "blocked_unsigned");
        assert_eq!(verdict.verdict["mutates"], false);
        assert_eq!(verdict.verdict["timestamp"], 42);
        assert_eq!(verdict.queued_at, 42);
    }
}
