use cynic_protocol::{
    NodeControlPlan, NodeDesiredState, validate_node_control_plan, validate_node_desired_state,
};

use super::sanitize_record_id;
use crate::domain::node_control::{NodeControlStoreError, NodeControlStorePort};
use crate::storage::{SurrealHttpStorage, escape_surreal};

#[async_trait::async_trait]
impl NodeControlStorePort for SurrealHttpStorage {
    async fn put_desired_state(
        &self,
        desired: NodeDesiredState,
    ) -> Result<(), NodeControlStoreError> {
        let violations = validate_node_desired_state(&desired);
        if !violations.is_empty() {
            return Err(NodeControlStoreError::Invalid(violations.join(",")));
        }
        let json = serde_json::to_string(&desired).map_err(storage_json)?;
        let sql = format!(
            "UPSERT node_desired_state:`{id}` SET node_id = '{node}', generation = {generation}, record_json = '{json}' WHERE generation = NONE OR generation < {generation} RETURN AFTER;",
            id = sanitize_record_id(&desired.node_id),
            node = escape_surreal(&desired.node_id),
            generation = desired.generation,
            json = escape_surreal(&json),
        );
        let rows = self.query_one(&sql).await.map_err(storage_query)?;
        if rows.is_empty() {
            return Err(NodeControlStoreError::Conflict(
                "desired generation must increase".to_string(),
            ));
        }
        Ok(())
    }

    async fn desired_state(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeDesiredState>, NodeControlStoreError> {
        read_record(self, "node_desired_state", node_id).await
    }

    async fn put_status(&self, status: NodeControlPlan) -> Result<(), NodeControlStoreError> {
        let violations = validate_node_control_plan(&status);
        if !violations.is_empty() {
            return Err(NodeControlStoreError::Invalid(violations.join(",")));
        }
        let json = serde_json::to_string(&status).map_err(storage_json)?;
        let sql = format!(
            "UPSERT node_status:`{id}` SET node_id = '{node}', desired_generation = {generation}, record_json = '{json}' WHERE desired_generation = NONE OR desired_generation <= {generation} RETURN AFTER;",
            id = sanitize_record_id(&status.node_id),
            node = escape_surreal(&status.node_id),
            generation = status.desired_generation,
            json = escape_surreal(&json),
        );
        let rows = self.query_one(&sql).await.map_err(storage_query)?;
        if rows.is_empty() {
            return Err(NodeControlStoreError::Conflict(
                "status generation is stale".to_string(),
            ));
        }
        Ok(())
    }

    async fn latest_status(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeControlPlan>, NodeControlStoreError> {
        read_record(self, "node_status", node_id).await
    }
}

async fn read_record<T: serde::de::DeserializeOwned>(
    storage: &SurrealHttpStorage,
    table: &str,
    node_id: &str,
) -> Result<Option<T>, NodeControlStoreError> {
    let sql = format!(
        "SELECT record_json FROM {table}:`{}`;",
        sanitize_record_id(node_id)
    );
    let rows = storage.query_one(&sql).await.map_err(storage_query)?;
    rows.first()
        .map(|row| {
            let json = row["record_json"].as_str().ok_or_else(|| {
                NodeControlStoreError::Storage("stored record_json is absent".to_string())
            })?;
            serde_json::from_str(json).map_err(storage_json)
        })
        .transpose()
}

fn storage_query(error: crate::domain::storage::StorageError) -> NodeControlStoreError {
    NodeControlStoreError::Storage(error.to_string())
}

fn storage_json(error: serde_json::Error) -> NodeControlStoreError {
    NodeControlStoreError::Storage(error.to_string())
}
