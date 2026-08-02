use std::path::{Path, PathBuf};

use async_trait::async_trait;
use cynic_protocol::{
    NodeControlPlan, NodeDesiredState, validate_node_control_plan, validate_node_desired_state,
};
use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::node_control::{NodeControlStoreError, NodeControlStorePort};

#[derive(Debug, Clone)]
pub struct SqliteNodeControlStore {
    path: PathBuf,
}

impl SqliteNodeControlStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, NodeControlStoreError> {
        let store = Self { path: path.into() };
        initialize(&open_connection(&store.path)?)?;
        Ok(store)
    }
}

#[async_trait]
impl NodeControlStorePort for SqliteNodeControlStore {
    async fn put_desired_state(
        &self,
        desired: NodeDesiredState,
    ) -> Result<(), NodeControlStoreError> {
        let violations = validate_node_desired_state(&desired);
        if !violations.is_empty() {
            return Err(NodeControlStoreError::Invalid(violations.join(",")));
        }
        let path = self.path.clone();
        run_blocking(move || {
            let connection = open_connection(&path)?;
            let json = serde_json::to_string(&desired).map_err(storage_json)?;
            let changed = connection
                .execute(
                    "INSERT INTO node_desired_state (node_id, generation, record_json)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(node_id) DO UPDATE SET
                    generation = excluded.generation,
                    record_json = excluded.record_json
                 WHERE excluded.generation > node_desired_state.generation",
                    params![desired.node_id, as_sql_integer(desired.generation)?, json],
                )
                .map_err(storage_error)?;
            if changed != 1 {
                return Err(NodeControlStoreError::Conflict(
                    "desired generation must increase".to_string(),
                ));
            }
            Ok(())
        })
        .await
    }

    async fn desired_state(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeDesiredState>, NodeControlStoreError> {
        read_record(self.path.clone(), node_id.to_string(), "node_desired_state").await
    }

    async fn put_status(&self, status: NodeControlPlan) -> Result<(), NodeControlStoreError> {
        let violations = validate_node_control_plan(&status);
        if !violations.is_empty() {
            return Err(NodeControlStoreError::Invalid(violations.join(",")));
        }
        let path = self.path.clone();
        run_blocking(move || {
            let connection = open_connection(&path)?;
            let json = serde_json::to_string(&status).map_err(storage_json)?;
            let changed = connection
                .execute(
                    "INSERT INTO node_status (node_id, desired_generation, record_json)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(node_id) DO UPDATE SET
                    desired_generation = excluded.desired_generation,
                    record_json = excluded.record_json
                 WHERE excluded.desired_generation >= node_status.desired_generation",
                    params![
                        status.node_id,
                        as_sql_integer(status.desired_generation)?,
                        json
                    ],
                )
                .map_err(storage_error)?;
            if changed != 1 {
                return Err(NodeControlStoreError::Conflict(
                    "status generation is stale".to_string(),
                ));
            }
            Ok(())
        })
        .await
    }

    async fn latest_status(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeControlPlan>, NodeControlStoreError> {
        read_record(self.path.clone(), node_id.to_string(), "node_status").await
    }
}

async fn read_record<T>(
    path: PathBuf,
    node_id: String,
    table: &'static str,
) -> Result<Option<T>, NodeControlStoreError>
where
    T: serde::de::DeserializeOwned + Send + 'static,
{
    run_blocking(move || {
        let connection = open_connection(&path)?;
        let query = format!("SELECT record_json FROM {table} WHERE node_id = ?1");
        let json = connection
            .query_row(&query, params![node_id], |row| row.get::<_, String>(0))
            .optional()
            .map_err(storage_error)?;
        json.map(|value| serde_json::from_str(&value).map_err(storage_json))
            .transpose()
    })
    .await
}

fn open_connection(path: &Path) -> Result<Connection, NodeControlStoreError> {
    let connection = Connection::open(path).map_err(storage_error)?;
    connection
        .busy_timeout(std::time::Duration::from_secs(2))
        .map_err(storage_error)?;
    Ok(connection)
}

fn initialize(connection: &Connection) -> Result<(), NodeControlStoreError> {
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = FULL;
         CREATE TABLE IF NOT EXISTS node_desired_state (
             node_id TEXT PRIMARY KEY NOT NULL,
             generation INTEGER NOT NULL,
             record_json TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS node_status (
             node_id TEXT PRIMARY KEY NOT NULL,
             desired_generation INTEGER NOT NULL,
             record_json TEXT NOT NULL
         );",
        )
        .map_err(storage_error)
}

fn as_sql_integer(value: u64) -> Result<i64, NodeControlStoreError> {
    i64::try_from(value).map_err(|_| {
        NodeControlStoreError::Invalid("node control integer exceeds sqlite range".to_string())
    })
}

fn storage_error(error: rusqlite::Error) -> NodeControlStoreError {
    NodeControlStoreError::Storage(error.to_string())
}

fn storage_json(error: serde_json::Error) -> NodeControlStoreError {
    NodeControlStoreError::Storage(error.to_string())
}

async fn run_blocking<T, F>(operation: F) -> Result<T, NodeControlStoreError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, NodeControlStoreError> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| NodeControlStoreError::Storage(format!("sqlite task failed: {error}")))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        NODE_CONTROL_PLAN_SCHEMA, NODE_DESIRED_STATE_SCHEMA, NodeControlConnectionState,
        NodeControlDecision, RUNTIME_SEED_REQUEST_SCHEMA, RuntimeSeedRequest,
    };

    fn desired(generation: u64) -> NodeDesiredState {
        NodeDesiredState {
            schema: NODE_DESIRED_STATE_SCHEMA.to_string(),
            node_id: "node-a".to_string(),
            role: "build_runner".to_string(),
            generation,
            policy_digest: format!("sha256:{}", "a".repeat(64)),
            release: RuntimeSeedRequest {
                schema: RUNTIME_SEED_REQUEST_SCHEMA.to_string(),
                node_id: "node-a".to_string(),
                release_id: "release-a".to_string(),
                release_epoch: 1,
                source_commit: "c".repeat(40),
                build_proof_digest: format!("sha256:{}", "d".repeat(64)),
                basket_digest: format!("sha256:{}", "b".repeat(64)),
                basket_path: "/inbox/release-a.tar".to_string(),
                detached_signature_path: "/inbox/release-a.tar.sig".to_string(),
                allowed_signers_path: "/etc/cynic/allowed-signers".to_string(),
                signer_identity: "release-authority".to_string(),
                signature_namespace: "cynic-runtime-seed".to_string(),
                required_seed_contract_revision: 2,
                install_root: "/var/lib/cynic/runtime".to_string(),
                staged_slot: "releases/release-a".to_string(),
                current_link: "current".to_string(),
                previous_link: "previous".to_string(),
                postflight_argv: vec!["/var/lib/cynic/runtime/current/bin/cynic".to_string()],
            },
            required_capabilities: Vec::new(),
            required_services: Vec::new(),
            paused: false,
        }
    }

    fn status(generation: u64) -> NodeControlPlan {
        NodeControlPlan {
            schema: NODE_CONTROL_PLAN_SCHEMA.to_string(),
            status: "converged".to_string(),
            node_id: "node-a".to_string(),
            role: "build_runner".to_string(),
            desired_generation: generation,
            observed_generation: Some(generation),
            connection: NodeControlConnectionState::Connected,
            decision: NodeControlDecision::Converged,
            evidence_refs: vec!["sha256:proof".to_string()],
            unknowns: Vec::new(),
        }
    }

    #[tokio::test]
    async fn desired_state_is_monotonic_and_readable() {
        let directory = tempfile::tempdir().unwrap();
        let store = SqliteNodeControlStore::open(directory.path().join("node.sqlite")).unwrap();

        store.put_desired_state(desired(1)).await.unwrap();
        let stale = store.put_desired_state(desired(1)).await.unwrap_err();
        assert!(matches!(stale, NodeControlStoreError::Conflict(_)));
        store.put_desired_state(desired(2)).await.unwrap();

        assert_eq!(
            store
                .desired_state("node-a")
                .await
                .unwrap()
                .unwrap()
                .generation,
            2
        );
    }

    #[tokio::test]
    async fn status_rejects_generation_regression() {
        let directory = tempfile::tempdir().unwrap();
        let store = SqliteNodeControlStore::open(directory.path().join("node.sqlite")).unwrap();

        store.put_status(status(2)).await.unwrap();
        let stale = store.put_status(status(1)).await.unwrap_err();

        assert!(matches!(stale, NodeControlStoreError::Conflict(_)));
        assert_eq!(
            store
                .latest_status("node-a")
                .await
                .unwrap()
                .unwrap()
                .desired_generation,
            2
        );
    }
}
