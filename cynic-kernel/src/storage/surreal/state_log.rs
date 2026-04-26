use crate::domain::state_log::StateBlock;
use crate::domain::storage::StorageError;
use crate::storage::{SurrealHttpStorage, escape_surreal};

pub(super) async fn store_state_block(
    storage: &SurrealHttpStorage,
    block: &StateBlock,
) -> Result<(), StorageError> {
    let dogs_json = serde_json::to_string(&block.dogs)
        .map_err(|e| StorageError::QueryFailed(format!("serialize dogs: {e}")))?;

    let sql = format!(
        "CREATE state_log SET \
            seq = {seq}, \
            timestamp = '{ts}', \
            prev_hash = '{prev}', \
            dogs = {dogs}, \
            system_status = '{sys_status}', \
            system_healthy_dogs = {sys_hd}, \
            system_total_dogs = {sys_td}, \
            system_verdict_count = {sys_vc}, \
            system_total_tokens = {sys_tt}, \
            system_crystals_forming = {sys_cf}, \
            system_crystals_crystallized = {sys_cc}, \
            resource_cpu_pct = {r_cpu}, \
            resource_memory_used_gb = {r_mem}, \
            resource_disk_avail_gb = {r_disk}, \
            resource_uptime_secs = {r_up}, \
            hash = '{hash}';",
        seq = block.seq,
        ts = escape_surreal(&block.timestamp),
        prev = escape_surreal(&block.prev_hash),
        dogs = dogs_json,
        sys_status = escape_surreal(&block.system.status),
        sys_hd = block.system.healthy_dogs,
        sys_td = block.system.total_dogs,
        sys_vc = block.system.verdict_count,
        sys_tt = block.system.total_tokens,
        sys_cf = block.system.crystals_forming,
        sys_cc = block.system.crystals_crystallized,
        r_cpu = block.resource.cpu_pct,
        r_mem = block.resource.memory_used_gb,
        r_disk = block.resource.disk_avail_gb,
        r_up = block.resource.uptime_secs,
        hash = escape_surreal(&block.hash),
    );
    storage
        .query(&sql)
        .await
        .map_err(|e| StorageError::QueryFailed(format!("store state_block: {e}")))?;
    Ok(())
}

pub(super) async fn last_state_block(
    storage: &SurrealHttpStorage,
) -> Result<Option<StateBlock>, StorageError> {
    let sql = "SELECT * FROM state_log ORDER BY seq DESC LIMIT 1;";
    let rows = storage
        .query_one(sql)
        .await
        .map_err(|e| StorageError::QueryFailed(format!("last_state_block: {e}")))?;
    match rows.first() {
        Some(row) => {
            let block = row_to_state_block(row)?;
            Ok(Some(block))
        }
        None => Ok(None),
    }
}

pub(super) async fn list_state_blocks(
    storage: &SurrealHttpStorage,
    since: &str,
    limit: u32,
) -> Result<Vec<StateBlock>, StorageError> {
    let safe_limit = limit.min(1000);
    // SurrealDB stores timestamp as string — compare with >= works for RFC3339
    // but the default '1970-...' may not match. Use seq-based ordering as primary.
    let sql = if since == "1970-01-01T00:00:00Z" || since.is_empty() {
        format!("SELECT * FROM state_log ORDER BY seq ASC LIMIT {safe_limit};")
    } else {
        format!(
            "SELECT * FROM state_log WHERE timestamp >= '{}' ORDER BY seq ASC LIMIT {};",
            escape_surreal(since),
            safe_limit,
        )
    };
    let rows = storage
        .query_one(&sql)
        .await
        .map_err(|e| StorageError::QueryFailed(format!("list_state_blocks: {e}")))?;
    rows.iter().map(row_to_state_block).collect()
}

fn row_to_state_block(row: &serde_json::Value) -> Result<StateBlock, StorageError> {
    use crate::domain::state_log::{DogSnapshot, ResourceSnapshot, SystemSnapshot};

    let dogs: Vec<DogSnapshot> = row
        .get("dogs")
        .and_then(|v| {
            serde_json::from_value(v.clone())
                .inspect_err(|e| tracing::debug!("state_log dogs deserialize: {e}"))
                .ok()
        })
        .unwrap_or_default();

    let system = SystemSnapshot {
        status: row["system_status"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        healthy_dogs: row["system_healthy_dogs"].as_u64().unwrap_or(0) as usize,
        total_dogs: row["system_total_dogs"].as_u64().unwrap_or(0) as usize,
        verdict_count: row["system_verdict_count"].as_u64().unwrap_or(0),
        total_tokens: row["system_total_tokens"].as_u64().unwrap_or(0),
        crystals_forming: row["system_crystals_forming"].as_u64().unwrap_or(0) as usize,
        crystals_crystallized: row["system_crystals_crystallized"].as_u64().unwrap_or(0) as usize,
    };

    let resource = ResourceSnapshot {
        cpu_pct: row["resource_cpu_pct"].as_f64().unwrap_or(0.0),
        memory_used_gb: row["resource_memory_used_gb"].as_f64().unwrap_or(0.0),
        disk_avail_gb: row["resource_disk_avail_gb"].as_f64().unwrap_or(0.0),
        uptime_secs: row["resource_uptime_secs"].as_u64().unwrap_or(0),
    };

    Ok(StateBlock {
        seq: row["seq"].as_u64().unwrap_or(0),
        timestamp: row["timestamp"].as_str().unwrap_or("").to_string(),
        prev_hash: row["prev_hash"].as_str().unwrap_or("").to_string(),
        dogs,
        system,
        resource,
        hash: row["hash"].as_str().unwrap_or("").to_string(),
    })
}
