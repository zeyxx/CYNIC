use super::{build_where_clause, safe_limit, sanitize_record_id};
use crate::domain::ccm::SessionSummary;
use crate::domain::compliance::SessionCompliance;
use crate::domain::storage::{
    Observation, ObservationFrequency, RawObservation, SessionTarget, StorageError,
};
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
        "CREATE observation SET \
            project = '{project}', \
            agent_id = '{agent_id}', \
            tool = '{tool}', \
            target = '{target}', \
            domain = '{domain}', \
            status = '{status}', \
            context = '{context}', \
            session_id = '{session_id}', \
            tags = {tags}, \
            created_at = time::now();",
        project = escape_surreal(&obs.project),
        agent_id = escape_surreal(&obs.agent_id),
        tool = escape_surreal(&obs.tool),
        target = escape_surreal(&obs.target),
        domain = escape_surreal(&obs.domain),
        status = escape_surreal(&obs.status),
        context = escape_surreal(&obs.context),
        session_id = escape_surreal(&obs.session_id),
        tags = tags_sql,
    );
    storage.query(&sql).await?;
    Ok(())
}

pub(super) async fn query_observations(
    storage: &SurrealHttpStorage,
    project: &str,
    domain: Option<&str>,
    limit: u32,
) -> Result<Vec<ObservationFrequency>, StorageError> {
    let domain_clause = match domain {
        Some(d) => format!(" AND domain = '{}'", escape_surreal(d)),
        None => String::new(),
    };
    let sql = format!(
        "SELECT target, tool, count() AS freq FROM observation \
         WHERE project = '{}'{} \
         GROUP BY target, tool ORDER BY freq DESC LIMIT {};",
        escape_surreal(project),
        domain_clause,
        safe_limit(limit),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows
        .iter()
        .map(|r| ObservationFrequency {
            target: r["target"].as_str().unwrap_or("").to_string(),
            tool: r["tool"].as_str().unwrap_or("").to_string(),
            freq: r["freq"].as_u64().unwrap_or(0),
        })
        .collect())
}

pub(super) async fn query_session_targets(
    storage: &SurrealHttpStorage,
    project: &str,
    limit: u32,
) -> Result<Vec<SessionTarget>, StorageError> {
    let sql = format!(
        "SELECT agent_id AS session_id, target FROM observation \
         WHERE project = '{}' AND agent_id != '' AND agent_id != 'unknown' \
         AND tool IN ['Edit', 'Write', 'Read'] \
         ORDER BY agent_id, target LIMIT {};",
        escape_surreal(project),
        limit.min(1000),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows
        .iter()
        .map(|r| SessionTarget {
            session_id: r["session_id"].as_str().unwrap_or("").to_string(),
            target: r["target"].as_str().unwrap_or("").to_string(),
        })
        .collect())
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
    let sql = format!(
        "SELECT agent_id, count() AS obs_count \
         FROM observation \
         WHERE agent_id != '' AND agent_id != 'unknown' \
         AND agent_id NOT IN (SELECT VALUE session_id FROM session_summary) \
         GROUP BY agent_id \
         ORDER BY obs_count DESC \
         LIMIT {};",
        safe_limit(limit),
    );
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
