use std::path::PathBuf;

use serde_json::Value;

pub const DEFAULT_NODE_PUBLIC_KEY: &str = "stub-public-key-phase-3-4-pending";
pub const DEFAULT_VERDICT_SIGNATURE: &str = "stub-signature-phase-3-4-pending";

#[derive(Debug, Clone)]
pub struct RuntimeFederationConfig {
    pub kernel_ws_url: String,
    pub api_key: String,
    pub node_public_key: String,
    pub node_identity: String,
    pub queue_dir: PathBuf,
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
        _ => PathBuf::from("/var/lib/cynic/node"),
    }
}

pub fn deterministic_fallback_verdict(stimulus_id: &str, now_secs: i64) -> FallbackVerdict {
    FallbackVerdict {
        verdict: serde_json::json!({
            "id": format!("verd_{stimulus_id}"),
            "domain": "unknown",
            "kind": "DETERMINISTIC_FALLBACK",
            "q_score": 0.5,
            "axiom_scores": {
                "FIDELITY": 0.5,
                "PHI": 0.5,
                "VERIFY": 0.5,
                "CULTURE": 0.5,
                "BURN": 0.5,
                "SOVEREIGNTY": 0.5
            },
            "dog_id": "deterministic-fallback-phase-3-2",
            "dog_scores": [0.5],
            "timestamp": now_secs
        }),
        signature: DEFAULT_VERDICT_SIGNATURE.to_string(),
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
        let local_mode = std::env::var("CYNIC_ALLOW_LOCAL_DEV_BUILD")
            .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"));
        let expected = if local_mode {
            PathBuf::from("/tmp/home/.cynic/node")
        } else {
            PathBuf::from("/tmp/home/node")
        };
        assert_eq!(
            default_queue_dir(Some(PathBuf::from("/tmp/home"))),
            expected
        );
        let expected_default = if local_mode {
            PathBuf::from("/tmp/cynic/node")
        } else {
            PathBuf::from("/var/lib/cynic/node")
        };
        assert_eq!(default_queue_dir(None), expected_default);
    }

    #[test]
    fn fallback_verdict_shape_is_stable() {
        let verdict = deterministic_fallback_verdict("stim-1", 42);
        assert_eq!(verdict.verdict["id"], "verd_stim-1");
        assert_eq!(verdict.verdict["kind"], "DETERMINISTIC_FALLBACK");
        assert_eq!(verdict.verdict["timestamp"], 42);
        assert_eq!(verdict.queued_at, 42);
    }
}
