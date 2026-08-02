//! Read-only runtime schema observation.
//!
//! Schema mutation belongs to a separately authorized migrator identity. The
//! steady-state runtime identity observes the required contract and fails
//! closed when an operator migration is required.

use crate::storage::SurrealHttpStorage;

pub(crate) async fn observe_schema(
    storage: &SurrealHttpStorage,
) -> Result<(), crate::domain::storage::StorageError> {
    let result = storage.query_one("INFO FOR DB;").await.map_err(|error| {
        crate::domain::storage::StorageError::ConnectionFailed(format!(
            "runtime identity cannot observe the database schema: {error}"
        ))
    })?;
    let info = result.first().ok_or_else(|| {
        crate::domain::storage::StorageError::ConnectionFailed(
            "database schema observation returned no proof".to_string(),
        )
    })?;
    required_schema_blockers(info).map_or(Ok(()), |blockers| {
        Err(crate::domain::storage::StorageError::ConnectionFailed(
            format!("storage schema migration required: {}", blockers.join(",")),
        ))
    })
}

fn required_schema_blockers(info: &serde_json::Value) -> Option<Vec<String>> {
    let tables = info.get("tables").and_then(serde_json::Value::as_object);
    let mut blockers = Vec::new();
    for table in [
        "agent_session",
        "agent_workload_delivery",
        "agent_workspace_claim",
        "runtime_proof_commit",
    ] {
        let Some(definition) = tables
            .and_then(|tables| tables.get(table))
            .and_then(serde_json::Value::as_str)
        else {
            blockers.push(format!("table_missing:{table}"));
            continue;
        };
        if matches!(
            table,
            "agent_workload_delivery" | "agent_workspace_claim" | "runtime_proof_commit"
        ) && !definition.contains("SCHEMAFULL")
        {
            blockers.push(format!("table_not_schemafull:{table}"));
        }
    }
    (!blockers.is_empty()).then_some(blockers)
}

#[cfg(test)]
mod tests {
    use super::required_schema_blockers;
    use serde_json::json;

    #[test]
    fn runtime_schema_observer_requires_exact_table_modes_without_mutating() {
        let valid = json!({"tables": {
            "agent_session": "DEFINE TABLE agent_session TYPE NORMAL SCHEMAFULL PERMISSIONS NONE",
            "agent_workload_delivery": "DEFINE TABLE agent_workload_delivery TYPE NORMAL SCHEMAFULL PERMISSIONS NONE",
            "agent_workspace_claim": "DEFINE TABLE agent_workspace_claim TYPE NORMAL SCHEMAFULL PERMISSIONS NONE",
            "runtime_proof_commit": "DEFINE TABLE runtime_proof_commit TYPE NORMAL SCHEMAFULL PERMISSIONS NONE"
        }});
        assert!(required_schema_blockers(&valid).is_none());

        let drifted = json!({"tables": {
            "agent_session": "DEFINE TABLE agent_session SCHEMAFULL",
            "agent_workload_delivery": "DEFINE TABLE agent_workload_delivery SCHEMAFULL",
            "agent_workspace_claim": "DEFINE TABLE agent_workspace_claim SCHEMALESS",
            "runtime_proof_commit": "DEFINE TABLE runtime_proof_commit SCHEMAFULL"
        }});
        assert_eq!(
            required_schema_blockers(&drifted).unwrap(),
            vec!["table_not_schemafull:agent_workspace_claim"]
        );
    }
}
