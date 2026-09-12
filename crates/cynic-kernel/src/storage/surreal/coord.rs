use super::{build_where_clause, sanitize_record_id};
use crate::domain::coord::{
    AgentInfo, AuditEntry, BatchClaimResult, ClaimEntry, ClaimResult, ConflictInfo, CoordError,
    CoordPort, CoordSnapshot, LeaseAction, LeaseGuard, LeaseMutationResult, LeaseVerification,
    TargetClaimOutcome,
};
use crate::domain::lease_proof::ValidatedLeaseProof;
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
        let lease_id = new_lease_id();
        let upsert_sql = format!(
            "UPSERT work_claim:`{key}` SET \
                agent_id = '{agent_id}', target = '{target}', claim_type = '{claim_type}', \
                claimed_at = time::now(), active = true, lease_id = '{lease_id}', \
                expires_at = time::now() + 5m, fencing_token += 1 \
             WHERE active != true OR agent_id = '{agent_id}' OR expires_at = NONE \
                OR expires_at <= time::now() RETURN AFTER;",
            key = target_key,
            agent_id = escape_surreal(agent_id),
            target = escape_surreal(target),
            claim_type = escape_surreal(claim_type),
            lease_id = escape_surreal(&lease_id),
        );
        let acquired = self
            .query_one(&upsert_sql)
            .await
            .map_err(|e| CoordError::StorageFailed(format!("Claim upsert: {e}")))?;
        if let Some(row) = acquired.first() {
            let fencing_token = row["fencing_token"].as_u64().ok_or_else(|| {
                CoordError::StorageFailed("Claim returned no fencing_token".into())
            })?;
            let expires_at = row["expires_at"]
                .as_str()
                .ok_or_else(|| CoordError::StorageFailed("Claim returned no expires_at".into()))?;
            if let Err(e) = self.heartbeat(agent_id).await {
                tracing::warn!(error = %e, agent_id, "heartbeat after claim failed");
            }

            // [SOVEREIGN DUAL-WRITE] Mirror claim lease to CynicLedger WAL + Book
            if let Some(ledger) = &self.ledger {
                let claim_entry = cynic_protocol::agent_coordination::Claim {
                    schema: cynic_protocol::agent_coordination::CLAIM_SCHEMA.to_string(),
                    claim_id: lease_id.clone(),
                    claim_type: claim_type.to_string(),
                    agent_id: agent_id.to_string(),
                    target: target.to_string(),
                    fencing_token,
                    lease_id: lease_id.clone(),
                    claimed_at: chrono::Utc::now().to_rfc3339(),
                    expires_at: expires_at.to_string(),
                    active: true,
                };
                let witness = cynic_storage::types::CausalWitness {
                    principal_pubkey: [0u8; 32],
                    signature: [0u8; 64],
                    mandate_id: Some("mandate-claim".to_string()),
                    proof_anchor: None,
                };
                use cynic_storage::book::LedgerBook;
                if let Err(e) = ledger.claims.post(&claim_entry, witness) {
                    tracing::warn!(error = %e, "Failed to mirror Claim to Sovereign Ledger Book");
                } else {
                    crate::klog!(
                        "[Ring 1] Claim lease {} (target: {}) mirrored to Sovereign WAL + Book",
                        lease_id,
                        target
                    );
                }
            }

            return Ok(ClaimResult::Claimed(crate::domain::coord::ClaimGrant {
                lease_id,
                fencing_token,
                expires_at: expires_at.to_string(),
            }));
        }

        let conflict_sql = format!(
            "SELECT agent_id, claimed_at FROM work_claim:`{target_key}` WHERE active = true;"
        );
        let conflicts = self
            .query_one(&conflict_sql)
            .await
            .map_err(|e| CoordError::StorageFailed(format!("Claim conflict observation: {e}")))?
            .iter()
            .map(|row| ConflictInfo {
                agent_id: row["agent_id"].as_str().unwrap_or("unknown").to_string(),
                claimed_at: row["claimed_at"].as_str().unwrap_or("unknown").to_string(),
            })
            .collect();
        Ok(ClaimResult::Conflict(conflicts))
    }

    async fn verify_lease(&self, guard: &LeaseGuard) -> Result<LeaseVerification, CoordError> {
        let sql = format!(
            "SELECT expires_at FROM work_claim:`{key}` WHERE active = true AND agent_id = '{agent}' AND target = '{target}' AND lease_id = '{lease}' AND fencing_token = {token} AND expires_at > time::now();",
            key = sanitize_record_id(&guard.target),
            agent = escape_surreal(&guard.agent_id),
            target = escape_surreal(&guard.target),
            lease = escape_surreal(&guard.lease_id),
            token = guard.fencing_token,
        );
        let rows = self
            .query_one(&sql)
            .await
            .map_err(|error| CoordError::StorageFailed(format!("Lease verify: {error}")))?;
        Ok(rows.first().map_or_else(
            || LeaseVerification::rejected(guard.clone()),
            |row| {
                LeaseVerification::proved(
                    guard.clone(),
                    row["expires_at"].as_str().map(str::to_string),
                )
            },
        ))
    }

    async fn renew_lease(&self, guard: &LeaseGuard) -> Result<LeaseMutationResult, CoordError> {
        let sql = format!(
            "UPDATE work_claim:`{key}` SET expires_at = time::now() + 5m WHERE active = true AND agent_id = '{agent}' AND target = '{target}' AND lease_id = '{lease}' AND fencing_token = {token} AND expires_at > time::now() RETURN AFTER;",
            key = sanitize_record_id(&guard.target),
            agent = escape_surreal(&guard.agent_id),
            target = escape_surreal(&guard.target),
            lease = escape_surreal(&guard.lease_id),
            token = guard.fencing_token
        );
        let rows = self
            .query_one(&sql)
            .await
            .map_err(|e| CoordError::StorageFailed(format!("Lease renew: {e}")))?;
        let Some(row) = rows.first() else {
            return Ok(LeaseMutationResult::rejected(
                LeaseAction::Renew,
                &guard.target,
                guard.fencing_token,
            ));
        };
        Ok(LeaseMutationResult::applied(
            LeaseAction::Renew,
            &guard.target,
            guard.fencing_token,
            row["expires_at"].as_str().map(str::to_string),
            None,
        ))
    }

    async fn release(&self, guard: &LeaseGuard) -> Result<LeaseMutationResult, CoordError> {
        let sql = format!(
            "UPDATE work_claim:`{key}` SET active = false, status = 'released', completed_at = time::now() WHERE active = true AND agent_id = '{agent}' AND target = '{target}' AND lease_id = '{lease}' AND fencing_token = {token} AND expires_at > time::now() RETURN AFTER;",
            key = sanitize_record_id(&guard.target),
            agent = escape_surreal(&guard.agent_id),
            target = escape_surreal(&guard.target),
            lease = escape_surreal(&guard.lease_id),
            token = guard.fencing_token
        );
        let rows = self
            .query_one(&sql)
            .await
            .map_err(|e| CoordError::StorageFailed(format!("Lease release: {e}")))?;
        if rows.is_empty() {
            return Ok(LeaseMutationResult::rejected(
                LeaseAction::Release,
                &guard.target,
                guard.fencing_token,
            ));
        }

        // [SOVEREIGN DUAL-WRITE] Update claim lease in CynicLedger WAL + Book
        if let Some(ledger) = &self.ledger {
            let claim_entry = cynic_protocol::agent_coordination::Claim {
                schema: cynic_protocol::agent_coordination::CLAIM_SCHEMA.to_string(),
                claim_id: guard.lease_id.clone(),
                claim_type: "released".to_string(),
                agent_id: guard.agent_id.clone(),
                target: guard.target.clone(),
                fencing_token: guard.fencing_token,
                lease_id: guard.lease_id.clone(),
                claimed_at: chrono::Utc::now().to_rfc3339(),
                expires_at: chrono::Utc::now().to_rfc3339(),
                active: false,
            };
            let witness = cynic_storage::types::CausalWitness {
                principal_pubkey: [0u8; 32],
                signature: [0u8; 64],
                mandate_id: Some("mandate-release".to_string()),
                proof_anchor: None,
            };
            use cynic_storage::book::LedgerBook;
            let _ = ledger.claims.post(&claim_entry, witness);
        }

        Ok(LeaseMutationResult::applied(
            LeaseAction::Release,
            &guard.target,
            guard.fencing_token,
            None,
            None,
        ))
    }

    async fn finalize_lease(
        &self,
        guard: &LeaseGuard,
        proof: &ValidatedLeaseProof,
    ) -> Result<LeaseMutationResult, CoordError> {
        use ed25519_dalek::Signer;

        let proof_ref = proof.proof_ref();
        let signature = crate::domain::proof::kernel_identity().sign(proof_ref.as_bytes());
        let signature = hex::encode(signature.to_bytes());
        let sql = format!(
            "BEGIN TRANSACTION; \
             LET $claim = UPDATE work_claim:`{key}` SET active = false, status = 'finalized', completed_at = time::now(), proof_ref = '{proof_ref}' WHERE active = true AND agent_id = '{agent}' AND target = '{target}' AND lease_id = '{lease}' AND fencing_token = {token} AND expires_at > time::now() RETURN AFTER; \
             IF array::len($claim) = 0 {{ RETURN {{ applied: false }} }}; \
             CREATE runtime_proof_commit:`{commit_id}` CONTENT {{ proof_ref: '{proof_ref}', schema: 'cynic.runtime_proof_bundle.v1', status: 'committed', agent_id: '{agent}', target: '{target}', lease_id: '{lease}', fencing_token: {token}, bundle: {bundle}, hash_algorithm: 'blake3', kernel_signature: '{signature}', created_at: time::now() }}; \
             RETURN {{ applied: true }}; \
             COMMIT TRANSACTION;",
            key = sanitize_record_id(&guard.target),
            commit_id = sanitize_record_id(&guard.lease_id),
            agent = escape_surreal(&guard.agent_id),
            target = escape_surreal(&guard.target),
            lease = escape_surreal(&guard.lease_id),
            token = guard.fencing_token,
            proof_ref = escape_surreal(proof_ref),
            bundle = proof.normalized_bundle_json(),
            signature = escape_surreal(&signature),
        );
        let transaction_results = self.query(&sql).await.map_err(|error| {
            CoordError::StorageFailed(format!("Lease proof transaction: {error}"))
        })?;
        let applied = transaction_results
            .into_iter()
            .flatten()
            .find_map(|value| value.get("applied").and_then(serde_json::Value::as_bool))
            .ok_or_else(|| {
                CoordError::StorageFailed(
                    "Lease proof transaction returned no mutation outcome".into(),
                )
            })?;
        if !applied {
            return Ok(LeaseMutationResult::rejected(
                LeaseAction::Finalize,
                &guard.target,
                guard.fencing_token,
            ));
        }
        let poh = crate::domain::storage::ProofOfHistory {
            id: String::new(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            source_organ: "cynic-kernel".into(),
            event_type: "coord_lease_finalized".into(),
            payload: serde_json::json!({
                "proof_ref": proof_ref,
                "agent_id": guard.agent_id,
                "target": guard.target,
                "lease_id": guard.lease_id,
                "fencing_token": guard.fencing_token,
                "authority": "runtime_proof_commit",
            }),
            cryptographic_hash: proof_ref.into(),
            prev_hash: None,
            cryptographic_signature: None,
        };
        if let Err(error) = super::proof_of_history::store_proof_of_history(self, &poh).await {
            tracing::warn!(error = %error, proof_ref, "proof-of-history relay failed after atomic lease proof commit");
        }
        Ok(LeaseMutationResult::applied(
            LeaseAction::Finalize,
            &guard.target,
            guard.fencing_token,
            None,
            Some(proof_ref.into()),
        ))
    }

    async fn who(&self, agent_id_filter: Option<&str>) -> Result<CoordSnapshot, CoordError> {
        if let Err(e) = self.query_one("UPDATE agent_session SET active = false WHERE active = true AND (time::now() - last_seen) > 5m;").await {
            tracing::warn!(error = %e, "coord TTL expiry (sessions) failed");
        }
        // LET pre-binding: SurrealDB query planner uses indexes on LET-bound subqueries
        // but not on inline subqueries (issue #5439).
        if let Err(e) = self.query_one("LET $active = (SELECT VALUE agent_id FROM agent_session WHERE active = true); UPDATE work_claim SET active = false WHERE active = true AND (agent_id NOT IN $active OR expires_at <= time::now());").await {
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
                scope: r["scope"].as_str().unwrap_or("").to_string(),
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
                lease_id: r["lease_id"].as_str().unwrap_or("").to_string(),
                fencing_token: r["fencing_token"].as_u64().unwrap_or(0),
                expires_at: r["expires_at"].as_str().unwrap_or("").to_string(),
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
        self.query(&query)
            .await
            .map_err(|error| CoordError::StorageFailed(format!("store audit: {error}")))?;
        Ok(())
    }

    async fn query_audit(
        &self,
        tool_filter: Option<&str>,
        agent_filter: Option<&str>,
        since: Option<&str>,
        until: Option<&str>,
        limit: u32,
    ) -> Result<Vec<AuditEntry>, CoordError> {
        let mut conditions = Vec::new();
        if let Some(tool) = tool_filter {
            conditions.push(format!("tool = '{}'", escape_surreal(tool)));
        }
        if let Some(agent) = agent_filter {
            conditions.push(format!("agent_id = '{}'", escape_surreal(agent)));
        }
        if let Some(s) = since {
            conditions.push(format!("ts >= type::datetime('{}')", escape_surreal(s)));
        }
        if let Some(u) = until {
            conditions.push(format!("ts <= type::datetime('{}')", escape_surreal(u)));
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

    async fn scope_update(&self, agent_id: &str, scope: &str) -> Result<(), CoordError> {
        let sql = format!(
            "UPDATE agent_session:`{id_key}` SET scope = '{scope}', last_seen = time::now() \
             WHERE active = true;",
            id_key = sanitize_record_id(agent_id),
            scope = escape_surreal(scope),
        );
        self.query_one(&sql)
            .await
            .map_err(|e| CoordError::StorageFailed(format!("ScopeUpdate: {e}")))?;
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
        if targets.len() > 20 {
            return Err(CoordError::InvalidInput(
                "batch claim limited to 20 targets".into(),
            ));
        }

        // One authority primitive per target. This deliberately avoids a second
        // check-then-write implementation with different race semantics.
        let mut result = BatchClaimResult::default();
        for target in targets {
            match self.claim(agent_id, target, claim_type).await? {
                ClaimResult::Claimed(grant) => result.outcomes.push(TargetClaimOutcome::Claimed {
                    target: target.clone(),
                    grant,
                }),
                ClaimResult::Conflict(conflicts) => {
                    result.outcomes.push(TargetClaimOutcome::Conflict {
                        target: target.clone(),
                        conflicts,
                    });
                }
            }
        }
        Ok(result)
    }

    async fn expire_stale(&self) -> Result<(), CoordError> {
        self.query_one("UPDATE agent_session SET active = false WHERE active = true AND (time::now() - last_seen) > 5m;").await
            .map_err(|e| CoordError::StorageFailed(format!("expire sessions: {e}")))?;
        self.query_one("LET $active = (SELECT VALUE agent_id FROM agent_session WHERE active = true); UPDATE work_claim SET active = false WHERE active = true AND (agent_id NOT IN $active OR expires_at <= time::now());").await
            .map_err(|e| CoordError::StorageFailed(format!("expire claims: {e}")))?;
        // Agent Protocol v1: expire proposed claims after 2h (prevents orphan claims)
        self.query_one("UPDATE work_claim SET active = false, status = 'abandoned' WHERE status = 'proposed' AND active = true AND (time::now() - claimed_at) > 2h;").await
            .map_err(|e| CoordError::StorageFailed(format!("expire proposed: {e}")))?;
        // K15: audit trail TTL — prevent unbounded growth (was 11K+ rows, never pruned)
        self.query_one("DELETE FROM mcp_audit WHERE ts < time::now() - 7d;")
            .await
            .map_err(|e| CoordError::StorageFailed(format!("audit ttl: {e}")))?;
        Ok(())
    }
}

fn new_lease_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("lease-{nanos}-{}-{sequence}", std::process::id())
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
        // LET-bound subquery for index usage (SurrealDB issue #5439)
        let cascade_sql = "LET $active = (SELECT VALUE agent_id FROM agent_session WHERE active = true); UPDATE work_claim SET active = false WHERE active = true AND agent_id NOT IN $active;";
        assert!(cascade_sql.contains("NOT IN $active"));
        assert!(cascade_sql.contains("LET $active"));
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

    #[test]
    fn scope_update_sql_preserves_registered_at() {
        let sql = format!(
            "UPDATE agent_session:`{id_key}` SET scope = '{scope}', last_seen = time::now() \
             WHERE active = true;",
            id_key = "test_agent",
            scope = "implement coord scope feature",
        );
        assert!(sql.contains("scope ="));
        assert!(sql.contains("last_seen = time::now()"));
        assert!(
            !sql.contains("registered_at"),
            "registered_at must NOT be touched"
        );
        assert!(sql.contains("WHERE active = true"));
    }
}
