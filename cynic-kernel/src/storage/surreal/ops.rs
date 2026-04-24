use super::sanitize_record_id;
use crate::domain::storage::{StorageError, UsageRow};
use crate::organ::health::DogStats;
use crate::storage::{SurrealHttpStorage, escape_surreal};

pub(super) async fn flush_usage(
    storage: &SurrealHttpStorage,
    snapshot: &[(String, crate::domain::usage::DogUsage)],
) -> Result<(), StorageError> {
    if snapshot.is_empty() {
        return Ok(());
    }

    let mut sql = String::new();
    for (dog_id, usage) in snapshot {
        use std::fmt::Write;
        let id_key = sanitize_record_id(dog_id);
        let id_val = escape_surreal(dog_id);
        let _ = write!(
            sql,
            "UPSERT dog_usage:`{id_key}` SET \
                dog_id = '{id_val}', \
                prompt_tokens = {pt}, \
                completion_tokens = {ct}, \
                requests = {req}, \
                failures = {fail}, \
                total_latency_ms = {lat}, \
                updated_at = time::now(); ",
            id_key = id_key,
            id_val = id_val,
            pt = usage.prompt_tokens,
            ct = usage.completion_tokens,
            req = usage.requests,
            fail = usage.failures,
            lat = usage.total_latency_ms,
        );
    }
    storage.query(&sql).await?;
    Ok(())
}

pub(super) async fn load_usage_history(
    storage: &SurrealHttpStorage,
) -> Result<Vec<UsageRow>, StorageError> {
    let rows = storage.query_one("SELECT * FROM dog_usage;").await?;
    Ok(rows
        .iter()
        .map(|r| UsageRow {
            dog_id: r["dog_id"].as_str().unwrap_or("").to_string(),
            prompt_tokens: r["prompt_tokens"].as_u64().unwrap_or(0),
            completion_tokens: r["completion_tokens"].as_u64().unwrap_or(0),
            requests: r["requests"].as_u64().unwrap_or(0),
            failures: r["failures"].as_u64().unwrap_or(0),
            total_latency_ms: r["total_latency_ms"].as_u64().unwrap_or(0),
        })
        .collect())
}

pub(super) async fn flush_dog_stats(
    storage: &SurrealHttpStorage,
    stats: &[(String, DogStats)],
) -> Result<(), StorageError> {
    if stats.is_empty() {
        return Ok(());
    }

    let mut sql = String::new();
    for (dog_id, stats) in stats {
        use std::fmt::Write;
        let id_key = sanitize_record_id(dog_id);
        let id_val = escape_surreal(dog_id);
        let last_success_sql = match &stats.last_success {
            Some(ts) => format!("'{}'", escape_surreal(ts)),
            None => "NONE".to_string(),
        };
        let _ = write!(
            sql,
            "UPSERT dog_stats:`{id_key}` SET \
                dog_id = '{id_val}', \
                total_calls = {tc}, \
                success_count = {sc}, \
                zero_flood_count = {zf}, \
                collapse_count = {cc}, \
                parse_error_count = {pe}, \
                timeout_count = {to}, \
                api_error_count = {ae}, \
                last_success = {ls}, \
                total_latency_ms = {lat}, \
                total_completion_tokens = {tct}, \
                max_completion_tokens = {mct}, \
                updated_at = time::now(); ",
            id_key = id_key,
            id_val = id_val,
            tc = stats.total_calls,
            sc = stats.success_count,
            zf = stats.zero_flood_count,
            cc = stats.collapse_count,
            pe = stats.parse_error_count,
            to = stats.timeout_count,
            ae = stats.api_error_count,
            ls = last_success_sql,
            lat = stats.total_latency_ms,
            tct = stats.total_completion_tokens,
            mct = stats.max_completion_tokens,
        );
    }
    storage.query(&sql).await?;
    Ok(())
}

pub(super) async fn load_dog_stats(
    storage: &SurrealHttpStorage,
) -> Result<Vec<(String, DogStats)>, StorageError> {
    let rows = storage.query_one("SELECT * FROM dog_stats;").await?;
    Ok(rows
        .iter()
        .filter_map(|r| {
            let dog_id = r["dog_id"].as_str()?.to_string();
            if dog_id.is_empty() {
                return None;
            }
            Some((
                dog_id,
                DogStats {
                    total_calls: r["total_calls"].as_u64().unwrap_or(0),
                    success_count: r["success_count"].as_u64().unwrap_or(0),
                    zero_flood_count: r["zero_flood_count"].as_u64().unwrap_or(0),
                    collapse_count: r["collapse_count"].as_u64().unwrap_or(0),
                    parse_error_count: r["parse_error_count"].as_u64().unwrap_or(0),
                    timeout_count: r["timeout_count"].as_u64().unwrap_or(0),
                    api_error_count: r["api_error_count"].as_u64().unwrap_or(0),
                    last_success: r["last_success"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string()),
                    total_latency_ms: r["total_latency_ms"].as_u64().unwrap_or(0),
                    total_completion_tokens: r["total_completion_tokens"].as_u64().unwrap_or(0),
                    max_completion_tokens: r["max_completion_tokens"].as_u64().unwrap_or(0) as u32,
                    max_content_tokens: r["max_content_tokens"].as_u64().unwrap_or(0) as u32,
                    max_thinking_tokens: r["max_thinking_tokens"].as_u64().unwrap_or(0) as u32,
                },
            ))
        })
        .collect())
}
