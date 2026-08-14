use tracing::{debug, error, info, warn};

use crate::websocket::{
    NodeMessage, PendingStimulus, WebSocketClient, WsWriter, domain_dispatch,
    wire::{IncomingWireMessage, parse_incoming_wire_message},
};

impl WebSocketClient {
    pub(super) async fn handle_incoming_text(
        &self,
        write: &mut WsWriter,
        text: &str,
    ) -> Result<Option<Result<(), String>>, String> {
        if let Some(message) = parse_incoming_wire_message(text) {
            return self.handle_incoming_wire_message(write, message).await;
        }
        Ok(None)
    }

    async fn handle_incoming_wire_message(
        &self,
        write: &mut WsWriter,
        message: IncomingWireMessage,
    ) -> Result<Option<Result<(), String>>, String> {
        match message {
            IncomingWireMessage::Stimulus {
                stimulus_id,
                domain,
                content,
                context,
                timeout_ms,
                idempotent_key,
            } => {
                self.handle_stimulus(
                    write,
                    stimulus_id,
                    domain,
                    content,
                    context,
                    timeout_ms,
                    idempotent_key,
                )
                .await?;
            }
            IncomingWireMessage::Pong => {
                debug!("Received heartbeat pong");
            }
            IncomingWireMessage::Backpressure {
                queue_usage_percent,
                resume_after_ms,
            } => {
                warn!(
                    queue_usage_percent = queue_usage_percent,
                    resume_after_ms = resume_after_ms,
                    "Kernel backpressure applied, pausing stimulus requests"
                );
            }
            IncomingWireMessage::Eviction { reason, message } => {
                error!(
                    reason = %reason,
                    message = %message,
                    "Kernel evicted node, reconnecting..."
                );
                return Ok(Some(Err(format!("Evicted by kernel: {message}"))));
            }
            IncomingWireMessage::Error {
                error_code,
                stimulus_id,
                message,
            } => {
                error!(
                    error_code = %error_code,
                    stimulus_id = ?stimulus_id,
                    message = %message,
                    "Kernel error"
                );
            }
            IncomingWireMessage::Other(message) => {
                debug!("Received unexpected message type: {:?}", message);
            }
        }
        Ok(None)
    }

    async fn handle_stimulus(
        &self,
        write: &mut WsWriter,
        stimulus_id: String,
        domain: String,
        content: serde_json::Value,
        context: serde_json::Value,
        timeout_ms: u64,
        idempotent_key: String,
    ) -> Result<(), String> {
        if self.should_skip_stimulus(&stimulus_id).await {
            debug!(stimulus_id = %stimulus_id, "Skipping duplicate stimulus");
            let ack = NodeMessage::Ack {
                stimulus_id: stimulus_id.clone(),
                status: "already_processed".to_string(),
                verdict_id: format!("verd_{stimulus_id}"),
            };
            self.send_message(write, &ack).await?;
            return Ok(());
        }

        self.mark_stimulus_processed(stimulus_id.clone()).await;
        info!(
            stimulus_id = %stimulus_id,
            domain = %domain,
            timeout_ms = timeout_ms,
            "Received stimulus from kernel"
        );

        if !domain_dispatch::spawn_native_handler(
            domain.as_str(),
            stimulus_id.clone(),
            content.clone(),
            self.pending_verdicts.clone(),
        ) {
            let pending = PendingStimulus {
                stimulus_id: stimulus_id.clone(),
                stimulus: NodeMessage::Stimulus {
                    stimulus_id,
                    domain,
                    content,
                    context,
                    timeout_ms,
                    idempotent_key,
                },
                received_at: chrono::Utc::now().timestamp(),
            };
            self.queue_stimulus(pending).await;
        }
        Ok(())
    }

    pub(super) async fn flush_pending_verdicts(&self, write: &mut WsWriter) -> Result<(), String> {
        while let Some(pending) = self.dequeue_verdict().await {
            let verdict = NodeMessage::Verdict {
                stimulus_id: pending.stimulus_id.clone(),
                verdict: pending.verdict.clone(),
                signature: pending.signature.clone(),
                node_public_key: self.node_public_key.clone(),
            };
            self.send_message(write, &verdict).await?;
            info!(stimulus_id = %pending.stimulus_id, "Sent pending verdict to kernel");
        }
        Ok(())
    }
}
