//! Live WebSocket handshake proof against a running kernel.
//!
//! Requires env:
//! - `CYNIC_KERNEL_WS_URL` (default `ws://127.0.0.1:3030/events/ws`)
//! - `CYNIC_KERNEL_WS_API_KEY` (Bearer, presence only, never logged)
//! - `CYNIC_NODE_SIGNING_KEY_FILE` (absolute path of the enrolled workload
//!   Ed25519 key; the kernel rejects unsigned handshakes with 401)
//!
//! Without an enrolled signing key the test reports SKIP (explicit unknown)
//! instead of failing: an unsigned handshake is expected to be rejected.

use ed25519_dalek::Signer;
use futures_util::{SinkExt, StreamExt};
use http::Request;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn websocket_connects_to_kernel_and_registers() {
    let key_path = std::env::var("CYNIC_NODE_SIGNING_KEY_FILE").ok();
    let Some(key_path) = key_path else {
        eprintln!(
            "SKIP: CYNIC_NODE_SIGNING_KEY_FILE not set; kernel requires a signed handshake"
        );
        return;
    };
    let api_key = std::env::var("CYNIC_KERNEL_WS_API_KEY")
        .expect("CYNIC_KERNEL_WS_API_KEY must be set for websocket integration test");
    let url = std::env::var("CYNIC_KERNEL_WS_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:3030/events/ws".to_string());

    let raw = std::fs::read_to_string(&key_path).expect("read signing key");
    let bytes: [u8; 32] = hex::decode(raw.trim())
        .expect("signing key hex")
        .try_into()
        .expect("signing key length");
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&bytes);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_secs()
        .to_string();
    let path = url
        .find("://")
        .and_then(|scheme_end| {
            let rest = &url[scheme_end + 3..];
            rest.find('/')
                .map(|path_start| rest[path_start..].to_string())
        })
        .unwrap_or_else(|| "/events/ws".to_string());
    let mut payload = timestamp.as_bytes().to_vec();
    payload.extend_from_slice(b"GET");
    payload.push(b'|');
    payload.extend_from_slice(path.as_bytes());
    payload.push(b'|');
    let signature_hex = hex::encode(signing_key.sign(&payload).to_bytes());
    let public_key_hex = hex::encode(signing_key.verifying_key().as_bytes());

    let request = Request::builder()
        .uri(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("X-Node-Public-Key", "test-integration-pub-key")
        .header("X-Node-Identity", "test-integration-node")
        .header("X-Cynic-Public-Key", public_key_hex)
        .header("X-Cynic-Signature", signature_hex)
        .header("X-Cynic-Timestamp", timestamp)
        .body(())
        .expect("build request");

    let (ws_stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .expect("connect to kernel WebSocket");

    let (mut write, mut read) = ws_stream.split();

    let register_msg = serde_json::json!({
        "type": "register",
        "node_public_key": "test-integration-pub-key",
        "node_identity": "test-integration-node"
    });
    write
        .send(Message::Text(register_msg.to_string()))
        .await
        .expect("send register");

    let mut got_response = false;
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if text.contains("register_response") {
                        got_response = true;
                        break;
                    }
                }
                Ok(Message::Close(_)) => break,
                _ => {}
            }
        }
    })
    .await;

    assert!(timeout.is_ok(), "registration response timeout");
    assert!(
        got_response,
        "did not receive register_response from kernel"
    );
}
