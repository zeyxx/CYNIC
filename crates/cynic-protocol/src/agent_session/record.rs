use std::collections::BTreeMap;

use super::types::{
    AGENT_SESSION_RECORD_SCHEMA, AgentSessionCredentialKind, AgentSessionRecord, AgentSessionState,
    valid_sha256,
};

pub fn validate_agent_session_record(record: &AgentSessionRecord) -> Vec<String> {
    let mut violations = Vec::new();
    if record.schema != AGENT_SESSION_RECORD_SCHEMA {
        violations.push("agent_session_record_schema_mismatch".to_string());
    }
    if record.session_id.trim().is_empty()
        || record.request_id.trim().is_empty()
        || record.agent_id.trim().is_empty()
        || record.node_id.trim().is_empty()
        || record.authority_revision == 0
        || record.generation == 0
    {
        violations.push("agent_session_record_identity_invalid".to_string());
    }
    if !valid_sha256(&record.delegation_digest) {
        violations.push("agent_session_record_delegation_digest_invalid".to_string());
    }
    if record.expires_at_unix_ms <= record.issued_at_unix_ms {
        violations.push("agent_session_record_expiry_invalid".to_string());
    }
    match record.credential_kind {
        AgentSessionCredentialKind::LocalPeerBinding => {
            if !record
                .transport_binding_sha256
                .as_deref()
                .is_some_and(valid_sha256)
            {
                violations.push("agent_session_record_transport_binding_missing".to_string());
            }
            if record.credential_digest.is_some() {
                violations.push("agent_session_local_peer_must_not_store_credential".to_string());
            }
        }
        AgentSessionCredentialKind::OpaqueReference | AgentSessionCredentialKind::SignedJwt => {
            if !record
                .credential_digest
                .as_deref()
                .is_some_and(valid_sha256)
            {
                violations.push("agent_session_record_credential_digest_missing".to_string());
            }
            if !record
                .confirmation_key_sha256
                .as_deref()
                .is_some_and(valid_sha256)
            {
                violations.push("agent_session_record_confirmation_key_missing".to_string());
            }
        }
    }
    if record.state == AgentSessionState::Revoked
        && (record.revoked_at_unix_ms.is_none()
            || record
                .revoked_at_unix_ms
                .is_some_and(|revoked_at| revoked_at < record.issued_at_unix_ms)
            || record
                .revocation_reason
                .as_deref()
                .is_none_or(str::is_empty))
    {
        violations.push("agent_session_record_revocation_proof_missing".to_string());
    }
    let mut capabilities = BTreeMap::new();
    for capability in &record.capabilities {
        if capability.id.trim().is_empty()
            || capabilities
                .insert(capability.id.as_str(), capability.mutates)
                .is_some()
        {
            violations.push("agent_session_record_capabilities_invalid".to_string());
        }
    }
    violations.sort();
    violations.dedup();
    violations
}
