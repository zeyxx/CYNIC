use std::path::PathBuf;
use std::sync::Arc;

use tokio::task::JoinHandle;

use crate::adapters::local_runtime_federation_queue_store::LocalRuntimeFederationQueueStore;
use crate::domain::runtime_federation::{
    DEFAULT_NODE_PUBLIC_KEY, RuntimeFederationConfig, default_queue_dir,
    deterministic_fallback_verdict, kernel_websocket_url,
};
use crate::ports::runtime_federation_port::{RuntimeFederationFactory, RuntimeFederationPort};
use crate::websocket::{NodeMessage, PendingStimulus, PendingVerdict, WebSocketClient};

#[derive(Clone)]
pub struct WebSocketFederationAdapter {
    client: Arc<WebSocketClient>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WebSocketFederationFactory;

impl RuntimeFederationFactory for WebSocketFederationFactory {
    type Federation = WebSocketFederationAdapter;

    fn build(&self, config: RuntimeFederationConfig) -> Self::Federation {
        WebSocketFederationAdapter::new(config)
    }
}

impl WebSocketFederationAdapter {
    pub fn from_env(kernel_http_url: &str, node_identity: &str) -> Self {
<<<<<<< HEAD
        let api_key = cynic_secrets::env_var("ws-api-key", "CYNIC_API_KEY").unwrap_or_else(|| {
            tracing::warn!("CYNIC_API_KEY not set; WebSocket authentication will fail");
            String::new()
        });
        let node_public_key = cynic_secrets::env_var("node-public-key", "CYNIC_NODE_PUBLIC_KEY")
            .unwrap_or_else(|| DEFAULT_NODE_PUBLIC_KEY.to_string());
=======
        let api_key = std::env::var("CYNIC_API_KEY").unwrap_or_else(|_| {
            tracing::warn!("CYNIC_API_KEY not set; WebSocket authentication will fail");
            String::new()
        });
        let node_public_key = std::env::var("CYNIC_NODE_PUBLIC_KEY")
            .unwrap_or_else(|_| DEFAULT_NODE_PUBLIC_KEY.to_string());
>>>>>>> github/main
        let queue_home = std::env::var("CYNIC_STATE_ROOT")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(PathBuf::from);
        Self::new(RuntimeFederationConfig {
            kernel_ws_url: kernel_websocket_url(kernel_http_url),
            api_key,
            node_public_key,
            node_identity: node_identity.to_string(),
            queue_dir: default_queue_dir(queue_home),
        })
    }

    pub fn new(config: RuntimeFederationConfig) -> Self {
        Self {
            client: Arc::new(WebSocketClient::new(
                config.kernel_ws_url,
                config.api_key,
                config.node_public_key,
                config.node_identity,
                Arc::new(LocalRuntimeFederationQueueStore::new(config.queue_dir)),
            )),
        }
    }
}

impl RuntimeFederationPort for WebSocketFederationAdapter {
    async fn load_persistent_queues(&self) -> Result<(), String> {
        self.client
            .load_persistent_queues()
            .await
            .map_err(|error| error.to_string())
    }

    fn spawn_connection(&self) -> JoinHandle<()> {
        let client = self.client.clone();
        tokio::spawn(async move {
            match client.run().await {
                Ok(_) => tracing::info!("WebSocket client exited cleanly"),
                Err(error) => tracing::error!("WebSocket client error: {error}"),
            }
        })
    }

    async fn process_next_stimulus(&self) -> Result<(), String> {
        let Some(stimulus) = self.client.dequeue_stimulus().await else {
            return Ok(());
        };
        let pending_verdict = fallback_verdict_for_stimulus(stimulus)?;
        self.client.queue_verdict(pending_verdict).await;
        tracing::debug!("stimulus processed and verdict queued");
        Ok(())
    }
}

fn fallback_verdict_for_stimulus(stimulus: PendingStimulus) -> Result<PendingVerdict, String> {
    let stimulus_id = stimulus.stimulus_id.clone();
    match &stimulus.stimulus {
        NodeMessage::Stimulus { .. } => {
            let now_secs = chrono::Utc::now().timestamp();
            let fallback = deterministic_fallback_verdict(&stimulus_id, now_secs);
            Ok(PendingVerdict {
                stimulus_id,
                verdict: fallback.verdict,
                signature: fallback.signature,
                queued_at: fallback.queued_at,
            })
        }
        _ => Err(format!(
            "stimulus {stimulus_id}: message is not a Stimulus variant"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{WebSocketFederationFactory, fallback_verdict_for_stimulus};
    use crate::ports::runtime_federation_port::RuntimeFederationFactory;
    use crate::websocket::{NodeMessage, PendingStimulus};

    #[test]
    fn fallback_verdict_requires_stimulus_message() {
        let error = fallback_verdict_for_stimulus(PendingStimulus {
            stimulus_id: "not-stimulus".to_string(),
            stimulus: NodeMessage::Ping { timestamp: 1 },
            received_at: 1,
        })
        .expect_err("non-stimulus message must fail");

        assert!(error.contains("message is not a Stimulus variant"));
    }

    #[test]
    fn fallback_verdict_uses_stimulus_identity() {
        let verdict = fallback_verdict_for_stimulus(PendingStimulus {
            stimulus_id: "stim-1".to_string(),
            stimulus: NodeMessage::Stimulus {
                stimulus_id: "stim-1".to_string(),
                domain: "unknown".to_string(),
                content: serde_json::json!({}),
                context: serde_json::json!({}),
                timeout_ms: 1000,
                idempotent_key: "idem-1".to_string(),
            },
            received_at: 1,
        })
        .expect("fallback verdict");

        assert_eq!(verdict.stimulus_id, "stim-1");
        assert_eq!(verdict.verdict["id"], "verd_stim-1");
<<<<<<< HEAD
        assert_eq!(verdict.verdict["status"], "blocked_unsigned");
        assert_eq!(verdict.verdict["mutates"], false);
=======
        assert_eq!(verdict.verdict["kind"], "DETERMINISTIC_FALLBACK");
>>>>>>> github/main
    }

    #[test]
    fn factory_builds_default_websocket_federation_adapter() {
        let factory = WebSocketFederationFactory;
        let _adapter = factory.build(crate::domain::runtime_federation::RuntimeFederationConfig {
            kernel_ws_url: "ws://127.0.0.1:3030/events/ws".to_string(),
            api_key: "key".to_string(),
            node_public_key: "public-key".to_string(),
            node_identity: "node-a".to_string(),
            queue_dir: std::path::PathBuf::from("/var/lib/cynic/node"),
        });
    }
}
