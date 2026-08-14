//! Schema bootstrap and backward-compatible SurrealQL migrations.

use crate::storage::SurrealHttpStorage;

const MAIN_SCHEMA_SQL: &str = include_str!("schema_bootstrap.sql");
const OBSERVATION_MIGRATION_SQL: &str = include_str!("schema_observation_migration.sql");
const AGENT_TASK_MIGRATION_SQL: &str = include_str!("schema_agent_task_migration.sql");

pub(crate) async fn bootstrap_schema(storage: &SurrealHttpStorage) {
    if let Err(e) = storage.query(MAIN_SCHEMA_SQL).await {
        // CRITICAL, not WARNING: HNSW vector index (crystal_vec_idx) is in this batch.
        // Without it, search_crystals_semantic can fall back to non-semantic listing.
        tracing::error!(error = %e, "schema bootstrap failed — vector search may not work");
    }

    // Historical Surreal schemas used stricter optional observation fields.
    // `IF NOT EXISTS` does not migrate preexisting schema.
    if let Err(e) = storage.query(OBSERVATION_MIGRATION_SQL).await {
        tracing::error!(
            error = %e,
            "observation optional field schema migration failed"
        );
    }

    // Historical kernels wrote task timestamps as RFC3339 strings and omitted
    // nullable fields, which SurrealDB represents as NONE rather than NULL.
    if let Err(e) = storage.query(AGENT_TASK_MIGRATION_SQL).await {
        tracing::error!(error = %e, "agent task schema migration failed");
    }
}
