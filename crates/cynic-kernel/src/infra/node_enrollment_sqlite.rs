use std::path::{Path, PathBuf};

use async_trait::async_trait;
use cynic_protocol::SignedNodeEnrollmentGrant;
use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::node_enrollment::{NodeEnrollmentStoreError, NodeEnrollmentStorePort};

#[derive(Debug, Clone)]
pub struct SqliteNodeEnrollmentStore {
    path: PathBuf,
}

impl SqliteNodeEnrollmentStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, NodeEnrollmentStoreError> {
        let store = Self { path: path.into() };
        initialize(&open_connection(&store.path)?)?;
        Ok(store)
    }
}

#[async_trait]
impl NodeEnrollmentStorePort for SqliteNodeEnrollmentStore {
    async fn consume_token_and_store_grant(
        &self,
        token_digest: &str,
        grant: SignedNodeEnrollmentGrant,
    ) -> Result<(), NodeEnrollmentStoreError> {
        let path = self.path.clone();
        let token_digest = token_digest.to_string();
        run_blocking(move || {
            let mut connection = open_connection(&path)?;
            let transaction = connection.transaction().map_err(storage_error)?;
            let consumed = transaction.execute(
                "INSERT INTO consumed_enrollment_tokens (token_digest, node_id) VALUES (?1, ?2)",
                params![token_digest, grant.grant.node_id],
            ).map_err(|error| {
                if error.to_string().contains("UNIQUE constraint failed") {
                    NodeEnrollmentStoreError::Conflict("machine enrollment token was already consumed".to_string())
                } else {
                    storage_error(error)
                }
            })?;
            if consumed != 1 {
                return Err(NodeEnrollmentStoreError::Storage("token consumption was not persisted".to_string()));
            }
            let json = serde_json::to_string(&grant)
                .map_err(|error| NodeEnrollmentStoreError::Storage(error.to_string()))?;
            transaction.execute(
                "INSERT INTO node_enrollment_grants (node_id, grant_json) VALUES (?1, ?2)
                 ON CONFLICT(node_id) DO UPDATE SET grant_json = excluded.grant_json",
                params![grant.grant.node_id, json],
            ).map_err(storage_error)?;
            transaction.commit().map_err(storage_error)
        }).await
    }

    async fn grant(
        &self,
        node_id: &str,
    ) -> Result<Option<SignedNodeEnrollmentGrant>, NodeEnrollmentStoreError> {
        let path = self.path.clone();
        let node_id = node_id.to_string();
        run_blocking(move || {
            let connection = open_connection(&path)?;
            let json = connection
                .query_row(
                    "SELECT grant_json FROM node_enrollment_grants WHERE node_id = ?1",
                    params![node_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(storage_error)?;
            json.map(|value| {
                serde_json::from_str(&value)
                    .map_err(|error| NodeEnrollmentStoreError::Storage(error.to_string()))
            })
            .transpose()
        })
        .await
    }

    async fn replace_grant(
        &self,
        expected_grant_id: &str,
        expected_generation: u64,
        grant: SignedNodeEnrollmentGrant,
    ) -> Result<(), NodeEnrollmentStoreError> {
        let path = self.path.clone();
        let expected_grant_id = expected_grant_id.to_string();
        run_blocking(move || {
            let mut connection = open_connection(&path)?;
            let transaction = connection.transaction().map_err(storage_error)?;
            let current_json: Option<String> = transaction
                .query_row(
                    "SELECT grant_json FROM node_enrollment_grants WHERE node_id = ?1",
                    params![grant.grant.node_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage_error)?;
            let current: SignedNodeEnrollmentGrant = current_json
                .ok_or_else(|| {
                    NodeEnrollmentStoreError::Conflict("current grant is absent".to_string())
                })
                .and_then(|json| {
                    serde_json::from_str(&json)
                        .map_err(|error| NodeEnrollmentStoreError::Storage(error.to_string()))
                })?;
            if current.grant.grant_id != expected_grant_id
                || current.grant.generation != expected_generation
                || grant.grant.generation != expected_generation.saturating_add(1)
            {
                return Err(NodeEnrollmentStoreError::Conflict(
                    "grant fencing generation changed".to_string(),
                ));
            }
            let json = serde_json::to_string(&grant)
                .map_err(|error| NodeEnrollmentStoreError::Storage(error.to_string()))?;
            transaction
                .execute(
                    "UPDATE node_enrollment_grants SET grant_json = ?1 WHERE node_id = ?2",
                    params![json, grant.grant.node_id],
                )
                .map_err(storage_error)?;
            transaction.commit().map_err(storage_error)
        })
        .await
    }
}

fn open_connection(path: &Path) -> Result<Connection, NodeEnrollmentStoreError> {
    let connection = Connection::open(path).map_err(storage_error)?;
    connection
        .busy_timeout(std::time::Duration::from_secs(2))
        .map_err(storage_error)?;
    Ok(connection)
}

fn initialize(connection: &Connection) -> Result<(), NodeEnrollmentStoreError> {
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = FULL;
         CREATE TABLE IF NOT EXISTS consumed_enrollment_tokens (
             token_digest TEXT PRIMARY KEY NOT NULL,
             node_id TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS node_enrollment_grants (
             node_id TEXT PRIMARY KEY NOT NULL,
             grant_json TEXT NOT NULL
         );",
        )
        .map_err(storage_error)
}

fn storage_error(error: rusqlite::Error) -> NodeEnrollmentStoreError {
    NodeEnrollmentStoreError::Storage(error.to_string())
}

async fn run_blocking<T, F>(operation: F) -> Result<T, NodeEnrollmentStoreError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, NodeEnrollmentStoreError> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| {
            NodeEnrollmentStoreError::Storage(format!("sqlite task failed: {error}"))
        })?
}
