use std::sync::Arc;

use tempfile::tempdir;

use super::{NodeMessage, PendingStimulus, PendingVerdict, WebSocketClient};
use crate::adapters::local_runtime_federation_queue_store::LocalRuntimeFederationQueueStore;

fn test_client(queue_dir: &std::path::Path) -> WebSocketClient {
    WebSocketClient::new(
        "ws://127.0.0.1:3030/events/ws".to_string(),
        "api-key".to_string(),
        "node-public-key".to_string(),
        "node-a".to_string(),
        Arc::new(LocalRuntimeFederationQueueStore::new(
            queue_dir.to_path_buf(),
        )),
    )
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
