use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::PendingVerdict;

const UNSIGNED_NATIVE_ADAPTER_SIGNATURE: &str = "NATIVE_ADAPTER_UNSIGNED_YET";

pub(super) fn spawn_native_handler(
    domain: &str,
    stimulus_id: String,
    content: serde_json::Value,
    verdicts_queue: Arc<RwLock<VecDeque<PendingVerdict>>>,
) -> bool {
    match domain {
        "fleet.lxc_exec" => {
            tokio::spawn(async move {
                use crate::adapters::proxmox::lxc_bridge::{LxcExecBridge, LxcExecRequest};

                let verdict_payload = match serde_json::from_value::<LxcExecRequest>(content) {
                    Ok(req) => match LxcExecBridge::execute(req).await {
                        Ok(resp) => serde_json::json!({ "status": "success", "response": resp }),
                        Err(e) => serde_json::json!({ "status": "error", "message": e }),
                    },
                    Err(e) => {
                        serde_json::json!({ "status": "error", "message": format!("Invalid payload: {}", e) })
                    }
                };

                queue_native_verdict(verdicts_queue, stimulus_id, verdict_payload).await;
            });
            true
        }
        "fleet.host_exec" => {
            tokio::spawn(async move {
                use crate::adapters::host::{HostExecAdapter, HostExecRequest};

                let verdict_payload = match serde_json::from_value::<HostExecRequest>(content) {
                    Ok(req) => match HostExecAdapter::execute(req).await {
                        Ok(resp) => serde_json::json!({ "status": "success", "response": resp }),
                        Err(e) => serde_json::json!({ "status": "error", "message": e }),
                    },
                    Err(e) => {
                        serde_json::json!({ "status": "error", "message": format!("Invalid payload: {}", e) })
                    }
                };

                queue_native_verdict(verdicts_queue, stimulus_id, verdict_payload).await;
            });
            true
        }
        _ => false,
    }
}

async fn queue_native_verdict(
    verdicts_queue: Arc<RwLock<VecDeque<PendingVerdict>>>,
    stimulus_id: String,
    verdict: serde_json::Value,
) {
    let mut queue = verdicts_queue.write().await;
    queue.push_back(PendingVerdict {
        stimulus_id,
        verdict,
        signature: UNSIGNED_NATIVE_ADAPTER_SIGNATURE.to_string(),
        queued_at: chrono::Utc::now().timestamp(),
    });
}
