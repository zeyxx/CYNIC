use std::sync::Arc;

use tempfile::tempdir;

use super::session::handshake_path;
use super::{NodeMessage, PendingStimulus, PendingVerdict, WebSocketClient};
use crate::adapters::local_runtime_federation_queue_store::LocalRuntimeFederationQueueStore;
use crate::adapters::workload_request_signer::WorkloadRequestSigner;

fn test_client(queue_dir: &std::path::Path) -> WebSocketClient {
    WebSocketClient::new(
        "ws://127.0.0.1:3030/events/ws".to_string(),
        "api-key".to_string(),
        None,
        "node-public-key".to_string(),
        "node-a".to_string(),
        Arc::new(LocalRuntimeFederationQueueStore::new(
            queue_dir.to_path_buf(),
        )),
    )
}

fn test_signer(dir: &std::path::Path) -> WorkloadRequestSigner {
    let key_path = dir.join("signing.key");
    std::fs::write(&key_path, hex::encode([7_u8; 32])).expect("write signing key");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
            .expect("signing key perms");
    }
    WorkloadRequestSigner::new(key_path).expect("workload signer")
}

fn test_stimulus(stimulus_id: &str, queued_at: i64) -> PendingStimulus {
    PendingStimulus {
        stimulus_id: stimulus_id.to_string(),
        stimulus: NodeMessage::Stimulus {
            stimulus_id: stimulus_id.to_string(),
            domain: "fleet.host_exec".to_string(),
            content: serde_json::json!({ "cmd": ["true"] }),
            context: serde_json::json!({ "source": "test" }),
            timeout_ms: 1_000,
            idempotent_key: format!("idem-{stimulus_id}"),
        },
        received_at: queued_at,
    }
}

fn test_verdict(stimulus_id: &str, queued_at: i64) -> PendingVerdict {
    PendingVerdict {
        stimulus_id: stimulus_id.to_string(),
        verdict: serde_json::json!({ "status": "ok", "id": format!("verd_{stimulus_id}") }),
        signature: "sig".to_string(),
        queued_at,
    }
}

#[tokio::test]
async fn queue_round_trip_persists_stimuli_and_verdicts() {
    let dir = tempdir().expect("tempdir");
    let queue_dir = dir.path().join("queue");
    let client = test_client(&queue_dir);

    client.queue_stimulus(test_stimulus("stim-1", 10)).await;
    client.queue_stimulus(test_stimulus("stim-2", 20)).await;
    client.queue_verdict(test_verdict("stim-1", 30)).await;

    let reloaded = test_client(&queue_dir);
    reloaded
        .load_persistent_queues()
        .await
        .expect("load persistent queues");

    assert_eq!(reloaded.pending_stimulus_count().await, 2);
    assert_eq!(reloaded.pending_verdict_count().await, 1);

    let first = reloaded.dequeue_stimulus().await.expect("first stimulus");
    let second = reloaded.dequeue_stimulus().await.expect("second stimulus");
    let verdict = reloaded.dequeue_verdict().await.expect("first verdict");

    assert_eq!(first.stimulus_id, "stim-1");
    assert_eq!(second.stimulus_id, "stim-2");
    assert_eq!(verdict.stimulus_id, "stim-1");
}

#[tokio::test]
async fn dequeue_updates_persistent_queue_files() {
    let dir = tempdir().expect("tempdir");
    let queue_dir = dir.path().join("queue");
    let client = test_client(&queue_dir);

    client.queue_stimulus(test_stimulus("stim-1", 10)).await;
    client.queue_stimulus(test_stimulus("stim-2", 20)).await;

    let dequeued = client.dequeue_stimulus().await.expect("dequeue stimulus");
    assert_eq!(dequeued.stimulus_id, "stim-1");

    let reloaded = test_client(&queue_dir);
    reloaded
        .load_persistent_queues()
        .await
        .expect("reload queues after dequeue");

    assert_eq!(reloaded.pending_stimulus_count().await, 1);
    let remaining = reloaded
        .dequeue_stimulus()
        .await
        .expect("remaining stimulus");
    assert_eq!(remaining.stimulus_id, "stim-2");
}

#[test]
fn handshake_path_extracts_ws_route() {
    assert_eq!(
        handshake_path("ws://127.0.0.1:3030/events/ws"),
        "/events/ws"
    );
    assert_eq!(
        handshake_path("wss://kernel.example:443/events/ws"),
        "/events/ws"
    );
    assert_eq!(handshake_path("ws://127.0.0.1:3030"), "/events/ws");
}

#[test]
fn unsigned_handshake_request_carries_no_workload_signature() {
    let dir = tempdir().expect("tempdir");
    let client = test_client(dir.path());
    let request = client.build_request().expect("build request");
    assert!(!request.headers().contains_key("x-cynic-signature"));
    assert!(!request.headers().contains_key("x-cynic-timestamp"));
    assert!(!request.headers().contains_key("x-cynic-public-key"));
}

#[test]
fn signed_handshake_request_carries_verifiable_workload_signature() {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let dir = tempdir().expect("tempdir");
    let signer = test_signer(dir.path());
    let queue_dir = dir.path().join("queue");
    let client = WebSocketClient::new(
        "ws://127.0.0.1:3030/events/ws".to_string(),
        "api-key".to_string(),
        Some(signer),
        "node-public-key".to_string(),
        "node-a".to_string(),
        Arc::new(LocalRuntimeFederationQueueStore::new(queue_dir)),
    );
    let request = client.build_request().expect("build request");
    let headers = request.headers();
    let timestamp = headers["x-cynic-timestamp"]
        .to_str()
        .expect("timestamp header")
        .to_string();
    let signature_hex = headers["x-cynic-signature"]
        .to_str()
        .expect("signature header")
        .to_string();
    let public_key_hex = headers["x-cynic-public-key"]
        .to_str()
        .expect("public key header")
        .to_string();

    let payload = format!("{timestamp}GET|/events/ws|").into_bytes();
    let public_key_bytes: [u8; 32] = hex::decode(&public_key_hex)
        .expect("public key hex")
        .try_into()
        .expect("public key length");
    let signature_bytes: [u8; 64] = hex::decode(&signature_hex)
        .expect("signature hex")
        .try_into()
        .expect("signature length");
    VerifyingKey::from_bytes(&public_key_bytes)
        .expect("public key")
        .verify(
            &payload,
            &Signature::from_bytes(&signature_bytes),
        )
        .expect("workload signature verifies");
}
