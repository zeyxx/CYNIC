use futures_util::Sink;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use http::Request;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, info, warn};

use crate::adapters::workload_request_signer::apply_signature_headers;
use crate::websocket::{NodeMessage, WebSocketClient, WsReader, WsWriter};

/// Request path of the kernel WS URL ("ws://host:port/events/ws" -> "/events/ws").
/// The kernel verifies the workload signature over this path.
pub(super) fn handshake_path(kernel_url: &str) -> String {
    if let Some(scheme_end) = kernel_url.find("://") {
        let rest = &kernel_url[scheme_end + 3..];
        if let Some(path_start) = rest.find('/') {
            return rest[path_start..].to_string();
        }
    }
    "/events/ws".to_string()
}

impl WebSocketClient {
    pub async fn run(&self) -> Result<(), String> {
        let mut backoff_ms = 100u64;
        let max_backoff_ms = 30000u64;

        loop {
            match self.connect_and_run().await {
                Ok(_) => {
                    info!("WebSocket connection closed by kernel");
                    return Ok(());
                }
                Err(error) => {
                    warn!(
                        error = %error,
                        backoff_ms = backoff_ms,
                        "WebSocket connection failed, reconnecting..."
                    );
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
                }
            }
        }
    }

    async fn connect_and_run(&self) -> Result<(), String> {
        debug!("Connecting to kernel at {}", self.kernel_url);

        let request = self.build_request()?;
        let (ws_stream, _) = connect_async(request)
            .await
            .map_err(|error| error.to_string())?;
        info!("Connected to kernel at {}", self.kernel_url);

        let (mut write, mut read) = ws_stream.split();
        self.register_session(&mut write, &mut read).await?;
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    self.send_ping(&mut write).await?;
                }
                msg_result = read.next() => {
                    if let Some(result) = msg_result {
                        match result.map_err(|error| error.to_string())? {
                            Message::Text(text) => {
                                if let Some(err) = self.handle_incoming_text(&mut write, &text).await? {
                                    return err;
                                }
                            }
                            Message::Close(_) => {
                                info!("Kernel closed WebSocket connection");
                                return Ok(());
                            }
                            _ => {}
                        }
                    } else {
                        warn!("WebSocket stream ended");
                        return Err("WebSocket stream closed".into());
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)), if self.pending_verdict_count().await > 0 => {
                    self.flush_pending_verdicts(&mut write).await?;
                }
            }
        }
    }

    pub(super) fn build_request(&self) -> Result<Request<()>, String> {
        let mut builder = Request::builder()
            .uri(&self.kernel_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("X-Node-Public-Key", &self.node_public_key)
            .header("X-Node-Identity", &self.node_identity);
        if let Some(signer) = &self.signer {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| "system clock precedes unix epoch".to_string())?
                .as_secs()
                .to_string();
            let path = handshake_path(&self.kernel_url);
            let signature = signer.sign_request(&timestamp, "GET", &path, &[])?;
            builder = apply_signature_headers(builder, &signature);
        }
        builder
            .body(())
            .map_err(|error| format!("Failed to build WebSocket request: {error}"))
    }

    async fn register_session(
        &self,
        write: &mut WsWriter,
        read: &mut WsReader,
    ) -> Result<(), String> {
        let register_msg = NodeMessage::Register {
            node_public_key: self.node_public_key.clone(),
            node_identity: self.node_identity.clone(),
        };
        self.send_message(write, &register_msg).await?;
        debug!("Sent registration to kernel");

        loop {
            if let Some(msg_result) = read.next().await {
                match msg_result.map_err(|error| error.to_string())? {
                    Message::Text(text) => {
                        if let Ok(NodeMessage::RegisterResponse {
                            node_id,
                            kernel_version,
                            max_stimulus_timeout,
                            max_queue_size,
                        }) = serde_json::from_str(&text)
                        {
                            self.set_node_id(node_id.clone()).await;
                            info!(
                                node_id = %node_id,
                                kernel_version = %kernel_version,
                                max_stimulus_timeout = max_stimulus_timeout,
                                max_queue_size = max_queue_size,
                                "Registered with kernel"
                            );
                            return Ok(());
                        }
                    }
                    Message::Close(_) => {
                        return Err("Kernel closed connection during registration".into());
                    }
                    _ => {}
                }
            } else {
                return Err("WebSocket stream ended during registration".into());
            }
        }
    }

    async fn send_ping(&self, write: &mut WsWriter) -> Result<(), String> {
        let ping = NodeMessage::Ping {
            timestamp: chrono::Utc::now().timestamp(),
        };
        self.send_message(write, &ping).await?;
        debug!("Sent heartbeat ping");
        Ok(())
    }

    pub(super) async fn send_message<S>(
        &self,
        write: &mut S,
        msg: &NodeMessage,
    ) -> Result<(), String>
    where
        S: Sink<Message> + Unpin,
        S::Error: std::fmt::Display,
    {
        let json = serde_json::to_string(msg).map_err(|error| error.to_string())?;
        write
            .send(Message::Text(json))
            .await
            .map_err(|error| error.to_string())
    }
}
