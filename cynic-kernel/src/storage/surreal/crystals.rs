use super::{SurrealHttpStorage, build_where_clause};
use crate::domain::ccm::{CANONICAL_CYCLES, Crystal, CrystalState, MIN_CRYSTALLIZATION_CYCLES};
use crate::domain::dog::{MIN_QUORUM, PHI_INV, PHI_INV2};
use crate::domain::storage::StorageError;
use crate::storage::{escape_surreal, safe_limit, sanitize_id};

pub(super) async fn store_crystal(
    storage: &SurrealHttpStorage,
    crystal: &Crystal,
) -> Result<(), StorageError> {
    let escape = |s: &str| escape_surreal(s);
    let safe_id = sanitize_id(&crystal.id)?;
    let sql = format!(
        "UPSERT crystal:`{}` SET content = '{}', domain = '{}', confidence = {}, observations = {}, state = '{}', created_at = '{}', updated_at = '{}'",
        safe_id,
        escape(&crystal.content),
        escape(&crystal.domain),
        crystal.confidence,
        crystal.observations,
        crystal.state,
        escape(&crystal.created_at),
        escape(&crystal.updated_at)
    );
    storage.query_one(&sql).await?;
    Ok(())
}

pub(super) async fn get_crystal(
    storage: &SurrealHttpStorage,
    id: &str,
) -> Result<Option<Crystal>, StorageError> {
    let id = sanitize_id(id)?;
    let sql = format!("SELECT * FROM crystal:`{id}`");
    let rows = storage.query_one(&sql).await?;
    Ok(rows.first().map(row_to_crystal))
}

pub(super) async fn list_crystals(
    storage: &SurrealHttpStorage,
    limit: u32,
) -> Result<Vec<Crystal>, StorageError> {
    let sql = format!(
        "SELECT * FROM crystal ORDER BY state ASC, confidence DESC LIMIT {}",
        safe_limit(limit),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows.iter().map(row_to_crystal).collect())
}

pub(super) async fn list_crystals_filtered(
    storage: &SurrealHttpStorage,
    limit: u32,
    domain: Option<&str>,
    state: Option<&str>,
) -> Result<Vec<Crystal>, StorageError> {
    let mut conditions = Vec::new();
    if let Some(d) = domain {
        conditions.push(format!("domain = '{}'", escape_surreal(d)));
    }
    if let Some(s) = state {
        conditions.push(format!("state = '{}'", escape_surreal(s)));
    }
    let where_clause = build_where_clause(&conditions);
    let sql = format!(
        "SELECT * FROM crystal{where_clause} ORDER BY state ASC, confidence DESC LIMIT {}",
        safe_limit(limit),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows.iter().map(row_to_crystal).collect())
}

pub(super) async fn delete_crystal(
    storage: &SurrealHttpStorage,
    id: &str,
) -> Result<(), StorageError> {
    let safe_id = sanitize_id(id)?;
    let sql = format!("DELETE crystal:`{safe_id}`");
    storage.query_one(&sql).await?;
    Ok(())
}

pub(super) async fn list_crystals_for_domain(
    storage: &SurrealHttpStorage,
    domain: &str,
    limit: u32,
) -> Result<Vec<Crystal>, StorageError> {
    let safe_domain = escape_surreal(domain);
    let sql = format!(
        "SELECT * FROM crystal \
         WHERE (domain = '{domain}' OR domain = 'general') \
         AND (state = 'crystallized' OR state = 'canonical') \
         ORDER BY confidence DESC \
         LIMIT {limit}",
        domain = safe_domain,
        limit = safe_limit(limit),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows.iter().map(row_to_crystal).collect())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn observe_crystal(
    storage: &SurrealHttpStorage,
    id: &str,
    content: &str,
    domain: &str,
    score: f64,
    timestamp: &str,
    voter_count: usize,
    verdict_id: &str,
) -> Result<(), StorageError> {
    if voter_count < MIN_QUORUM {
        return Err(StorageError::QueryFailed(format!(
            "quorum not met: voter_count={voter_count}, min={MIN_QUORUM}",
        )));
    }

    let safe_id = sanitize_id(id)?;
    let sanitized_content = crate::domain::sanitize::sanitize_crystal_content(content);
    let score = if score.is_finite() { score } else { 0.0 };

    let sql = format!(
        "BEGIN TRANSACTION; \
         LET $prev_obs = (SELECT VALUE observations FROM crystal:`{id}`)[0] ?? 0; \
         LET $prev_conf = (SELECT VALUE confidence FROM crystal:`{id}`)[0] ?? 0.0; \
         LET $new_obs = $prev_obs + 1; \
         LET $new_conf = IF $prev_obs > 0 THEN ($prev_conf * $prev_obs + {score}) / $new_obs ELSE {score} END; \
         LET $new_state = IF $new_obs >= {t_canon} AND $new_conf >= {c_high} THEN 'canonical' \
             ELSE IF $new_obs >= {t_cryst} AND $new_conf >= {c_high} THEN 'crystallized' \
             ELSE IF $new_obs >= {t_cryst} AND $new_conf < {c_low} THEN 'decaying' \
             ELSE 'forming' END; \
         UPSERT crystal:`{id}` SET \
             content = content ?? '{content}', \
             domain = domain ?? '{domain}', \
             observations = $new_obs, \
             confidence = $new_conf, \
             state = $new_state, \
             contributing_verdicts = IF array::len(contributing_verdicts ?? []) < 500 THEN array::union(contributing_verdicts ?? [], ['{vid}']) ELSE contributing_verdicts ?? [] END, \
             created_at = created_at ?? '{ts}', \
             updated_at = '{ts}'; \
         COMMIT TRANSACTION;",
        id = safe_id,
        content = escape_surreal(&sanitized_content),
        domain = escape_surreal(domain),
        vid = escape_surreal(verdict_id),
        score = score,
        t_canon = CANONICAL_CYCLES,
        t_cryst = MIN_CRYSTALLIZATION_CYCLES,
        c_high = PHI_INV,
        c_low = PHI_INV2,
        ts = escape_surreal(timestamp),
    );
    storage.query(&sql).await?;
    Ok(())
}

pub(super) async fn store_crystal_embedding(
    storage: &SurrealHttpStorage,
    id: &str,
    embedding: &[f32],
) -> Result<(), StorageError> {
    let safe_id = sanitize_id(id)?;
    let vec_str = embedding_to_surreal_literal(embedding);
    let sql = format!("UPDATE crystal:`{safe_id}` SET embedding = [{vec_str}]");
    storage.query_one(&sql).await?;
    Ok(())
}

pub(super) async fn search_crystals_semantic(
    storage: &SurrealHttpStorage,
    query_embedding: &[f32],
    limit: u32,
) -> Result<Vec<Crystal>, StorageError> {
    let k = safe_limit(limit).min(20);
    let vec_str = embedding_to_surreal_literal(query_embedding);
    let sql = format!(
        "LET $q = [{vec_str}]; \
         SELECT *, vector::similarity::cosine(embedding, $q) AS similarity \
         FROM crystal \
         WHERE embedding <|{k},40|> $q \
         AND (state = 'crystallized' OR state = 'canonical') \
         ORDER BY similarity DESC;",
    );
    let results = storage.query(&sql).await?;
    let rows = results.into_iter().nth(1).unwrap_or_default();
    Ok(rows.iter().map(row_to_crystal).collect())
}

pub(super) async fn find_similar_crystal(
    storage: &SurrealHttpStorage,
    embedding: &[f32],
    domain: &str,
    threshold: f64,
) -> Result<Option<(String, f64)>, StorageError> {
    let safe_domain = escape_surreal(domain);
    let vec_str = embedding_to_surreal_literal(embedding);
    let sql = format!(
        "LET $q = [{vec_str}]; \
         SELECT meta::id(id) AS crystal_id, vector::similarity::cosine(embedding, $q) AS similarity \
         FROM crystal \
         WHERE embedding <|5,40|> $q \
         AND domain = '{safe_domain}' \
         ORDER BY similarity DESC \
         LIMIT 1;",
    );
    let results = storage.query(&sql).await?;
    let rows = results.into_iter().nth(1).unwrap_or_default();
    if let Some(row) = rows.first() {
        let sim = row
            .get("similarity")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if sim >= threshold {
            let id = row
                .get("crystal_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !id.is_empty() {
                return Ok(Some((id, sim)));
            }
        }
    }
    Ok(None)
}

pub(super) fn row_to_crystal(row: &serde_json::Value) -> Crystal {
    let state: CrystalState = row["state"]
        .as_str()
        .unwrap_or("forming")
        .parse()
        .unwrap_or(CrystalState::Forming);
    let raw_id = row["id"].as_str().unwrap_or("");
    let id = raw_id
        .strip_prefix("crystal:")
        .unwrap_or(raw_id)
        .trim_matches('`')
        .to_string();
    let contributing_verdicts = row["contributing_verdicts"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Crystal {
        id,
        content: row["content"].as_str().unwrap_or("").to_string(),
        domain: row["domain"].as_str().unwrap_or("").to_string(),
        confidence: row["confidence"].as_f64().unwrap_or(0.0),
        observations: row["observations"].as_u64().unwrap_or(0) as u32,
        state,
        created_at: row["created_at"].as_str().unwrap_or("").to_string(),
        updated_at: row["updated_at"].as_str().unwrap_or("").to_string(),
        contributing_verdicts,
    }
}

fn embedding_to_surreal_literal(embedding: &[f32]) -> String {
    embedding
        .iter()
        .map(|v| if v.is_finite() { *v } else { 0.0f32 })
        .map(|v| format!("{v}"))
        .collect::<Vec<_>>()
        .join(",")
}
