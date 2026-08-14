use crate::websocket::NodeMessage;

#[derive(Debug)]
pub(super) enum IncomingWireMessage {
    Stimulus {
        stimulus_id: String,
        domain: String,
        content: serde_json::Value,
        context: serde_json::Value,
        timeout_ms: u64,
        idempotent_key: String,
    },
    Pong,
    Backpressure {
        queue_usage_percent: u8,
        resume_after_ms: u64,
    },
    Eviction {
        reason: String,
        message: String,
    },
    Error {
        error_code: String,
        stimulus_id: Option<String>,
        message: String,
    },
    Other(NodeMessage),
}

pub(super) fn parse_incoming_wire_message(text: &str) -> Option<IncomingWireMessage> {
    let message = serde_json::from_str::<NodeMessage>(text).ok()?;
    Some(match message {
        NodeMessage::Stimulus {
            stimulus_id,
            domain,
            content,
            context,
            timeout_ms,
            idempotent_key,
        } => IncomingWireMessage::Stimulus {
            stimulus_id,
            domain,
            content,
            context,
            timeout_ms,
            idempotent_key,
        },
        NodeMessage::Pong { timestamp: _ } => IncomingWireMessage::Pong,
        NodeMessage::Backpressure {
            queue_usage_percent,
            resume_after_ms,
        } => IncomingWireMessage::Backpressure {
            queue_usage_percent,
            resume_after_ms,
        },
        NodeMessage::Eviction { reason, message } => {
            IncomingWireMessage::Eviction { reason, message }
        }
        NodeMessage::Error {
            error_code,
            stimulus_id,
            message,
        } => IncomingWireMessage::Error {
            error_code,
            stimulus_id,
            message,
        },
        other => IncomingWireMessage::Other(other),
    })
}
