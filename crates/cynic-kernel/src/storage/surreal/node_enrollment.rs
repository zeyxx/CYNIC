use cynic_protocol::SignedNodeEnrollmentGrant;

use super::sanitize_record_id;
use crate::domain::node_enrollment::{NodeEnrollmentStoreError, NodeEnrollmentStorePort};
use crate::storage::{SurrealHttpStorage, escape_surreal};

#[async_trait::async_trait]
impl NodeEnrollmentStorePort for SurrealHttpStorage {
    async fn consume_token_and_store_grant(
        &self,
        token_digest: &str,
        grant: SignedNodeEnrollmentGrant,
    ) -> Result<(), NodeEnrollmentStoreError> {
        let json = serde_json::to_string(&grant).map_err(storage_json)?;
        let sql = format!(
            "BEGIN TRANSACTION; \
             CREATE consumed_enrollment_token:`{token_id}` CONTENT {{ token_digest: '{token}', node_id: '{node}', consumed_at: time::now() }}; \
             CREATE node_enrollment_grant:`{node_id}` CONTENT {{ node_id: '{node}', grant_id: '{grant_id}', generation: {generation}, grant_json: '{json}', updated_at: time::now() }}; \
             COMMIT TRANSACTION;",
            token_id = sanitize_record_id(token_digest),
            token = escape_surreal(token_digest),
            node = escape_surreal(&grant.grant.node_id),
            node_id = sanitize_record_id(&grant.grant.node_id),
            grant_id = escape_surreal(&grant.grant.grant_id),
            generation = grant.grant.generation,
            json = escape_surreal(&json),
        );
        self.query(&sql).await.map_err(|error| {
            let message = error.to_string();
            if message.contains("already exists") || message.contains("already contains") {
                NodeEnrollmentStoreError::Conflict(
                    "machine enrollment token was already consumed".to_string(),
                )
            } else {
                NodeEnrollmentStoreError::Storage(message)
            }
        })?;
        Ok(())
    }

    async fn grant(
        &self,
        node_id: &str,
    ) -> Result<Option<SignedNodeEnrollmentGrant>, NodeEnrollmentStoreError> {
        let sql = format!(
            "SELECT grant_json FROM node_enrollment_grant:`{}`;",
            sanitize_record_id(node_id)
        );
        let rows = self.query_one(&sql).await.map_err(storage_query)?;
        rows.first()
            .map(|row| {
                let json = row["grant_json"].as_str().ok_or_else(|| {
                    NodeEnrollmentStoreError::Storage("stored grant_json is absent".to_string())
                })?;
                serde_json::from_str(json).map_err(storage_json)
            })
            .transpose()
    }

    async fn replace_grant(
        &self,
        expected_grant_id: &str,
        expected_generation: u64,
        grant: SignedNodeEnrollmentGrant,
    ) -> Result<(), NodeEnrollmentStoreError> {
        let json = serde_json::to_string(&grant).map_err(storage_json)?;
        let sql = format!(
            "UPDATE node_enrollment_grant:`{node_id}` SET grant_id = '{new_id}', generation = {new_generation}, grant_json = '{json}', updated_at = time::now() WHERE grant_id = '{expected_id}' AND generation = {expected_generation} RETURN AFTER;",
            node_id = sanitize_record_id(&grant.grant.node_id),
            new_id = escape_surreal(&grant.grant.grant_id),
            new_generation = grant.grant.generation,
            json = escape_surreal(&json),
            expected_id = escape_surreal(expected_grant_id),
        );
        if grant.grant.generation != expected_generation.saturating_add(1) {
            return Err(NodeEnrollmentStoreError::Conflict(
                "grant generation does not increment the fence".to_string(),
            ));
        }
        let rows = self.query_one(&sql).await.map_err(storage_query)?;
        if rows.is_empty() {
            return Err(NodeEnrollmentStoreError::Conflict(
                "grant fencing generation changed".to_string(),
            ));
        }
        Ok(())
    }
}

fn storage_query(error: crate::domain::storage::StorageError) -> NodeEnrollmentStoreError {
    NodeEnrollmentStoreError::Storage(error.to_string())
}

fn storage_json(error: serde_json::Error) -> NodeEnrollmentStoreError {
    NodeEnrollmentStoreError::Storage(error.to_string())
}
