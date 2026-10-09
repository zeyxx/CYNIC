use super::*;
use cynic_protocol::{AGENT_SESSION_RECORD_SCHEMA, AgentSessionCredentialKind, AgentSessionState};

fn record(session_id: &str, credential: &str) -> AgentSessionRecord {
    AgentSessionRecord {
        schema: AGENT_SESSION_RECORD_SCHEMA.to_string(),
        session_id: session_id.to_string(),
        request_id: format!("request-{session_id}"),
        agent_id: "agent-a".to_string(),
        agent_type: Some("codex".to_string()),
        node_id: "node-a".to_string(),
        delegation_digest: format!("sha256:{}", "a".repeat(64)),
        authority_revision: 1,
        credential_kind: AgentSessionCredentialKind::OpaqueReference,
        credential_digest: Some(credential.to_string()),
        confirmation_key_sha256: Some(format!("sha256:{}", "b".repeat(64))),
        transport_binding_sha256: None,
        issued_at_unix_ms: 1_000,
        expires_at_unix_ms: 10_000,
        generation: 1,
        state: AgentSessionState::Active,
        revoked_at_unix_ms: None,
        revocation_reason: None,
        capabilities: Vec::new(),
        evidence_refs: vec!["session-issue:test".to_string()],
    }
}

#[tokio::test]
async fn persists_resolves_and_revokes_without_storing_raw_credentials() {
    let directory = tempfile::tempdir().unwrap();
    let store = SqliteAgentSessionStore::open(directory.path().join("sessions.sqlite")).unwrap();
    let digest = format!("sha256:{}", "c".repeat(64));
    store.insert(record("session-1", &digest)).await.unwrap();

    let resolved = store
        .resolve_active_credential(&digest, 2_000)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resolved.session_id, "session-1");

    let revoked = store
        .revoke(
            "session-1",
            1,
            2_100,
            "operator_cancelled",
            "revocation:test",
        )
        .await
        .unwrap();
    assert_eq!(revoked.state, AgentSessionState::Revoked);
    assert_eq!(revoked.generation, 2);
    assert!(
        store
            .resolve_active("session-1", 2_200)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn rotates_session_atomically_with_fencing_read_back() {
    let directory = tempfile::tempdir().unwrap();
    let store = SqliteAgentSessionStore::open(directory.path().join("sessions.sqlite")).unwrap();
    let old_digest = format!("sha256:{}", "c".repeat(64));
    let previous = record("session-old", &old_digest);
    store.insert(previous.clone()).await.unwrap();

    let mut replacement = previous;
    replacement.session_id = "session-new".to_string();
    replacement.request_id = "renewal-1".to_string();
    replacement.credential_digest = Some(format!("sha256:{}", "d".repeat(64)));
    replacement.issued_at_unix_ms = 2_000;
    replacement.expires_at_unix_ms = 9_000;
    replacement.generation = 2;
    replacement.evidence_refs = vec!["agent-session-rotation:2000".to_string()];

    let rotated = store
        .rotate(
            "session-old",
            1,
            replacement,
            2_000,
            "agent-session-rotation:2000",
        )
        .await
        .unwrap();
    assert_eq!(rotated.session_id, "session-new");
    assert!(
        store
            .resolve_active("session-old", 2_001)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        store
            .resolve_active("session-new", 2_001)
            .await
            .unwrap()
            .unwrap()
            .generation,
        2
    );
}

#[tokio::test]
async fn expiry_is_persisted_with_a_new_fencing_generation() {
    let directory = tempfile::tempdir().unwrap();
    let store = SqliteAgentSessionStore::open(directory.path().join("sessions.sqlite")).unwrap();
    let digest = format!("sha256:{}", "d".repeat(64));
    store.insert(record("session-2", &digest)).await.unwrap();
    assert_eq!(store.expire(10_000).await.unwrap(), 1);
    assert!(
        store
            .resolve_active("session-2", 10_001)
            .await
            .unwrap()
            .is_none()
    );
}
