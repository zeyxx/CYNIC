//! Mesh network topology persistence in SurrealDB.

use crate::domain::storage::{MeshNode, MeshRoute, StorageError};
use crate::storage::{SurrealHttpStorage, escape_surreal};

/// List all registered mesh nodes in SurrealDB.
pub(super) async fn list_mesh_nodes(
    storage: &SurrealHttpStorage,
) -> Result<Vec<MeshNode>, StorageError> {
    let rows = storage.query_one("SELECT * FROM mesh_nodes;").await?;
    let mut nodes = Vec::new();
    for row in &rows {
        if let Some(obj) = row.as_object() {
            nodes.push(MeshNode {
                id: obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                name: obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                public_key: obj
                    .get("public_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                mesh_ip: obj
                    .get("mesh_ip")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                external_endpoint: obj
                    .get("external_endpoint")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                last_seen: obj
                    .get("last_seen")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }
    Ok(nodes)
}

/// List all registered mesh routes in SurrealDB.
pub(super) async fn list_mesh_routes(
    storage: &SurrealHttpStorage,
) -> Result<Vec<MeshRoute>, StorageError> {
    let rows = storage.query_one("SELECT * FROM mesh_routes;").await?;
    let mut routes = Vec::new();
    for row in &rows {
        if let Some(obj) = row.as_object() {
            routes.push(MeshRoute {
                id: obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                source_node: obj
                    .get("source_node")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                target_node: obj
                    .get("target_node")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                allowed_ips: obj
                    .get("allowed_ips")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|val| val.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
            });
        }
    }
    Ok(routes)
}

/// Publish a mesh node's public key (IP is static).
pub(super) async fn publish_mesh_key(
    storage: &SurrealHttpStorage,
    mesh_ip: &str,
    public_key: &str,
) -> Result<MeshNode, StorageError> {
    // We use the mesh_ip as the ID since it is unique and statically assigned
    let id_key = super::sanitize_record_id(mesh_ip);
    let mesh_ip_escaped = escape_surreal(mesh_ip);
    let pubkey_escaped = escape_surreal(public_key);

    let upsert_sql = format!(
        "UPSERT mesh_nodes:`{id_key}` SET \
            public_key = '{pubkey_escaped}', \
            mesh_ip = '{mesh_ip_escaped}', \
            last_seen = time::now();"
    );
    storage.query_one(&upsert_sql).await?;

    let select_new_sql = format!("SELECT * FROM mesh_nodes:`{id_key}`;");
    let result = storage.query_one(&select_new_sql).await?;
    let row = result
        .first()
        .ok_or_else(|| StorageError::QueryFailed("Failed to retrieve node after upsert".into()))?;

    Ok(MeshNode {
        id: row["id"].as_str().unwrap_or("").to_string(),
        name: row["name"].as_str().unwrap_or("").to_string(),
        public_key: row["public_key"].as_str().unwrap_or("").to_string(),
        mesh_ip: row["mesh_ip"].as_str().unwrap_or("").to_string(),
        external_endpoint: row["external_endpoint"].as_str().unwrap_or("").to_string(),
        last_seen: row["last_seen"].as_str().unwrap_or("").to_string(),
    })
}
