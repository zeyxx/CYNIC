use super::crystals::row_to_crystal;
use crate::domain::ccm::{CANONICAL_CYCLES, Crystal, MIN_CRYSTALLIZATION_CYCLES};
use crate::domain::dog::{PHI_INV, PHI_INV2};
use crate::domain::storage::StorageError;
use crate::storage::{SurrealHttpStorage, escape_surreal, sanitize_id};

pub(super) async fn cleanup_ttl(storage: &SurrealHttpStorage) -> Result<(), StorageError> {
    storage
        .query_one("DELETE observation WHERE created_at < time::now() - 30d;")
        .await
        .map_err(|e| StorageError::QueryFailed(format!("TTL cleanup observation: {e}")))?;
    storage
        .query_one("DELETE mcp_audit WHERE ts < time::now() - 7d;")
        .await
        .map_err(|e| StorageError::QueryFailed(format!("TTL cleanup mcp_audit: {e}")))?;

    let dissolved = storage
        .query_one(
            "UPDATE crystal SET state = 'dissolved' \
             WHERE state = 'forming' AND updated_at < time::now() - 90d; \
             UPDATE crystal SET state = 'dissolved' \
             WHERE state = 'decaying' AND confidence < 0.1;",
        )
        .await;
    if let Err(e) = dissolved {
        tracing::warn!("crystal dissolution failed (non-fatal): {e}");
    }

    let purged = storage
        .query_one("DELETE crystal WHERE state = 'dissolved' AND updated_at < time::now() - 30d;")
        .await;
    if let Err(e) = purged {
        tracing::warn!("dissolved crystal purge failed (non-fatal): {e}");
    }

    Ok(())
}

pub(super) async fn last_integrity_hash(
    storage: &SurrealHttpStorage,
) -> Result<Option<String>, StorageError> {
    let rows = storage
        .query_one(
            "SELECT integrity_hash, created_at FROM verdict ORDER BY created_at DESC LIMIT 1;",
        )
        .await?;
    Ok(rows
        .first()
        .and_then(|r| r["integrity_hash"].as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string()))
}

pub(super) async fn list_crystals_missing_embedding(
    storage: &SurrealHttpStorage,
    limit: u32,
) -> Result<Vec<Crystal>, StorageError> {
    let sql = format!(
        "SELECT * FROM crystal WHERE embedding IS NONE OR embedding = [] LIMIT {}",
        limit.min(500),
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows.iter().map(row_to_crystal).collect())
}

pub(super) async fn count_verdicts(storage: &SurrealHttpStorage) -> Result<u64, StorageError> {
    let rows = storage
        .query_one("SELECT count() AS total FROM verdict GROUP ALL;")
        .await?;
    Ok(rows.first().and_then(|r| r["total"].as_u64()).unwrap_or(0))
}

pub(super) async fn count_crystal_observations(
    storage: &SurrealHttpStorage,
) -> Result<u64, StorageError> {
    let rows = storage
        .query_one("SELECT math::sum(observations) AS total FROM crystal GROUP ALL;")
        .await?;
    Ok(rows.first().and_then(|r| r["total"].as_u64()).unwrap_or(0))
}

pub(super) async fn consolidate_duplicate_crystals(
    storage: &SurrealHttpStorage,
) -> Result<u64, StorageError> {
    let groups_sql = "SELECT domain, content, \
                      array::group(meta::id(id)) AS ids, \
                      count() AS cnt \
                      FROM crystal \
                      GROUP BY domain, content";
    let all_groups = storage.query_one(groups_sql).await?;
    let groups: Vec<&serde_json::Value> = all_groups
        .iter()
        .filter(|g| g.get("cnt").and_then(|v| v.as_u64()).unwrap_or(0) > 1)
        .collect();

    let mut removed: u64 = 0;
    for group in &groups {
        let Some(ids) = group.get("ids").and_then(|v| v.as_array()) else {
            continue;
        };
        if ids.len() < 2 {
            continue;
        }

        let id_strs: Vec<&str> = ids.iter().filter_map(|v| v.as_str()).collect();
        if id_strs.is_empty() {
            continue;
        }
        let id_list: String = id_strs
            .iter()
            .map(|id| format!("crystal:`{id}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let detail_sql = format!(
            "SELECT meta::id(id) AS cid, observations, confidence, \
             contributing_verdicts, created_at \
             FROM [{id_list}] ORDER BY observations DESC"
        );
        let details = storage.query_one(&detail_sql).await?;
        if details.len() < 2 {
            continue;
        }

        let survivor_id = match details[0].get("cid").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };
        let mut total_obs: u64 = 0;
        let mut weighted_conf: f64 = 0.0;
        let mut all_verdicts: Vec<String> = Vec::new();
        let mut earliest_created = String::new();

        for detail in &details {
            let obs = detail
                .get("observations")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let conf = detail
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            total_obs += obs;
            weighted_conf += conf * obs as f64;
            if let Some(arr) = detail
                .get("contributing_verdicts")
                .and_then(|v| v.as_array())
            {
                for verdict in arr {
                    if let Some(s) = verdict.as_str() {
                        all_verdicts.push(s.to_string());
                    }
                }
            }
            if let Some(created_at) = detail.get("created_at").and_then(|v| v.as_str())
                && (earliest_created.is_empty() || created_at < earliest_created.as_str())
            {
                earliest_created = created_at.to_string();
            }
        }

        let merged_conf = if total_obs > 0 {
            weighted_conf / total_obs as f64
        } else {
            0.0
        };
        let new_state = if total_obs >= CANONICAL_CYCLES as u64 && merged_conf >= PHI_INV {
            "canonical"
        } else if total_obs >= MIN_CRYSTALLIZATION_CYCLES as u64 && merged_conf >= PHI_INV {
            "crystallized"
        } else if total_obs >= MIN_CRYSTALLIZATION_CYCLES as u64 && merged_conf < PHI_INV2 {
            "decaying"
        } else {
            "forming"
        };

        all_verdicts.sort();
        all_verdicts.dedup();
        let verdicts_arr: String = all_verdicts
            .iter()
            .map(|v| format!("'{}'", escape_surreal(v)))
            .collect::<Vec<_>>()
            .join(", ");

        let now = chrono::Utc::now().to_rfc3339();
        let safe_survivor = sanitize_id(&survivor_id)?;
        let safe_created = escape_surreal(&earliest_created);
        let safe_now = escape_surreal(&now);

        let update_sql = format!(
            "UPDATE crystal:`{safe_survivor}` SET \
             observations = {total_obs}, \
             confidence = {merged_conf}, \
             state = '{new_state}', \
             contributing_verdicts = [{verdicts_arr}], \
             created_at = '{safe_created}', \
             updated_at = '{safe_now}'"
        );
        storage.query_one(&update_sql).await?;

        let safe_domain =
            escape_surreal(group.get("domain").and_then(|v| v.as_str()).unwrap_or(""));
        let safe_content =
            escape_surreal(group.get("content").and_then(|v| v.as_str()).unwrap_or(""));
        let delete_sql = format!(
            "DELETE FROM crystal WHERE domain = '{safe_domain}' \
             AND content = '{safe_content}' \
             AND meta::id(id) != '{safe_survivor}' \
             RETURN BEFORE"
        );
        let deleted = storage.query_one(&delete_sql).await?;
        removed += deleted.len() as u64;
        tracing::info!(
            survivor = %safe_survivor,
            merged_obs = total_obs,
            merged_conf = %format!("{merged_conf:.3}"),
            state = new_state,
            duplicates_removed = id_strs.len() - 1,
            "crystal consolidation: merged duplicates"
        );
    }

    Ok(removed)
}
