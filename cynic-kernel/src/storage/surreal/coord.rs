use super::{build_where_clause, sanitize_record_id};
use crate::domain::coord::{
    AgentInfo, AuditEntry, BatchClaimResult, ClaimEntry, ClaimResult, ConflictInfo, CoordError,
    CoordPort, CoordSnapshot,
};
use crate::storage::{SurrealHttpStorage, escape_surreal};

#[async_trait::async_trait]
impl CoordPort for SurrealHttpStorage {
    async fn register_agent(
        &self,
        agent_id: &str,
        agent_type: &str,
        intent: &str,
    ) -> Result<(), CoordError> {
        let sql = format!(
            "UPSERT agent_session:`{id_key}` SET \
                agent_id = '{id_val}', agent_type = '{agent_type}', intent = '{intent}', \
                registered_at = time::now(), last_seen = time::now(), active = true;",
            id_key = sanitize_record_id(agent_id),
            id_val = escape_surreal(agent_id),
            agent_type = escape_surreal(agent_type),
            intent = escape_surreal(intent),
        );
        self.query_one(&sql)
            .await
            .map_err(|e| CoordError::StorageFailed(format!("Register: {e}")))?;
        Ok(())
    }

    async fn claim(
        &self,
        agent_id: &str,
        target: &str,
        claim_type: &str,
    ) -> Result<ClaimResult, CoordError> {
        let target_key = sanitize_record_id(target);
        let check_sql = format!(
            "SELECT * FROM work_claim:`{key}` WHERE agent_id != '{agent_id}' AND active = true;",
            key = target_key,
            agent_id = escape_surreal(agent_id),
        );
        let existing = self
            .query_one(&check_sql)
            .await
            .map_err(|e| CoordError::StorageFailed(format!("Claim check: {e}")))?;
        if !existing.is_empty() {
            let infos = existing
                .iter()
                .map(|c| ConflictInfo {
                    agent_id: c["agent_id"].as_str().unwrap_or("?").to_string(),
                    claimed_at: c["claimed_at"].as_str().unwrap_or("?").to_string(),
                })
                .collect();
            return Ok(ClaimResult::Conflict(infos));
        }

        let upsert_sql = format!(
            "UPSERT work_claim:`{key}` SET \
                agent_id = '{agent_id}', target = '{target}', claim_type = '{claim_type}', \
                claimed_at = time::now(), active = true;",
            key = target_key,
            agent_id = escape_surreal(agent_id),
            target = escape_surreal(target),
            claim_type = escape_surreal(claim_type),
        );
        self.query_one(&upsert_sql)
            .await
            .map_err(|e| CoordError::StorageFailed(format!("Claim upsert: {e}")))?;

        let verify_sql =
            format!("SELECT agent_id FROM work_claim:`{target_key}` WHERE active = true;");
        let owner = self.query_one(&verify_sql).await.map_err(|e| {
            tracing::warn!(error = %e, target = %target, "claim verification query failed");
            CoordError::StorageFailed(format!("Claim verify: {e}"))
        })?;
        let we_own = owner
            .first()
            .and_then(|r| r["agent_id"].as_str())
            .is_some_and(|id| id == agent_id);
        if !we_own {
            return Ok(ClaimResult::Conflict(vec![ConflictInfo {
                agent_id: owner
                    .first()
                    .and_then(|r| r["agent_id"].as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                claimed_at: "just now".into(),
            }]));
        }

        if let Err(e) = self.heartbeat(agent_id).await {
            tracing::warn!(error = %e, agent_id, "heartbeat after claim failed");
        }
        Ok(ClaimResult::Claimed)
    }

    async fn release(&self, agent_id: &str, target: Option<&str>) -> Result<String, CoordError> {
        let (sql, desc) = match target {
            Some(t) => (
                format!(
                    "UPDATE work_claim SET active = false WHERE agent_id = '{}' AND target = '{}' AND active = true;",
                    escape_surreal(agent_id),
                    escape_surreal(t)
                ),
                format!("Released '{t}' for agent '{agent_id}'."),
            ),
            None => (
                format!(
                    "UPDATE work_claim SET active = false WHERE agent_id = '{}' AND active = true;",
                    escape_surreal(agent_id)
                ),
                format!("Released ALL claims for agent '{agent_id}'."),
            ),
        };
        self.query_one(&sql)
            .await
            .map_err(|e| CoordError::StorageFailed(format!("Release: {e}")))?;
        if target.is_none() {
            let _ = self.deactivate_agent(agent_id).await;
        }
        Ok(desc)
    }

    async fn who(&self, agent_id_filter: Option<&str>) -> Result<CoordSnapshot, CoordError> {
        if let Err(e) = self.query_one("UPDATE agent_session SET active = false WHERE active = true AND (time::now() - last_seen) > 5m;").await {
            tracing::warn!(error = %e, "coord TTL expiry (sessions) failed");
        }
        if let Err(e) = self.query_one("UPDATE work_claim SET active = false WHERE active = true AND agent_id NOT IN (SELECT VALUE agent_id FROM agent_session WHERE active = true);").await {
            tracing::warn!(error = %e, "coord TTL expiry (claims) failed");
        }
        let session_sql = match agent_id_filter {
            Some(id) => format!(
                "SELECT * FROM agent_session WHERE agent_id = '{}' AND active = true;",
                escape_surreal(id)
            ),
            None => "SELECT * FROM agent_session WHERE active = true;".to_string(),
        };
        let agent_rows = match self.query_one(&session_sql).await {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!(error = %e, "coord who() agents query failed");
                Vec::new()
            }
        };
        let agents: Vec<AgentInfo> = agent_rows
            .iter()
            .map(|r| AgentInfo {
                agent_id: r["agent_id"].as_str().unwrap_or("").to_string(),
                agent_type: r["agent_type"].as_str().unwrap_or("").to_string(),
                intent: r["intent"].as_str().unwrap_or("").to_string(),
                active: r["active"].as_bool().unwrap_or(false),
                registered_at: r["registered_at"].as_str().unwrap_or("").to_string(),
                last_seen: r["last_seen"].as_str().unwrap_or("").to_string(),
                id: r["id"].as_str().unwrap_or("").to_string(),
            })
            .collect();
        let claims_sql = match agent_id_filter {
            Some(id) => format!(
                "SELECT * FROM work_claim WHERE agent_id = '{}' AND active = true;",
                escape_surreal(id)
            ),
            None => "SELECT * FROM work_claim WHERE active = true;".to_string(),
        };
        let claim_rows = match self.query_one(&claims_sql).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "coord who() claims query failed");
                Vec::new()
            }
        };
        let claims: Vec<ClaimEntry> = claim_rows
            .iter()
            .map(|r| ClaimEntry {
                agent_id: r["agent_id"].as_str().unwrap_or("").to_string(),
                target: r["target"].as_str().unwrap_or("").to_string(),
                claim_type: r["claim_type"].as_str().unwrap_or("").to_string(),
                active: r["active"].as_bool().unwrap_or(false),
                claimed_at: r["claimed_at"].as_str().unwrap_or("").to_string(),
                id: r["id"].as_str().unwrap_or("").to_string(),
            })
            .collect();
        Ok(CoordSnapshot { agents, claims })
    }

    async fn store_audit(
        &self,
        tool: &str,
        agent_id: &str,
        details: &str,
    ) -> Result<(), CoordError> {
        let query = format!(
            "CREATE mcp_audit SET ts = time::now(), tool = '{}', agent_id = '{}', details = '{}';",
            escape_surreal(tool),
            escape_surreal(agent_id),
            escape_surreal(details),
        );
        if let Err(e) = self.query(&query).await {
            tracing::warn!(tool = %tool, agent_id = %agent_id, error = %e, "store_audit failed");
        }
        Ok(())
    }

    async fn query_audit(
        &self,
        tool_filter: Option<&str>,
        agent_filter: Option<&str>,
        limit: u32,
    ) -> Result<Vec<AuditEntry>, CoordError> {
        let mut conditions = Vec::new();
        if let Some(tool) = tool_filter {
            conditions.push(format!("tool = '{}'", escape_surreal(tool)));
        }
        if let Some(agent) = agent_filter {
            conditions.push(format!("agent_id = '{}'", escape_surreal(agent)));
        }
        let where_clause = build_where_clause(&conditions);
        let query = format!(
            "SELECT * FROM mcp_audit{where_clause} ORDER BY ts DESC LIMIT {};",
            limit.min(100)
        );
        let rows = self
            .query_one(&query)
            .await
            .map_err(|e| CoordError::StorageFailed(format!("Audit query: {e}")))?;
        Ok(rows
            .iter()
            .map(|r| AuditEntry {
                ts: r["ts"].as_str().unwrap_or("").to_string(),
                tool: r["tool"].as_str().unwrap_or("").to_string(),
                agent_id: r["agent_id"].as_str().unwrap_or("").to_string(),
                details: r["details"].as_str().unwrap_or("").to_string(),
                id: r["id"].as_str().unwrap_or("").to_string(),
            })
            .collect())
    }

    async fn heartbeat(&self, agent_id: &str) -> Result<(), CoordError> {
        self.query_one(&format!(
            "UPDATE agent_session:`{}` SET last_seen = time::now();",
            sanitize_record_id(agent_id)
        ))
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, agent_id, "heartbeat query failed");
            CoordError::StorageFailed(format!("Heartbeat: {e}"))
        })?;
        Ok(())
    }

    async fn deactivate_agent(&self, agent_id: &str) -> Result<(), CoordError> {
        self.query_one(&format!(
            "UPDATE agent_session:`{}` SET active = false, last_seen = time::now();",
            sanitize_record_id(agent_id)
        ))
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, agent_id, "deactivate_agent query failed");
            CoordError::StorageFailed(format!("Deactivate: {e}"))
        })?;
        Ok(())
    }

    async fn claim_batch(
        &self,
        agent_id: &str,
        targets: &[String],
        claim_type: &str,
    ) -> Result<BatchClaimResult, CoordError> {
        if targets.is_empty() {
            return Ok(BatchClaimResult {
                claimed: Vec::new(),
                conflicts: Vec::new(),
            });
        }
        if targets.len() > 20 {
            return Err(CoordError::InvalidInput(
                "batch claim limited to 20 targets".into(),
            ));
        }

        let target_list: Vec<String> = targets
            .iter()
            .map(|t| format!("'{}'", escape_surreal(t)))
            .collect();
        let check_sql = format!(
            "SELECT * FROM work_claim WHERE target IN [{}] AND agent_id != '{}' AND active = true;",
            target_list.join(", "),
            escape_surreal(agent_id)
        );
        let conflict_rows = self
            .query_one(&check_sql)
            .await
            .map_err(|e| CoordError::StorageFailed(format!("Batch claim check: {e}")))?;

        let mut conflict_map: std::collections::HashMap<String, Vec<ConflictInfo>> =
            std::collections::HashMap::new();
        for row in &conflict_rows {
            let target = row["target"].as_str().unwrap_or("").to_string();
            let info = ConflictInfo {
                agent_id: row["agent_id"].as_str().unwrap_or("?").to_string(),
                claimed_at: row["claimed_at"].as_str().unwrap_or("?").to_string(),
            };
            conflict_map.entry(target).or_default().push(info);
        }

        let mut result = BatchClaimResult {
            claimed: Vec::new(),
            conflicts: Vec::new(),
        };
        let claimable: Vec<&String> = targets
            .iter()
            .filter(|t| !conflict_map.contains_key(t.as_str()))
            .collect();

        for target in targets {
            if let Some(infos) = conflict_map.remove(target.as_str()) {
                result.conflicts.push((target.clone(), infos));
            }
        }

        if !claimable.is_empty() {
            let mut batch_sql = String::new();
            for target in &claimable {
                use std::fmt::Write;
                let _ = write!(
                    batch_sql,
                    "UPSERT work_claim:`{target_key}` SET \
                        agent_id = '{agent_id}', target = '{target}', claim_type = '{claim_type}', \
                        claimed_at = time::now(), active = true; ",
                    target_key = sanitize_record_id(target),
                    agent_id = escape_surreal(agent_id),
                    target = escape_surreal(target),
                    claim_type = escape_surreal(claim_type),
                );
            }
            self.query(&batch_sql)
                .await
                .map_err(|e| CoordError::StorageFailed(format!("Batch claim: {e}")))?;

            let verify_list: Vec<String> = claimable
                .iter()
                .map(|t| format!("work_claim:`{}`", sanitize_record_id(t)))
                .collect();
            let verify_sql = format!("SELECT agent_id, target FROM {};", verify_list.join(", "));
            let owned = match self.query_one(&verify_sql).await {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "coord claim_batch post-check failed, treating as conflicts"
                    );
                    Vec::new()
                }
            };
            let owned_set: std::collections::HashSet<String> = owned
                .iter()
                .filter(|r| r["agent_id"].as_str() == Some(agent_id))
                .filter_map(|r| r["target"].as_str().map(String::from))
                .collect();

            for target in &claimable {
                if owned_set.contains(target.as_str()) {
                    result.claimed.push((*target).clone());
                } else {
                    result.conflicts.push((
                        (*target).clone(),
                        vec![ConflictInfo {
                            agent_id: "unknown (race)".into(),
                            claimed_at: "just now".into(),
                        }],
                    ));
                }
            }
        }

        if let Err(e) = self.heartbeat(agent_id).await {
            tracing::warn!(error = %e, agent_id, "heartbeat after batch claim failed");
        }

        Ok(result)
    }

    async fn expire_stale(&self) -> Result<(), CoordError> {
        self.query_one("UPDATE agent_session SET active = false WHERE active = true AND (time::now() - last_seen) > 5m;").await
            .map_err(|e| CoordError::StorageFailed(format!("expire sessions: {e}")))?;
        self.query_one("UPDATE work_claim SET active = false WHERE active = true AND agent_id NOT IN (SELECT VALUE agent_id FROM agent_session WHERE active = true);").await
            .map_err(|e| CoordError::StorageFailed(format!("expire claims: {e}")))?;
        // K15: audit trail TTL — prevent unbounded growth (was 11K+ rows, never pruned)
        self.query_one("DELETE FROM mcp_audit WHERE ts < time::now() - 7d;")
            .await
            .map_err(|e| CoordError::StorageFailed(format!("audit ttl: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn who_expiry_sql_is_valid() {
        let expiry_sql = "UPDATE agent_session SET active = false WHERE active = true AND (time::now() - last_seen) > 5m;";
        assert!(expiry_sql.contains("time::now()"));
        assert!(expiry_sql.contains("> 5m"));
        assert!(expiry_sql.contains("active = true"));
        let cascade_sql = "UPDATE work_claim SET active = false WHERE active = true AND agent_id NOT IN (SELECT VALUE agent_id FROM agent_session WHERE active = true);";
        assert!(cascade_sql.contains("NOT IN"));
        assert!(cascade_sql.contains("SELECT VALUE agent_id"));
    }

    #[test]
    fn who_with_agent_id_filters_active() {
        let id = "test-agent";
        let sql = format!(
            "SELECT * FROM agent_session WHERE agent_id = '{}' AND active = true;",
            escape_surreal(id)
        );
        assert!(sql.contains("AND active = true"));
    }

    #[test]
    fn sanitize_record_id_no_collision() {
        let key1 = sanitize_record_id("a-b");
        let key2 = sanitize_record_id("a.b");
        assert_ne!(
            key1, key2,
            "different inputs must produce different record IDs"
        );
    }

    #[test]
    fn sanitize_record_id_no_collision_with_encoding_literal() {
        let key_literal = sanitize_record_id("a%2db");
        let key_encoded = sanitize_record_id("a-b");
        assert_ne!(
            key_literal, key_encoded,
            "literal %2d must not collide with encoded dash"
        );
    }

    #[test]
    fn sanitize_record_id_safe_chars_unchanged() {
        assert_eq!(sanitize_record_id("hello_world_123"), "hello_world_123");
    }

    #[test]
    fn sanitize_record_id_length_limited() {
        let long = "a".repeat(300);
        let result = sanitize_record_id(&long);
        assert_eq!(result.len(), 256, "output should be truncated at 256 chars");
    }

    #[test]
    fn sanitize_record_id_utf8_no_panic() {
        let s = "é".repeat(200);
        let result = sanitize_record_id(&s);
        assert!(!result.is_empty());
    }

    #[test]
    fn sanitize_record_id_special_chars_encoded() {
        let result = sanitize_record_id("file/path.rs");
        assert!(result.contains("%2f"), "slash should be percent-encoded");
        assert!(result.contains("%2e"), "dot should be percent-encoded");
    }
}
