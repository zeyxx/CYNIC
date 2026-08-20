//! SurrealDB backend implementation for CryptographicLease and monotonic fencing.
//!
//! Enforces database-level rejection of stale mutations:
//! - Leases are acquired via atomic SurrealDB transactions.
//! - Fencing tokens must be strictly monotonic (new_token > stored_token).
//! - All role transitions are immutably appended to `role_transition_record`.

use cynic_protocol::runtime_takeover::{
    CryptographicLease, RoleTransitionRecord,
};

use super::sanitize_record_id;
use crate::domain::storage::StorageError;
use crate::storage::{SurrealHttpStorage, escape_surreal};

#[derive(Debug, thiserror::Error)]
pub enum SurrealLeaseError {
    #[error("fencing token conflict: {0}")]
    FencingTokenConflict(String),
    #[error("lease expired or invalid: {0}")]
    InvalidLease(String),
    #[error("surreal storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("json serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl SurrealHttpStorage {
    /// Atomically acquires or renews a cryptographic lease in SurrealDB with strictly monotonic fencing.
    pub async fn acquire_cryptographic_lease(
        &self,
        lease: &CryptographicLease,
        now_unix_ms: u64,
    ) -> Result<RoleTransitionRecord, SurrealLeaseError> {
        let blockers = lease.validate(now_unix_ms);
        if !blockers.is_empty() {
            return Err(SurrealLeaseError::InvalidLease(blockers.join(",")));
        }

        let role_key = sanitize_record_id(&lease.role);
        let lease_json = serde_json::to_string(lease)?;

        let sql = format!(
            "BEGIN TRANSACTION; \
             LET $existing = SELECT * FROM cryptographic_lease:`{role_key}`; \
             LET $prior_token = IF array::len($existing) > 0 {{ array::first($existing).fencing_token }} ELSE {{ 0 }}; \
             LET $prior_node = IF array::len($existing) > 0 {{ array::first($existing).holder_node_id }} ELSE {{ NONE }}; \
             IF $prior_token >= {fencing_token} {{ \
                THROW 'fencing_token_not_monotonic'; \
             }}; \
             UPSERT ONLY cryptographic_lease:`{role_key}` SET \
                role = '{role}', \
                holder_node_id = '{holder}', \
                lease_id = '{lease_id}', \
                fencing_token = {fencing_token}, \
                granted_at_unix_ms = {granted_at}, \
                ttl_ms = {ttl}, \
                expires_at_unix_ms = {expires_at}, \
                authority_id = '{authority}', \
                signature_hex = '{sig}', \
                lease_json = '{lease_json}'; \
             LET $transition = CREATE ONLY role_transition_record CONTENT {{ \
                transition_id: 'tx-{fencing_token}', \
                from_node_id: $prior_node, \
                to_node_id: '{holder}', \
                role: '{role}', \
                fencing_token: {fencing_token}, \
                prior_fencing_token: $prior_token, \
                transitioned_at_unix_ms: {now_unix_ms}, \
                lease_signature_hex: '{sig}' \
             }}; \
             COMMIT TRANSACTION; \
             RETURN $transition;",
            role_key = role_key,
            role = escape_surreal(&lease.role),
            holder = escape_surreal(&lease.holder_node_id),
            lease_id = escape_surreal(&lease.lease_id),
            fencing_token = lease.fencing_token,
            granted_at = lease.granted_at_unix_ms,
            ttl = lease.ttl_ms,
            expires_at = lease.expires_at_unix_ms(),
            authority = escape_surreal(&lease.authority_id),
            sig = escape_surreal(&lease.signature_hex),
            lease_json = escape_surreal(&lease_json),
            now_unix_ms = now_unix_ms,
        );

        let rows = self.query_one(&sql).await.map_err(|e| {
            if e.to_string().contains("fencing_token_not_monotonic") {
                SurrealLeaseError::FencingTokenConflict(format!(
                    "proposed fencing token {} is not strictly monotonic",
                    lease.fencing_token
                ))
            } else {
                SurrealLeaseError::Storage(e)
            }
        })?;

        let transition: RoleTransitionRecord = match rows.first() {
            Some(row) => serde_json::from_value(row.clone())?,
            None => {
                let from_node_id = None;
                RoleTransitionRecord::new(
                    format!("tx-{}", lease.fencing_token),
                    from_node_id,
                    lease.holder_node_id.clone(),
                    lease.role.clone(),
                    lease.fencing_token,
                    0,
                    now_unix_ms,
                    lease.signature_hex.clone(),
                )
            }
        };

        Ok(transition)
    }

    /// Reads the active cryptographic lease for a role if unexpired at `now_unix_ms`.
    pub async fn get_active_cryptographic_lease(
        &self,
        role: &str,
        now_unix_ms: u64,
    ) -> Result<Option<CryptographicLease>, SurrealLeaseError> {
        let role_key = sanitize_record_id(role);
        let sql = format!(
            "SELECT lease_json FROM cryptographic_lease:`{role_key}` WHERE expires_at_unix_ms > {now_unix_ms};"
        );
        let rows = self.query_one(&sql).await?;
        match rows.first().and_then(|r| r["lease_json"].as_str()) {
            Some(json_str) => {
                let lease: CryptographicLease = serde_json::from_str(json_str)?;
                Ok(Some(lease))
            }
            None => Ok(None),
        }
    }

    /// Lists the full sequenced audit history of role transitions.
    pub async fn list_role_transition_history(
        &self,
        role: &str,
    ) -> Result<Vec<RoleTransitionRecord>, SurrealLeaseError> {
        let sql = format!(
            "SELECT * FROM role_transition_record WHERE role = '{role}' ORDER BY fencing_token ASC;",
            role = escape_surreal(role)
        );
        let rows = self.query_one(&sql).await?;
        let records: Vec<RoleTransitionRecord> = serde_json::from_value(serde_json::Value::Array(rows))?;
        Ok(records)
    }
}
