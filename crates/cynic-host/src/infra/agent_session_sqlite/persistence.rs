use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::agent_session::AgentSessionStoreError;
use cynic_protocol::{AgentSessionRecord, AgentSessionState, validate_agent_session_record};

pub(super) fn insert(
    path: &Path,
    record: AgentSessionRecord,
) -> Result<(), AgentSessionStoreError> {
    let connection = open_connection(path)?;
    insert_record(&connection, &record)
}

pub(super) fn insert_record(
    connection: &Connection,
    record: &AgentSessionRecord,
) -> Result<(), AgentSessionStoreError> {
    let record_json = encode_record(record)?;
    connection
        .execute(
            "INSERT INTO agent_sessions (
                session_id, agent_id, node_id, state, expires_at_unix_ms,
                generation, credential_digest, record_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.session_id,
                record.agent_id,
                record.node_id,
                state_name(record.state),
                as_sql_integer(record.expires_at_unix_ms)?,
                as_sql_integer(record.generation)?,
                record.credential_digest,
                record_json,
            ],
        )
        .map_err(|error| {
            if error.to_string().contains("UNIQUE constraint failed") {
                AgentSessionStoreError::Conflict(
                    "session id or credential digest already exists".to_string(),
                )
            } else {
                storage_error(error)
            }
        })
        .map(|_| ())
}

pub(super) fn open_connection(path: &Path) -> Result<Connection, AgentSessionStoreError> {
    let connection = Connection::open(path).map_err(storage_error)?;
    connection
        .busy_timeout(std::time::Duration::from_secs(2))
        .map_err(storage_error)?;
    Ok(connection)
}

pub(super) fn initialize(connection: &Connection) -> Result<(), AgentSessionStoreError> {
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;
             CREATE TABLE IF NOT EXISTS agent_sessions (
                 session_id TEXT PRIMARY KEY NOT NULL,
                 agent_id TEXT NOT NULL,
                 node_id TEXT NOT NULL,
                 state TEXT NOT NULL,
                 expires_at_unix_ms INTEGER NOT NULL,
                 generation INTEGER NOT NULL,
                 credential_digest TEXT,
                 record_json TEXT NOT NULL
             );
             CREATE UNIQUE INDEX IF NOT EXISTS agent_sessions_credential_digest
                 ON agent_sessions(credential_digest)
                 WHERE credential_digest IS NOT NULL;
             CREATE INDEX IF NOT EXISTS agent_sessions_active_expiry
                 ON agent_sessions(state, expires_at_unix_ms);",
        )
        .map_err(storage_error)
}

pub(super) fn read_active(
    connection: &Connection,
    predicate: &str,
    value: &str,
    now_unix_ms: u64,
) -> Result<Option<AgentSessionRecord>, AgentSessionStoreError> {
    let query = format!(
        "SELECT record_json FROM agent_sessions
         WHERE {predicate} AND state = 'active' AND expires_at_unix_ms > ?2"
    );
    let json = connection
        .query_row(
            &query,
            params![value, as_sql_integer(now_unix_ms)?],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?;
    json.map(|json| decode_record(&json)).transpose()
}

pub(super) fn read_by_session_id(
    connection: &Connection,
    session_id: &str,
) -> Result<Option<AgentSessionRecord>, AgentSessionStoreError> {
    let json = connection
        .query_row(
            "SELECT record_json FROM agent_sessions WHERE session_id = ?1",
            params![session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?;
    json.map(|json| decode_record(&json)).transpose()
}

pub(super) fn replace_record(
    connection: &Connection,
    record: &AgentSessionRecord,
    expected_generation: u64,
) -> Result<(), AgentSessionStoreError> {
    let violations = validate_agent_session_record(record);
    if !violations.is_empty() {
        return Err(AgentSessionStoreError::Invalid(violations.join(",")));
    }
    let changed = connection
        .execute(
            "UPDATE agent_sessions
             SET state = ?1, expires_at_unix_ms = ?2, generation = ?3, record_json = ?4
             WHERE session_id = ?5 AND generation = ?6",
            params![
                state_name(record.state),
                as_sql_integer(record.expires_at_unix_ms)?,
                as_sql_integer(record.generation)?,
                encode_record(record)?,
                record.session_id,
                as_sql_integer(expected_generation)?,
            ],
        )
        .map_err(storage_error)?;
    if changed != 1 {
        return Err(AgentSessionStoreError::Conflict(
            "session fencing generation changed".to_string(),
        ));
    }
    Ok(())
}

fn encode_record(record: &AgentSessionRecord) -> Result<String, AgentSessionStoreError> {
    serde_json::to_string(record).map_err(|error| {
        AgentSessionStoreError::Storage(format!("serialize session record: {error}"))
    })
}

pub(super) fn decode_record(json: &str) -> Result<AgentSessionRecord, AgentSessionStoreError> {
    let record: AgentSessionRecord = serde_json::from_str(json).map_err(|error| {
        AgentSessionStoreError::Storage(format!("deserialize session record: {error}"))
    })?;
    let violations = validate_agent_session_record(&record);
    if !violations.is_empty() {
        return Err(AgentSessionStoreError::Storage(format!(
            "stored session record is invalid: {}",
            violations.join(",")
        )));
    }
    Ok(record)
}

fn state_name(state: AgentSessionState) -> &'static str {
    match state {
        AgentSessionState::Active => "active",
        AgentSessionState::Revoked => "revoked",
        AgentSessionState::Expired => "expired",
    }
}

pub(super) fn as_sql_integer(value: u64) -> Result<i64, AgentSessionStoreError> {
    i64::try_from(value).map_err(|_| {
        AgentSessionStoreError::Invalid("session integer exceeds sqlite range".to_string())
    })
}

pub(super) fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

pub(super) fn storage_error(error: rusqlite::Error) -> AgentSessionStoreError {
    AgentSessionStoreError::Storage(error.to_string())
}
