use std::path::PathBuf;

use async_trait::async_trait;

use crate::domain::agent_session::{AgentSessionStoreError, AgentSessionStorePort};
use cynic_protocol::{AgentSessionRecord, AgentSessionState, validate_agent_session_record};

mod persistence;

use persistence::{
    decode_record, initialize, insert_record, open_connection, read_active, read_by_session_id,
    replace_record,
};

#[derive(Debug, Clone)]
pub struct SqliteAgentSessionStore {
    path: PathBuf,
}

impl SqliteAgentSessionStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, AgentSessionStoreError> {
        let store = Self { path: path.into() };
        let connection = open_connection(&store.path)?;
        initialize(&connection)?;
        Ok(store)
    }
}

#[async_trait]
impl AgentSessionStorePort for SqliteAgentSessionStore {
    async fn insert(&self, record: AgentSessionRecord) -> Result<(), AgentSessionStoreError> {
        let violations = validate_agent_session_record(&record);
        if !violations.is_empty() {
            return Err(AgentSessionStoreError::Invalid(violations.join(",")));
        }
        let path = self.path.clone();
        run_blocking(move || persistence::insert(&path, record)).await
    }

    async fn resolve_active(
        &self,
        session_id: &str,
        now_unix_ms: u64,
    ) -> Result<Option<AgentSessionRecord>, AgentSessionStoreError> {
        let path = self.path.clone();
        let session_id = session_id.to_string();
        run_blocking(move || {
            let connection = open_connection(&path)?;
            read_active(&connection, "session_id = ?1", &session_id, now_unix_ms)
        })
        .await
    }

    async fn resolve_active_credential(
        &self,
        credential_digest: &str,
        now_unix_ms: u64,
    ) -> Result<Option<AgentSessionRecord>, AgentSessionStoreError> {
        if !persistence::valid_sha256(credential_digest) {
            return Err(AgentSessionStoreError::Invalid(
                "credential digest must be sha256".to_string(),
            ));
        }
        let path = self.path.clone();
        let credential_digest = credential_digest.to_string();
        run_blocking(move || {
            let connection = open_connection(&path)?;
            read_active(
                &connection,
                "credential_digest = ?1",
                &credential_digest,
                now_unix_ms,
            )
        })
        .await
    }

    async fn revoke(
        &self,
        session_id: &str,
        expected_generation: u64,
        revoked_at_unix_ms: u64,
        reason: &str,
        evidence_ref: &str,
    ) -> Result<AgentSessionRecord, AgentSessionStoreError> {
        if session_id.trim().is_empty()
            || reason.trim().is_empty()
            || evidence_ref.trim().is_empty()
        {
            return Err(AgentSessionStoreError::Invalid(
                "revocation requires session id, reason, and evidence".to_string(),
            ));
        }
        let path = self.path.clone();
        let session_id = session_id.to_string();
        let reason = reason.to_string();
        let evidence_ref = evidence_ref.to_string();
        run_blocking(move || {
            let mut connection = open_connection(&path)?;
            let transaction = connection
                .transaction()
                .map_err(persistence::storage_error)?;
            let mut record = read_by_session_id(&transaction, &session_id)?.ok_or_else(|| {
                AgentSessionStoreError::Conflict("session does not exist".to_string())
            })?;
            if record.state != AgentSessionState::Active || record.generation != expected_generation
            {
                return Err(AgentSessionStoreError::Conflict(
                    "session state or fencing generation changed".to_string(),
                ));
            }
            record.state = AgentSessionState::Revoked;
            record.generation = record.generation.checked_add(1).ok_or_else(|| {
                AgentSessionStoreError::Conflict("session generation overflow".to_string())
            })?;
            record.revoked_at_unix_ms = Some(revoked_at_unix_ms);
            record.revocation_reason = Some(reason);
            record.evidence_refs.push(evidence_ref);
            replace_record(&transaction, &record, expected_generation)?;
            transaction.commit().map_err(persistence::storage_error)?;
            Ok(record)
        })
        .await
    }

    async fn rotate(
        &self,
        previous_session_id: &str,
        expected_generation: u64,
        replacement: AgentSessionRecord,
        rotated_at_unix_ms: u64,
        evidence_ref: &str,
    ) -> Result<AgentSessionRecord, AgentSessionStoreError> {
        if previous_session_id.trim().is_empty() || evidence_ref.trim().is_empty() {
            return Err(AgentSessionStoreError::Invalid(
                "rotation requires previous session id and evidence".to_string(),
            ));
        }
        let path = self.path.clone();
        let previous_session_id = previous_session_id.to_string();
        let evidence_ref = evidence_ref.to_string();
        run_blocking(move || {
            let mut connection = open_connection(&path)?;
            let transaction = connection
                .transaction()
                .map_err(persistence::storage_error)?;
            let mut previous =
                read_by_session_id(&transaction, &previous_session_id)?.ok_or_else(|| {
                    AgentSessionStoreError::Conflict("source session does not exist".to_string())
                })?;
            if previous.generation != expected_generation {
                return Err(AgentSessionStoreError::Conflict(
                    "session fencing generation changed".to_string(),
                ));
            }
            let violations = cynic_node_protocol::validate_agent_session_rotation(
                &previous,
                &replacement,
                rotated_at_unix_ms,
            );
            if !violations.is_empty() {
                return Err(AgentSessionStoreError::Invalid(violations.join(",")));
            }
            previous.state = AgentSessionState::Revoked;
            previous.generation = replacement.generation;
            previous.revoked_at_unix_ms = Some(rotated_at_unix_ms);
            previous.revocation_reason = Some("agent_session_rotated".to_string());
            previous.evidence_refs.push(evidence_ref);
            replace_record(&transaction, &previous, expected_generation)?;
            insert_record(&transaction, &replacement)?;
            transaction.commit().map_err(persistence::storage_error)?;
            Ok(replacement)
        })
        .await
    }

    async fn expire(&self, now_unix_ms: u64) -> Result<u64, AgentSessionStoreError> {
        let path = self.path.clone();
        run_blocking(move || {
            let mut connection = open_connection(&path)?;
            let transaction = connection
                .transaction()
                .map_err(persistence::storage_error)?;
            let mut statement = transaction
                .prepare(
                    "SELECT record_json FROM agent_sessions
                     WHERE state = 'active' AND expires_at_unix_ms <= ?1",
                )
                .map_err(persistence::storage_error)?;
            let rows = statement
                .query_map(
                    rusqlite::params![persistence::as_sql_integer(now_unix_ms)?],
                    |row| row.get::<_, String>(0),
                )
                .map_err(persistence::storage_error)?;
            let mut expired = Vec::new();
            for row in rows {
                expired.push(decode_record(&row.map_err(persistence::storage_error)?)?);
            }
            drop(statement);

            for record in &mut expired {
                let expected_generation = record.generation;
                record.state = AgentSessionState::Expired;
                record.generation = record.generation.checked_add(1).ok_or_else(|| {
                    AgentSessionStoreError::Conflict("session generation overflow".to_string())
                })?;
                record
                    .evidence_refs
                    .push(format!("agent-session-expired:{now_unix_ms}"));
                replace_record(&transaction, record, expected_generation)?;
            }
            transaction.commit().map_err(persistence::storage_error)?;
            Ok(expired.len() as u64)
        })
        .await
    }
}

async fn run_blocking<T, F>(operation: F) -> Result<T, AgentSessionStoreError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AgentSessionStoreError> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| AgentSessionStoreError::Storage(format!("sqlite task failed: {error}")))?
}

#[cfg(test)]
mod tests;
