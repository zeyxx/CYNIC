use super::{build_where_clause, safe_limit, sanitize_record_id};
use crate::domain::ccm::SessionSummary;
use crate::domain::compliance::SessionCompliance;
use crate::domain::storage::{Event, Observation, RawEvent, RawObservation, StorageError};
use crate::storage::{SurrealHttpStorage, escape_surreal};

fn row_to_raw_observation(row: &serde_json::Value) -> RawObservation {
    let tags = row["tags"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let depends_on = row["depends_on"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let observers = row["observers"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    RawObservation {
        id: row["id"].as_str().unwrap_or("").to_string(),
        tool: row["tool"].as_str().unwrap_or("").to_string(),
        target: row["target"].as_str().unwrap_or("").to_string(),
        domain: row["domain"].as_str().unwrap_or("").to_string(),
        status: row["status"].as_str().unwrap_or("").to_string(),
        context: row["context"].as_str().unwrap_or("").to_string(),
        created_at: row["created_at"].as_str().unwrap_or("").to_string(),
        project: row["project"].as_str().unwrap_or("").to_string(),
        agent_id: row["agent_id"].as_str().unwrap_or("").to_string(),
        session_id: row["session_id"].as_str().unwrap_or("").to_string(),
        tags,
        value: if row["value"].is_null() {
            None
        } else {
            Some(row["value"].clone())
        },
        confidence: row["confidence"].as_str().map(String::from),
        consumer: row["consumer"].as_str().map(String::from),
        action: row["action"].as_str().map(String::from),
        depends_on,
        maturity: row["maturity"].as_f64(),
        hash: row["hash"].as_str().unwrap_or("").to_string(),
        prev_hash: row["prev_hash"].as_str().unwrap_or("").to_string(),
        observers,
        consensus_score: row["consensus_score"].as_f64(),
        source_tier: row["source_tier"].as_str().unwrap_or("").to_string(),
    }
}

pub(super) async fn store_observation(
    storage: &SurrealHttpStorage,
    obs: &Observation,
) -> Result<(), StorageError> {
    let tags_sql = if obs.tags.is_empty() {
        "[]".to_string()
    } else {
        let escaped: Vec<String> = obs
            .tags
            .iter()
            .map(|t| format!("'{}'", escape_surreal(t)))
            .collect();
        format!("[{}]", escaped.join(", "))
    };

    let sql = format!(
        "CREATE observation SET project = '{}', agent_id = '{}', tool = '{}', target = '{}', \
         domain = '{}', status = '{}', context = '{}', session_id = '{}', timestamp = '{}', \
         tags = {}, source_tier = '{}', created_at = time::now();",
        escape_surreal(&obs.project),
        escape_surreal(&obs.agent_id),
        escape_surreal(&obs.tool),
        escape_surreal(&obs.target),
        escape_surreal(&obs.domain),
        escape_surreal(&obs.status),
        escape_surreal(&obs.context),
        escape_surreal(&obs.session_id),
        escape_surreal(&obs.timestamp),
        tags_sql,
        escape_surreal(&obs.source_tier),
    );
    storage.query(&sql).await?;
    Ok(())
}

pub(super) async fn store_session_summary(
    storage: &SurrealHttpStorage,
    summary: &SessionSummary,
) -> Result<(), StorageError> {
    let safe_key = sanitize_record_id(&summary.session_id);
    let sql = format!(
        "UPSERT session_summary:`{key}` SET \
            session_id = '{session_id}', \
            agent_id = '{agent_id}', \
            summary = '{summary}', \
            observations_count = {obs_count}, \
            created_at = time::now();",
        key = safe_key,
        session_id = escape_surreal(&summary.session_id),
        agent_id = escape_surreal(&summary.agent_id),
        summary = escape_surreal(&summary.summary),
        obs_count = summary.observations_count,
    );
    storage.query_one(&sql).await?;
    Ok(())
}

pub(super) async fn list_session_summaries(
    storage: &SurrealHttpStorage,
    limit: u32,
) -> Result<Vec<SessionSummary>, StorageError> {
    let sql = format!(
        "SELECT * FROM session_summary ORDER BY created_at DESC LIMIT {}",
        safe_limit(limit),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows
        .iter()
        .map(|row| SessionSummary {
            session_id: row["session_id"].as_str().unwrap_or("").to_string(),
            agent_id: row["agent_id"].as_str().unwrap_or("").to_string(),
            summary: row["summary"].as_str().unwrap_or("").to_string(),
            observations_count: row["observations_count"].as_u64().unwrap_or(0) as u32,
            created_at: row["created_at"].as_str().unwrap_or("").to_string(),
        })
        .collect())
}

pub(super) async fn get_unsummarized_sessions(
    storage: &SurrealHttpStorage,
    min_observations: u32,
    limit: u32,
) -> Result<Vec<(String, String, u32)>, StorageError> {
    // Bound to 7d window — observations older than that are not worth summarizing,
    // and the time filter uses obs_created_idx to avoid full-table scan (was 8.9s unbounded).
    // LET pre-binding: SurrealDB query planner uses indexes on LET-bound subqueries
    // but not on inline subqueries (issue #5439).
    let sql = format!(
        "LET $summarized = (SELECT VALUE session_id FROM session_summary); \
         SELECT agent_id, count() AS obs_count \
         FROM observation \
         WHERE agent_id != '' AND agent_id != 'unknown' \
         AND created_at > time::now() - 7d \
         AND agent_id NOT IN $summarized \
         GROUP BY agent_id \
         ORDER BY obs_count DESC \
         LIMIT {};",
        safe_limit(limit),
    );
    // query_one pops the last result set (the SELECT), skipping the LET result
    let rows = storage.query_one(&sql).await?;
    Ok(rows
        .iter()
        .filter_map(|row| {
            let agent_id = row["agent_id"].as_str().unwrap_or("").to_string();
            let count = row["obs_count"].as_u64().unwrap_or(0) as u32;
            if count >= min_observations {
                Some((agent_id.clone(), agent_id, count))
            } else {
                None
            }
        })
        .collect())
}

pub(super) async fn get_session_observations(
    storage: &SurrealHttpStorage,
    session_id: &str,
) -> Result<Vec<RawObservation>, StorageError> {
    let sql = format!(
        "SELECT tool, target, domain, status, context, created_at FROM observation \
         WHERE agent_id = '{}' ORDER BY created_at ASC LIMIT 50;",
        escape_surreal(session_id),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows.iter().map(row_to_raw_observation).collect())
}

pub(super) async fn store_session_compliance(
    storage: &SurrealHttpStorage,
    compliance: &SessionCompliance,
) -> Result<(), StorageError> {
    let safe_key = sanitize_record_id(&compliance.agent_id);
    let warnings_json = serde_json::to_string(&compliance.warnings).unwrap_or_else(|_| "[]".into());
    let sql = format!(
        "UPSERT session_compliance:`{key}` SET \
            session_id = '{session_id}', \
            agent_id = '{agent_id}', \
            score = {score}, \
            warnings = {warnings}, \
            read_before_edit = {rbe}, \
            bash_retry_violations = {brv}, \
            files_modified = {fm}, \
            created_at = time::now();",
        key = safe_key,
        session_id = escape_surreal(&compliance.session_id),
        agent_id = escape_surreal(&compliance.agent_id),
        score = compliance.score,
        warnings = warnings_json,
        rbe = compliance.read_before_edit,
        brv = compliance.bash_retry_violations,
        fm = compliance.files_modified,
    );
    storage.query_one(&sql).await?;
    Ok(())
}

pub(super) async fn list_session_compliance(
    storage: &SurrealHttpStorage,
    limit: u32,
) -> Result<Vec<SessionCompliance>, StorageError> {
    let sql = format!(
        "SELECT * FROM session_compliance ORDER BY created_at DESC LIMIT {};",
        safe_limit(limit),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows
        .iter()
        .map(|row| SessionCompliance {
            session_id: row["session_id"].as_str().unwrap_or("").to_string(),
            agent_id: row["agent_id"].as_str().unwrap_or("").to_string(),
            score: row["score"].as_f64().unwrap_or(0.0),
            warnings: row["warnings"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            read_before_edit: row["read_before_edit"].as_f64().unwrap_or(0.0),
            bash_retry_violations: row["bash_retry_violations"].as_u64().unwrap_or(0) as u32,
            files_modified: row["files_modified"].as_u64().unwrap_or(0) as u32,
            created_at: row["created_at"].as_str().unwrap_or("").to_string(),
        })
        .collect())
}

pub(super) async fn list_observations_raw(
    storage: &SurrealHttpStorage,
    domain: Option<&str>,
    agent_id: Option<&str>,
    limit: u32,
) -> Result<Vec<RawObservation>, StorageError> {
    let mut conditions = Vec::new();
    if let Some(d) = domain {
        conditions.push(format!("domain = '{}'", escape_surreal(d)));
    }
    if let Some(a) = agent_id {
        conditions.push(format!("agent_id = '{}'", escape_surreal(a)));
    }
    let where_clause = build_where_clause(&conditions);
    let sql = format!(
        "SELECT * FROM observation{where_clause} ORDER BY created_at DESC LIMIT {}",
        safe_limit(limit),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows.iter().map(row_to_raw_observation).collect())
}

/// List observations filtered by domain + target — efficient single-target lookup.
pub(super) async fn list_observations_by_target(
    storage: &SurrealHttpStorage,
    domain: &str,
    target: &str,
    limit: u32,
) -> Result<Vec<RawObservation>, StorageError> {
    let domain_esc = escape_surreal(domain);
    let target_esc = escape_surreal(target);
    let sql = format!(
        "SELECT * FROM observation WHERE domain = '{domain_esc}' AND target = '{target_esc}' ORDER BY created_at DESC LIMIT {}",
        safe_limit(limit),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows.iter().map(row_to_raw_observation).collect())
}

/// List observations filtered by domain + tag — uses CONTAINS on tags array.
pub(super) async fn list_observations_by_tag(
    storage: &SurrealHttpStorage,
    domain: &str,
    tag: &str,
    limit: u32,
) -> Result<Vec<RawObservation>, StorageError> {
    let domain_esc = escape_surreal(domain);
    let tag_esc = escape_surreal(tag);
    let sql = format!(
        "SELECT * FROM observation WHERE domain = '{domain_esc}' AND tags CONTAINS '{tag_esc}' ORDER BY created_at DESC LIMIT {}",
        safe_limit(limit),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows.iter().map(row_to_raw_observation).collect())
}

/// Last observation per source — GROUP BY agent_id, return last timestamp + count.
/// Uses array::max instead of math::max (created_at is stored as string, not datetime).
pub(super) async fn last_observation_per_source(
    storage: &SurrealHttpStorage,
) -> Result<Vec<(String, String, u64)>, StorageError> {
    let sql = "SELECT agent_id, array::max(array::group(created_at)) AS last_at, count() AS total \
               FROM observation WHERE agent_id != '' AND (source_tier = 'permanent' OR source_tier = 'cron') \
               GROUP BY agent_id ORDER BY total DESC;";
    let rows = storage
        .query_one(sql)
        .await
        .map_err(|e| StorageError::QueryFailed(format!("last_observation_per_source: {e}")))?;
    Ok(rows
        .iter()
        .filter_map(|r| {
            let agent_id = r["agent_id"].as_str().unwrap_or("").to_string();
            if agent_id.is_empty() {
                return None;
            }
            let last_at = r["last_at"].as_str().unwrap_or("").to_string();
            let total = r["total"].as_u64().unwrap_or(0);
            Some((agent_id, last_at, total))
        })
        .collect())
}

// ── EVENT STORAGE ────────────────────────────────────

fn row_to_raw_event(row: &serde_json::Value) -> RawEvent {
    RawEvent {
        id: row["id"].as_str().unwrap_or("").to_string(),
        tool: row["tool"].as_str().unwrap_or("").to_string(),
        node: row["node"].as_str().unwrap_or("").to_string(),
        elapsed_ms: row["elapsed_ms"].as_u64().unwrap_or(0),
        output_bytes: row["output_bytes"].as_u64().unwrap_or(0),
        success: row["success"].as_bool().unwrap_or(false),
        metadata: row["metadata"].as_str().unwrap_or("").to_string(),
        agent_id: row["agent_id"].as_str().unwrap_or("").to_string(),
        created_at: row["created_at"].as_str().unwrap_or("").to_string(),
        failure_reason: row["failure_reason"].as_str().unwrap_or("").to_string(),
    }
}

/// Store infrastructure event: node latency, output size, success/fail.
/// Fire-and-forget, similar to store_observation.
pub(super) async fn store_event(
    storage: &SurrealHttpStorage,
    event: &Event,
) -> Result<(), StorageError> {
    let sql = format!(
        "CREATE event SET \
            tool = '{tool}', \
            node = '{node}', \
            elapsed_ms = {elapsed_ms}, \
            output_bytes = {output_bytes}, \
            success = {success}, \
            metadata = '{metadata}', \
            failure_reason = '{failure_reason}', \
            agent_id = '{agent_id}', \
            created_at = time::now();",
        tool = escape_surreal(&event.tool),
        node = escape_surreal(&event.node),
        elapsed_ms = event.elapsed_ms,
        output_bytes = event.output_bytes,
        success = event.success,
        metadata = escape_surreal(&event.metadata),
        failure_reason = escape_surreal(&event.failure_reason),
        agent_id = escape_surreal(&event.agent_id),
    );
    storage.query(&sql).await?;
    Ok(())
}

/// Fleet stats: aggregate latencies per node over a time window.
/// Returns: (node, avg_latency_ms, success_rate, last_seen_secs, failure_reason).
/// Used by inference router to select best node and degrade based on failure_reason.
pub(super) async fn fleet_stats(
    storage: &SurrealHttpStorage,
    window_secs: u64,
    limit: u32,
) -> Result<Vec<(String, u64, f64, u64, String)>, StorageError> {
    // Aggregate stats per node: avg latency, success rate.
    let sql = format!(
        "SELECT node, \
                math::mean(elapsed_ms) AS avg_latency, \
                count(success == true) / count() AS success_rate \
         FROM event \
         WHERE created_at > time::now() - {}s \
         GROUP BY node \
         ORDER BY avg_latency ASC \
         LIMIT {};",
        window_secs,
        safe_limit(limit),
    );
    let rows = storage
        .query_one(&sql)
        .await
        .map_err(|e| StorageError::QueryFailed(format!("fleet_stats (agg): {e}")))?;

    // Get most recent failure_reason per node (separate query to avoid SurrealDB ORDER BY subquery issues)
    let sql_reasons = format!(
        "SELECT node, failure_reason, created_at FROM event \
         WHERE created_at > time::now() - {window_secs}s \
         ORDER BY created_at DESC;"
    );
    let reason_rows = storage
        .query_one(&sql_reasons)
        .await
        .map_err(|e| StorageError::QueryFailed(format!("fleet_stats (reasons): {e}")))?;

    // Build a map of node → most recent failure_reason
    let mut latest_reason: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for row in reason_rows {
        if let Some(node) = row["node"].as_str() {
            let reason = row["failure_reason"].as_str().unwrap_or("unknown");
            // Only keep the first (most recent) reason for each node due to ORDER BY DESC
            if !latest_reason.contains_key(node) {
                latest_reason.insert(node.to_string(), reason.to_string());
            }
        }
    }

    Ok(rows
        .iter()
        .filter_map(|r| {
            let node = r["node"].as_str()?.to_string();
            let avg_latency = r["avg_latency"].as_f64().unwrap_or(0.0) as u64;
            let success_rate = r["success_rate"].as_f64().unwrap_or(0.0);
            let failure_reason = latest_reason
                .get(&node)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());

            // Since events matched the WHERE clause (created_at > now - window_secs),
            // we know they're recent. For now, report age as 0 (very fresh).
            let age_secs = 0u64;

            Some((node, avg_latency, success_rate, age_secs, failure_reason))
        })
        .collect())
}

/// List recent events, optionally filtered by node or tool.
pub(super) async fn list_events(
    storage: &SurrealHttpStorage,
    node: Option<&str>,
    tool: Option<&str>,
    limit: u32,
) -> Result<Vec<RawEvent>, StorageError> {
    let mut conditions = Vec::new();
    if let Some(n) = node {
        conditions.push(format!("node = '{}'", escape_surreal(n)));
    }
    if let Some(t) = tool {
        conditions.push(format!("tool = '{}'", escape_surreal(t)));
    }
    let where_clause = build_where_clause(&conditions);
    let sql = format!(
        "SELECT * FROM event{where_clause} ORDER BY created_at DESC LIMIT {}",
        safe_limit(limit),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows.iter().map(row_to_raw_event).collect())
}

/// Detect nodes with persistent fatal failures (process_crash, not_started).
/// Returns (node, failure_reason, recent_failure_count, last_fatal_seen_secs).
/// Used by auto-recovery system to trigger restarts.
pub(super) async fn list_degraded_nodes(
    storage: &SurrealHttpStorage,
    window_secs: u64,
    fatal_threshold: f64, // e.g., 0.8 = 80% of recent probes show fatal failure
) -> Result<Vec<(String, String, u64, u64)>, StorageError> {
    // K15: Identify nodes with persistent fatal failures.
    // Two-pass approach: (1) fatal counts, (2) total counts, compute ratio client-side.
    // This avoids SurrealDB SQL parsing issues with complex aggregations.
    let sql = format!(
        "SELECT node, failure_reason, count() AS fatal_count \
         FROM event \
         WHERE created_at > time::now() - {window_secs}s AND failure_reason IN ['process_crash', 'not_started'] \
         GROUP BY node, failure_reason;"
    );
    let fatal_rows = storage
        .query_one(&sql)
        .await
        .map_err(|e| StorageError::QueryFailed(format!("list_degraded_nodes (fatal): {e}")))?;

    // Get total counts per node in the same window
    let sql_total = format!(
        "SELECT node, count() AS total_count \
         FROM event \
         WHERE created_at > time::now() - {window_secs}s \
         GROUP BY node;"
    );
    let total_rows = storage
        .query_one(&sql_total)
        .await
        .map_err(|e| StorageError::QueryFailed(format!("list_degraded_nodes (total): {e}")))?;

    // Build a map of node → total_count for quick lookup
    let mut totals: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for row in total_rows {
        if let Some(node) = row["node"].as_str() {
            let count = row["total_count"].as_u64().unwrap_or(1);
            totals.insert(node.to_string(), count);
        }
    }

    // Filter: nodes where fatal_count/total_count > threshold
    Ok(fatal_rows
        .iter()
        .filter_map(|r| {
            let node = r["node"].as_str()?.to_string();
            let failure_reason = r["failure_reason"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let fatal_count = r["fatal_count"].as_u64().unwrap_or(0);
            let total = totals.get(&node).copied().unwrap_or(1);
            let ratio = fatal_count as f64 / total as f64;
            if ratio > fatal_threshold {
                Some((node, failure_reason, fatal_count, 0u64))
            } else {
                None
            }
        })
        .collect())
}

/// Query observation activity in a zone's path prefixes.
/// Returns agents (excluding `exclude_agent`) active in the last hour,
/// ordered by most recent activity.
pub(crate) async fn zone_activity(
    storage: &SurrealHttpStorage,
    path_prefixes: &[String],
    exclude_agent: &str,
    project_root: &str,
) -> Result<Vec<crate::api::rest::dispatch::AgentActivity>, StorageError> {
    if path_prefixes.is_empty() {
        return Ok(vec![]);
    }

    // Build CONTAINS conditions for path prefixes.
    // Observations store full paths (/home/user/Bureau/CYNIC/scripts/foo.sh)
    // Zone prefixes are relative (scripts/). Prepend project_root for matching.
    let conditions: Vec<String> = path_prefixes
        .iter()
        .map(|p| {
            let full = format!("{project_root}/{p}");
            format!("string::starts_with(target, '{}')", escape_surreal(&full))
        })
        .collect();
    let path_filter = conditions.join(" OR ");

    let exclude = escape_surreal(exclude_agent);

    // Fetch recent edits in zone, ordered by time desc. No GROUP BY (SurrealDB 3.x
    // math::max doesn't work on datetime strings). Deduplicate by agent_id in Rust.
    let query = format!(
        "SELECT agent_id, created_at AS last_active, target AS last_file \
         FROM observation \
         WHERE ({path_filter}) \
           AND agent_id != '{exclude}' \
           AND agent_id != '' \
           AND tool IN ['Edit', 'Write'] \
           AND target != '' \
           AND created_at > time::now() - 1h \
         ORDER BY created_at DESC \
         LIMIT 50;"
    );

    let results = storage.query(&query).await?;
    let rows = results.into_iter().next().unwrap_or_default();

    // Deduplicate: first occurrence per agent_id = most recent (query ordered DESC)
    let mut seen = std::collections::HashSet::new();
    let mut agent_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

    for row in &rows {
        if let Some(aid) = row["agent_id"].as_str() {
            *agent_counts.entry(aid.to_string()).or_default() += 1;
        }
    }

    let agents = rows
        .iter()
        .filter_map(|row| {
            let agent_id = row["agent_id"].as_str()?.to_string();
            if !seen.insert(agent_id.clone()) {
                return None; // already seen — earlier row was more recent
            }
            let last_active = row["last_active"].as_str().unwrap_or("").to_string();
            let last_file = row["last_file"].as_str().unwrap_or("").to_string();
            let activity_count = agent_counts.get(&agent_id).copied().unwrap_or(1);

            Some(crate::api::rest::dispatch::AgentActivity {
                agent_id,
                last_active,
                last_file,
                activity_count,
            })
        })
        .collect();

    Ok(agents)
}
