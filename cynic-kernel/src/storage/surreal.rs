//! StoragePort + CoordPort implementations for SurrealHttpStorage.

use super::{SurrealHttpStorage, sanitize_id, escape_surreal, safe_limit};
use crate::domain::coord::{CoordPort, CoordError, ClaimResult, ConflictInfo, CoordSnapshot, BatchClaimResult};
use crate::domain::dog::{Verdict, VerdictKind, QScore, AxiomReasoning};
use crate::domain::ccm::{Crystal, CrystalState};
use crate::domain::storage::{StoragePort, StorageError, Observation};

// ── VERDICT SERIALIZATION ────────────────────────────────────

fn verdict_to_sql(v: &Verdict) -> String {
    let escape = |s: &str| escape_surreal(s);

    let integrity = v.integrity_hash.as_deref().unwrap_or("");
    let prev = v.prev_hash.as_deref().unwrap_or("");

    format!(
        "CREATE verdict SET \
            verdict_id = '{}', \
            kind = '{:?}', \
            total = {}, \
            fidelity = {}, \
            phi = {}, \
            verify = {}, \
            culture = {}, \
            burn = {}, \
            sovereignty = {}, \
            reasoning_fidelity = '{}', \
            reasoning_phi = '{}', \
            reasoning_verify = '{}', \
            reasoning_culture = '{}', \
            reasoning_burn = '{}', \
            reasoning_sovereignty = '{}', \
            dog_id = '{}', \
            stimulus = '{}', \
            integrity_hash = '{}', \
            prev_hash = '{}', \
            anomaly_detected = {}, \
            max_disagreement = {}, \
            anomaly_axiom = '{}', \
            created_at = time::now()",
        escape(&v.id),
        v.kind,
        v.q_score.total,
        v.q_score.fidelity,
        v.q_score.phi,
        v.q_score.verify,
        v.q_score.culture,
        v.q_score.burn,
        v.q_score.sovereignty,
        escape(&v.reasoning.fidelity),
        escape(&v.reasoning.phi),
        escape(&v.reasoning.verify),
        escape(&v.reasoning.culture),
        escape(&v.reasoning.burn),
        escape(&v.reasoning.sovereignty),
        escape(&v.dog_id),
        escape(&v.stimulus_summary),
        escape(integrity),
        escape(prev),
        v.anomaly_detected,
        v.max_disagreement,
        escape(v.anomaly_axiom.as_deref().unwrap_or("")),
    )
}

fn row_to_verdict(row: &serde_json::Value) -> Verdict {
    let kind_str = row["kind"].as_str().unwrap_or("Bark");
    let kind = match kind_str {
        "Howl" => VerdictKind::Howl,
        "Wag" => VerdictKind::Wag,
        "Growl" => VerdictKind::Growl,
        _ => VerdictKind::Bark,
    };

    Verdict {
        id: row["verdict_id"].as_str().unwrap_or("").to_string(),
        kind,
        q_score: QScore {
            total: row["total"].as_f64().unwrap_or(0.0),
            fidelity: row["fidelity"].as_f64().unwrap_or(0.0),
            phi: row["phi"].as_f64().unwrap_or(0.0),
            verify: row["verify"].as_f64().unwrap_or(0.0),
            culture: row["culture"].as_f64().unwrap_or(0.0),
            burn: row["burn"].as_f64().unwrap_or(0.0),
            sovereignty: row["sovereignty"].as_f64().unwrap_or(0.0),
        },
        reasoning: AxiomReasoning {
            fidelity: row["reasoning_fidelity"].as_str().unwrap_or("").to_string(),
            phi: row["reasoning_phi"].as_str().unwrap_or("").to_string(),
            verify: row["reasoning_verify"].as_str().unwrap_or("").to_string(),
            culture: row["reasoning_culture"].as_str().unwrap_or("").to_string(),
            burn: row["reasoning_burn"].as_str().unwrap_or("").to_string(),
            sovereignty: row["reasoning_sovereignty"].as_str().unwrap_or("").to_string(),
        },
        dog_id: row["dog_id"].as_str().unwrap_or("").to_string(),
        stimulus_summary: row["stimulus"].as_str().unwrap_or("").to_string(),
        timestamp: row["created_at"].as_str().unwrap_or("").to_string(),
        dog_scores: Vec::new(),
        anomaly_detected: row["anomaly_detected"].as_bool().unwrap_or(false),
        max_disagreement: row["max_disagreement"].as_f64().unwrap_or(0.0),
        anomaly_axiom: row["anomaly_axiom"].as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        failed_dogs: Vec::new(),
        integrity_hash: row["integrity_hash"].as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        prev_hash: row["prev_hash"].as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
    }
}

fn row_to_crystal(row: &serde_json::Value) -> Crystal {
    let state_str = row["state"].as_str().unwrap_or("forming");
    let state = match state_str {
        "crystallized" => CrystalState::Crystallized,
        "canonical" => CrystalState::Canonical,
        "decaying" => CrystalState::Decaying,
        "dissolved" => CrystalState::Dissolved,
        _ => CrystalState::Forming,
    };
    // SurrealDB record IDs look like "crystal:abc123" — strip the table prefix
    let raw_id = row["id"].as_str().unwrap_or("");
    let id = raw_id.strip_prefix("crystal:").unwrap_or(raw_id).to_string();
    Crystal {
        id,
        content: row["content"].as_str().unwrap_or("").to_string(),
        domain: row["domain"].as_str().unwrap_or("").to_string(),
        confidence: row["confidence"].as_f64().unwrap_or(0.0),
        observations: row["observations"].as_u64().unwrap_or(0) as u32,
        state,
        created_at: row["created_at"].as_str().unwrap_or("").to_string(),
        updated_at: row["updated_at"].as_str().unwrap_or("").to_string(),
    }
}

// ── STORAGE PORT IMPLEMENTATION ──────────────────────────────

#[async_trait::async_trait]
impl StoragePort for SurrealHttpStorage {
    async fn ping(&self) -> Result<(), StorageError> {
        self.query("INFO FOR DB;").await?;
        Ok(())
    }

    fn metrics(&self) -> Option<crate::domain::storage::StorageMetrics> {
        let snap = self.metrics.snapshot();
        Some(crate::domain::storage::StorageMetrics {
            queries: snap.queries,
            errors: snap.errors,
            slow_queries: snap.slow_queries,
            avg_latency_ms: snap.avg_latency_ms,
            uptime_secs: snap.uptime_secs,
        })
    }

    async fn store_verdict(&self, verdict: &Verdict) -> Result<(), StorageError> {
        let sql = verdict_to_sql(verdict);
        self.query_one(&sql).await?;
        Ok(())
    }

    async fn get_verdict(&self, id: &str) -> Result<Option<Verdict>, StorageError> {
        let id = sanitize_id(id)?;
        let sql = format!("SELECT * FROM verdict WHERE verdict_id = '{}' LIMIT 1", id);
        let rows = self.query_one(&sql).await?;
        Ok(rows.first().map(row_to_verdict))
    }

    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError> {
        let sql = format!("SELECT * FROM verdict ORDER BY created_at DESC LIMIT {}", safe_limit(limit));
        let rows = self.query_one(&sql).await?;
        Ok(rows.iter().map(row_to_verdict).collect())
    }

    async fn store_crystal(&self, crystal: &Crystal) -> Result<(), StorageError> {
        let escape = |s: &str| escape_surreal(s);
        let state_str = match crystal.state {
            CrystalState::Forming => "forming",
            CrystalState::Crystallized => "crystallized",
            CrystalState::Canonical => "canonical",
            CrystalState::Decaying => "decaying",
            CrystalState::Dissolved => "dissolved",
        };
        let sql = format!(
            "UPSERT crystal:`{}` SET content = '{}', domain = '{}', confidence = {}, observations = {}, state = '{}', created_at = '{}', updated_at = '{}'",
            escape(&crystal.id), escape(&crystal.content), escape(&crystal.domain),
            crystal.confidence, crystal.observations, state_str,
            escape(&crystal.created_at), escape(&crystal.updated_at)
        );
        self.query_one(&sql).await?;
        Ok(())
    }

    async fn get_crystal(&self, id: &str) -> Result<Option<Crystal>, StorageError> {
        let id = sanitize_id(id)?;
        let sql = format!("SELECT * FROM crystal:`{}`", id);
        let rows = self.query_one(&sql).await?;
        Ok(rows.first().map(row_to_crystal))
    }

    async fn list_crystals(&self, limit: u32) -> Result<Vec<Crystal>, StorageError> {
        let sql = format!("SELECT * FROM crystal ORDER BY observations DESC LIMIT {}", safe_limit(limit));
        let rows = self.query_one(&sql).await?;
        Ok(rows.iter().map(row_to_crystal).collect())
    }

    async fn list_crystals_for_domain(&self, domain: &str, limit: u32) -> Result<Vec<Crystal>, StorageError> {
        let safe_domain = escape_surreal(domain);
        let sql = format!(
            "SELECT * FROM crystal \
             WHERE (domain = '{domain}' OR domain = 'general') \
             AND (state = 'crystallized' OR state = 'canonical') \
             ORDER BY confidence DESC \
             LIMIT {limit}",
            domain = safe_domain, limit = safe_limit(limit),
        );
        let rows = self.query_one(&sql).await?;
        Ok(rows.iter().map(row_to_crystal).collect())
    }

    async fn observe_crystal(&self, id: &str, content: &str, domain: &str, score: f64, timestamp: &str) -> Result<(), StorageError> {
        let safe_id = sanitize_id(id)?;

        // Atomic observe: LET binds + UPDATE avoids TOCTOU race.
        // SurrealDB 3.x doesn't support nested IF...END — use LET variables.
        // All LET expressions read pre-update snapshot (confirmed via probe).
        //
        // State classification thresholds (from domain/ccm.rs):
        //   Canonical:    obs >= 233 AND conf >= 0.618
        //   Crystallized: obs >= 21  AND conf >= 0.618
        //   Decaying:     obs >= 21  AND conf <  0.382
        //   Forming:      everything else
        let sql = format!(
            "LET $prev_obs = (SELECT VALUE observations FROM crystal:`{id}`)[0] ?? 0; \
             LET $prev_conf = (SELECT VALUE confidence FROM crystal:`{id}`)[0] ?? 0.0; \
             LET $new_obs = $prev_obs + 1; \
             LET $new_conf = IF $prev_obs > 0 THEN ($prev_conf * $prev_obs + {score}) / $new_obs ELSE {score} END; \
             LET $new_state = IF $new_obs >= 233 AND $new_conf >= 0.618 THEN 'canonical' \
                 ELSE IF $new_obs >= 21 AND $new_conf >= 0.618 THEN 'crystallized' \
                 ELSE IF $new_obs >= 21 AND $new_conf < 0.382 THEN 'decaying' \
                 ELSE 'forming' END; \
             UPSERT crystal:`{id}` SET \
                 content = '{content}', \
                 domain = '{domain}', \
                 observations = $new_obs, \
                 confidence = $new_conf, \
                 state = $new_state, \
                 created_at = created_at ?? '{ts}', \
                 updated_at = '{ts}';",
            id = safe_id,
            content = escape_surreal(content),
            domain = escape_surreal(domain),
            score = score,
            ts = escape_surreal(timestamp),
        );
        self.query(&sql).await?;
        Ok(())
    }

    async fn store_crystal_embedding(&self, id: &str, embedding: &[f32]) -> Result<(), StorageError> {
        let safe_id = escape_surreal(id);
        // Serialize Vec<f32> as SurrealQL array literal
        let vec_str: String = embedding.iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "UPDATE crystal:`{id}` SET embedding = [{vec}]",
            id = safe_id, vec = vec_str,
        );
        self.query_one(&sql).await?;
        Ok(())
    }

    async fn search_crystals_semantic(&self, query_embedding: &[f32], limit: u32) -> Result<Vec<Crystal>, StorageError> {
        let k = safe_limit(limit).min(20); // Cap KNN at 20
        let vec_str: String = query_embedding.iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(",");
        // KNN search with HNSW index, filter to mature crystals only
        let sql = format!(
            "LET $q = [{vec}]; \
             SELECT *, vector::similarity::cosine(embedding, $q) AS similarity \
             FROM crystal \
             WHERE embedding <|{k},40|> $q \
             AND (state = 'crystallized' OR state = 'canonical') \
             ORDER BY similarity DESC;",
            vec = vec_str, k = k,
        );
        let results = self.query(&sql).await?;
        // Multi-statement: LET returns empty, SELECT is the second result set
        let rows = results.into_iter().nth(1).unwrap_or_default();
        Ok(rows.iter().map(row_to_crystal).collect())
    }

    async fn find_similar_crystal(&self, embedding: &[f32], domain: &str, threshold: f64) -> Result<Option<(String, f64)>, StorageError> {
        let safe_domain = escape_surreal(domain);
        let vec_str: String = embedding.iter()
            .map(|v| if v.is_finite() { *v } else { 0.0f32 })
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(",");
        // KNN search across ALL crystal states (including Forming) within domain.
        // Used for merging: "1. e4 c5" and "1. e4 c5 — Sicilian Defense" should
        // accumulate observations on the same crystal, not fragment into separate ones.
        let sql = format!(
            "LET $q = [{vec}]; \
             SELECT meta::id(id) AS crystal_id, vector::similarity::cosine(embedding, $q) AS similarity \
             FROM crystal \
             WHERE embedding <|5,40|> $q \
             AND domain = '{domain}' \
             ORDER BY similarity DESC \
             LIMIT 1;",
            vec = vec_str, domain = safe_domain,
        );
        let results = self.query(&sql).await?;
        let rows = results.into_iter().nth(1).unwrap_or_default();
        if let Some(row) = rows.first() {
            let sim = row.get("similarity").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if sim >= threshold {
                let id = row.get("crystal_id")
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

    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError> {
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
                created_at = time::now();",
            project = escape_surreal(&obs.project),
            agent_id = escape_surreal(&obs.agent_id),
            tool = escape_surreal(&obs.tool),
            target = escape_surreal(&obs.target),
            domain = escape_surreal(&obs.domain),
            status = escape_surreal(&obs.status),
            context = escape_surreal(&obs.context),
            session_id = escape_surreal(&obs.session_id),
        );
        self.query(&sql).await?;
        Ok(())
    }

    async fn query_observations(&self, project: &str, domain: Option<&str>, limit: u32) -> Result<Vec<serde_json::Value>, StorageError> {
        let limit = safe_limit(limit);
        let domain_clause = match domain {
            Some(d) => format!(" AND domain = '{}'", escape_surreal(d)),
            None => String::new(),
        };
        // Return target frequency + co-occurrence data
        let sql = format!(
            "SELECT target, tool, count() AS freq FROM observation \
             WHERE project = '{}'{} \
             GROUP BY target, tool ORDER BY freq DESC LIMIT {};",
            escape_surreal(project), domain_clause, limit,
        );
        self.query_one(&sql).await
    }

    async fn query_session_targets(&self, project: &str, limit: u32) -> Result<Vec<serde_json::Value>, StorageError> {
        // Internal aggregation query — not user-facing. Needs more rows than safe_limit(100)
        // for co-occurrence detection across many sessions. Cap at 1000.
        let limit = limit.min(1000);
        // Group by agent_id (= session identity in Claude Code).
        // session_id field is unreliable (often empty). agent_id is derived
        // from CLAUDE_SESSION_ID by the PostToolUse hook.
        let sql = format!(
            "SELECT agent_id AS session_id, target FROM observation \
             WHERE project = '{}' AND agent_id != '' AND agent_id != 'unknown' \
             AND tool IN ['Edit', 'Write', 'Read'] \
             ORDER BY agent_id, target LIMIT {};",
            escape_surreal(project), limit,
        );
        self.query_one(&sql).await
    }

    async fn store_session_summary(&self, summary: &crate::domain::ccm::SessionSummary) -> Result<(), StorageError> {
        // Record key = session_id → idempotent UPSERT, no duplicates (H4 fix)
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
        self.query_one(&sql).await?;
        Ok(())
    }

    async fn list_session_summaries(&self, limit: u32) -> Result<Vec<crate::domain::ccm::SessionSummary>, StorageError> {
        let sql = format!(
            "SELECT * FROM session_summary ORDER BY created_at DESC LIMIT {}",
            safe_limit(limit),
        );
        let rows = self.query_one(&sql).await?;
        Ok(rows.iter().map(|row| crate::domain::ccm::SessionSummary {
            session_id: row["session_id"].as_str().unwrap_or("").to_string(),
            agent_id: row["agent_id"].as_str().unwrap_or("").to_string(),
            summary: row["summary"].as_str().unwrap_or("").to_string(),
            observations_count: row["observations_count"].as_u64().unwrap_or(0) as u32,
            created_at: row["created_at"].as_str().unwrap_or("").to_string(),
        }).collect())
    }

    async fn get_unsummarized_sessions(&self, min_observations: u32, limit: u32) -> Result<Vec<(String, String, u32)>, StorageError> {
        // SurrealDB 3.x: no HAVING clause. Filter in Rust after GROUP BY.
        // NOT IN subquery filters out already-summarized sessions.
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
        let rows = self.query_one(&sql).await?;
        Ok(rows.iter().filter_map(|row| {
            let agent_id = row["agent_id"].as_str().unwrap_or("").to_string();
            let count = row["obs_count"].as_u64().unwrap_or(0) as u32;
            if count >= min_observations {
                Some((agent_id.clone(), agent_id, count))
            } else {
                None
            }
        }).collect())
    }

    async fn get_session_observations(&self, session_id: &str) -> Result<Vec<serde_json::Value>, StorageError> {
        let sql = format!(
            "SELECT tool, target, domain, status, context, created_at FROM observation \
             WHERE agent_id = '{}' ORDER BY created_at ASC LIMIT 50;",
            escape_surreal(session_id),
        );
        self.query_one(&sql).await
    }

    async fn flush_usage(&self, snapshot: &[(String, crate::domain::usage::DogUsage)]) -> Result<(), StorageError> {
        if snapshot.is_empty() {
            return Ok(());
        }
        let mut sql = String::new();
        for (dog_id, u) in snapshot {
            use std::fmt::Write;
            let _ = write!(sql, // ok: fmt::Write on String is infallible
                "UPSERT dog_usage:`{id}` SET \
                    dog_id = '{id}', \
                    prompt_tokens = IF prompt_tokens THEN prompt_tokens + {pt} ELSE {pt} END, \
                    completion_tokens = IF completion_tokens THEN completion_tokens + {ct} ELSE {ct} END, \
                    requests = IF requests THEN requests + {req} ELSE {req} END, \
                    failures = IF failures THEN failures + {fail} ELSE {fail} END, \
                    total_latency_ms = IF total_latency_ms THEN total_latency_ms + {lat} ELSE {lat} END, \
                    updated_at = time::now(); ",
                id = dog_id, pt = u.prompt_tokens, ct = u.completion_tokens,
                req = u.requests, fail = u.failures, lat = u.total_latency_ms,
            );
        }
        self.query(&sql).await?;
        Ok(())
    }

    async fn cleanup_ttl(&self) -> Result<(), StorageError> {
        self.query_one("DELETE observation WHERE created_at < time::now() - 30d;")
            .await.map_err(|e| StorageError::QueryFailed(format!("TTL cleanup observation: {}", e)))?;
        self.query_one("DELETE mcp_audit WHERE ts < time::now() - 7d;")
            .await.map_err(|e| StorageError::QueryFailed(format!("TTL cleanup mcp_audit: {}", e)))?;
        Ok(())
    }

    async fn last_integrity_hash(&self) -> Result<Option<String>, StorageError> {
        let rows = self.query_one("SELECT integrity_hash FROM verdict ORDER BY created_at DESC LIMIT 1;").await?;
        Ok(rows.first()
            .and_then(|r| r["integrity_hash"].as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()))
    }

    async fn load_usage_history(&self) -> Result<Vec<serde_json::Value>, StorageError> {
        self.query_one("SELECT * FROM dog_usage;").await
    }
}

// ── COORD PORT IMPLEMENTATION ────────────────────────────────

fn sanitize_record_id(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect()
}

#[async_trait::async_trait]
impl CoordPort for SurrealHttpStorage {
    async fn register_agent(&self, agent_id: &str, agent_type: &str, intent: &str) -> Result<(), CoordError> {
        let sql = format!(
            "UPSERT agent_session:`{id_key}` SET \
                agent_id = '{id_val}', agent_type = '{agent_type}', intent = '{intent}', \
                registered_at = time::now(), last_seen = time::now(), active = true;",
            id_key = sanitize_record_id(agent_id),
            id_val = escape_surreal(agent_id),
            agent_type = escape_surreal(agent_type),
            intent = escape_surreal(intent),
        );
        self.query_one(&sql).await.map_err(|e| CoordError::StorageFailed(format!("Register: {}", e)))?;
        Ok(())
    }

    async fn claim(&self, agent_id: &str, target: &str, claim_type: &str) -> Result<ClaimResult, CoordError> {
        // Single-row-per-target: UPSERT uses `target` as the record key (not agent_id+target).
        // This ensures two agents racing on the same target write to the SAME row.
        // The last writer wins the UPSERT; the post-check reads back to see who owns it.
        // This eliminates the mutual-rollback liveness deadlock of the old per-agent key scheme.
        let target_key = sanitize_record_id(target);

        // First check: is this target already actively claimed by someone else?
        let check_sql = format!(
            "SELECT * FROM work_claim:`{key}` WHERE agent_id != '{agent_id}' AND active = true;",
            key = target_key,
            agent_id = escape_surreal(agent_id),
        );
        let existing = self.query_one(&check_sql).await
            .map_err(|e| CoordError::StorageFailed(format!("Claim check: {}", e)))?;
        if !existing.is_empty() {
            let infos = existing.iter().map(|c| ConflictInfo {
                agent_id: c["agent_id"].as_str().unwrap_or("?").to_string(),
                claimed_at: c["claimed_at"].as_str().unwrap_or("?").to_string(),
            }).collect();
            return Ok(ClaimResult::Conflict(infos));
        }

        // Claim: UPSERT the single row for this target
        let upsert_sql = format!(
            "UPSERT work_claim:`{key}` SET \
                agent_id = '{agent_id}', target = '{target}', claim_type = '{claim_type}', \
                claimed_at = time::now(), active = true;",
            key = target_key,
            agent_id = escape_surreal(agent_id),
            target = escape_surreal(target),
            claim_type = escape_surreal(claim_type),
        );
        self.query_one(&upsert_sql).await
            .map_err(|e| CoordError::StorageFailed(format!("Claim upsert: {}", e)))?;

        let _ = self.heartbeat(agent_id).await; // ok: fire-and-forget
        Ok(ClaimResult::Claimed)
    }

    async fn release(&self, agent_id: &str, target: Option<&str>) -> Result<String, CoordError> {
        let (sql, desc) = match target {
            Some(t) => (
                format!("UPDATE work_claim SET active = false WHERE agent_id = '{}' AND target = '{}' AND active = true;",
                    escape_surreal(agent_id), escape_surreal(t)),
                format!("Released '{}' for agent '{}'.", t, agent_id),
            ),
            None => (
                format!("UPDATE work_claim SET active = false WHERE agent_id = '{}' AND active = true;",
                    escape_surreal(agent_id)),
                format!("Released ALL claims for agent '{}'.", agent_id),
            ),
        };
        self.query_one(&sql).await.map_err(|e| CoordError::StorageFailed(format!("Release: {}", e)))?;
        if target.is_none() { let _ = self.deactivate_agent(agent_id).await; } // ok: fire-and-forget
        Ok(desc)
    }

    async fn who(&self, agent_id_filter: Option<&str>) -> Result<CoordSnapshot, CoordError> {
        if let Err(e) = self.query_one("UPDATE agent_session SET active = false WHERE active = true AND (time::now() - last_seen) > 5m;").await {
            eprintln!("[coord] TTL expiry (sessions) failed: {}", e);
        }
        if let Err(e) = self.query_one("UPDATE work_claim SET active = false WHERE active = true AND agent_id NOT IN (SELECT VALUE agent_id FROM agent_session WHERE active = true);").await {
            eprintln!("[coord] TTL expiry (claims) failed: {}", e);
        }
        let session_sql = match agent_id_filter {
            Some(id) => format!("SELECT * FROM agent_session WHERE agent_id = '{}' AND active = true;", escape_surreal(id)),
            None => "SELECT * FROM agent_session WHERE active = true;".to_string(),
        };
        let agents = self.query_one(&session_sql).await.unwrap_or_default();
        let claims_sql = match agent_id_filter {
            Some(id) => format!("SELECT * FROM work_claim WHERE agent_id = '{}' AND active = true;", escape_surreal(id)),
            None => "SELECT * FROM work_claim WHERE active = true;".to_string(),
        };
        let claims = self.query_one(&claims_sql).await.unwrap_or_default();
        Ok(CoordSnapshot { agents, claims })
    }

    async fn store_audit(&self, tool: &str, agent_id: &str, details: &serde_json::Value) -> Result<(), CoordError> {
        let safe_details = escape_surreal(&details.to_string());
        // Only CREATE — the expensive DELETE is done periodically by the usage flush task.
        // Previous version ran DELETE ... NOT IN (SELECT ... LIMIT 10000) on EVERY call,
        // causing full table scans and transaction drops under load.
        let query = format!(
            "CREATE mcp_audit SET ts = time::now(), tool = '{}', agent_id = '{}', details = '{}';",
            escape_surreal(tool), escape_surreal(agent_id), safe_details,
        );
        if let Err(e) = self.query(&query).await {
            eprintln!("[store_audit] ERROR (tool={}, agent={}): {}", tool, agent_id, e);
        }
        Ok(())
    }

    async fn query_audit(&self, tool_filter: Option<&str>, agent_filter: Option<&str>, limit: u32) -> Result<Vec<serde_json::Value>, CoordError> {
        let limit = limit.min(100);
        let mut conditions = Vec::new();
        if let Some(tool) = tool_filter { conditions.push(format!("tool = '{}'", escape_surreal(tool))); }
        if let Some(agent) = agent_filter { conditions.push(format!("agent_id = '{}'", escape_surreal(agent))); }
        let where_clause = if conditions.is_empty() { String::new() } else { format!(" WHERE {}", conditions.join(" AND ")) };
        let query = format!("SELECT * FROM mcp_audit{} ORDER BY ts DESC LIMIT {};", where_clause, limit);
        self.query_one(&query).await.map_err(|e| CoordError::StorageFailed(format!("Audit query: {}", e)))
    }

    async fn heartbeat(&self, agent_id: &str) -> Result<(), CoordError> {
        let _ = self.query_one(&format!("UPDATE agent_session:`{}` SET last_seen = time::now();", sanitize_record_id(agent_id))).await; // ok: fire-and-forget
        Ok(())
    }

    async fn deactivate_agent(&self, agent_id: &str) -> Result<(), CoordError> {
        let _ = self.query_one(&format!("UPDATE agent_session:`{}` SET active = false, last_seen = time::now();", sanitize_record_id(agent_id))).await; // ok: fire-and-forget
        Ok(())
    }

    async fn claim_batch(&self, agent_id: &str, targets: &[String], claim_type: &str) -> Result<BatchClaimResult, CoordError> {
        use crate::domain::coord::BatchClaimResult;

        if targets.is_empty() {
            return Ok(BatchClaimResult { claimed: Vec::new(), conflicts: Vec::new() });
        }
        if targets.len() > 20 {
            return Err(CoordError::InvalidInput("batch claim limited to 20 targets".into()));
        }

        // Single query: check all conflicts at once
        let target_list: Vec<String> = targets.iter()
            .map(|t| format!("'{}'", escape_surreal(t)))
            .collect();
        let check_sql = format!(
            "SELECT * FROM work_claim WHERE target IN [{}] AND agent_id != '{}' AND active = true;",
            target_list.join(", "), escape_surreal(agent_id)
        );
        let conflict_rows = self.query_one(&check_sql).await
            .map_err(|e| CoordError::StorageFailed(format!("Batch claim check: {}", e)))?;

        // Index conflicts by target
        let mut conflict_map: std::collections::HashMap<String, Vec<ConflictInfo>> = std::collections::HashMap::new();
        for row in &conflict_rows {
            let target = row["target"].as_str().unwrap_or("").to_string();
            let info = ConflictInfo {
                agent_id: row["agent_id"].as_str().unwrap_or("?").to_string(),
                claimed_at: row["claimed_at"].as_str().unwrap_or("?").to_string(),
            };
            conflict_map.entry(target).or_default().push(info);
        }

        let mut result = BatchClaimResult { claimed: Vec::new(), conflicts: Vec::new() };

        // Separate claimed vs conflicted
        let claimable: Vec<&String> = targets.iter()
            .filter(|t| !conflict_map.contains_key(t.as_str()))
            .collect();

        for target in targets {
            if let Some(infos) = conflict_map.remove(target.as_str()) {
                result.conflicts.push((target.clone(), infos));
            }
        }

        // Batch UPSERT all claimable targets in one SQL
        if !claimable.is_empty() {
            let mut batch_sql = String::new();
            for target in &claimable {
                use std::fmt::Write;
                let _ = write!(batch_sql, // ok: fmt::Write on String
                    "UPSERT work_claim:`{target_key}` SET \
                        agent_id = '{agent_id}', target = '{target}', claim_type = '{claim_type}', \
                        claimed_at = time::now(), active = true; ",
                    target_key = sanitize_record_id(target),
                    agent_id = escape_surreal(agent_id),
                    target = escape_surreal(target),
                    claim_type = escape_surreal(claim_type),
                );
                result.claimed.push((*target).clone());
            }
            self.query(&batch_sql).await
                .map_err(|e| CoordError::StorageFailed(format!("Batch claim: {}", e)))?;
        }

        // Single heartbeat for the batch
        let _ = self.heartbeat(agent_id).await; // ok: fire-and-forget

        Ok(result)
    }

    async fn expire_stale(&self) -> Result<(), CoordError> {
        // Two separate queries to avoid ordering dependency within a single batch
        self.query_one("UPDATE agent_session SET active = false WHERE active = true AND (time::now() - last_seen) > 5m;").await
            .map_err(|e| CoordError::StorageFailed(format!("expire sessions: {}", e)))?;
        self.query_one("UPDATE work_claim SET active = false WHERE active = true AND agent_id NOT IN (SELECT VALUE agent_id FROM agent_session WHERE active = true);").await
            .map_err(|e| CoordError::StorageFailed(format!("expire claims: {}", e)))?;
        Ok(())
    }
}

// ── TESTS ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dog::{AxiomReasoning, VerdictKind, QScore};

    fn test_verdict() -> Verdict {
        Verdict {
            id: "test-001".into(),
            kind: VerdictKind::Wag,
            q_score: QScore {
                total: 0.5, fidelity: 0.5, phi: 0.5,
                verify: 0.5, culture: 0.5, burn: 0.5, sovereignty: 0.5,
            },
            reasoning: AxiomReasoning {
                fidelity: "solid".into(), phi: "humble".into(), verify: "checked".into(),
                culture: "neutral".into(), burn: "concise".into(), sovereignty: "independent".into(),
            },
            dog_id: "deterministic".into(),
            stimulus_summary: "test stimulus".into(),
            timestamp: "2026-03-13T12:00:00Z".into(),
            dog_scores: Vec::new(),
            anomaly_detected: false,
            max_disagreement: 0.0,
            anomaly_axiom: None,
            failed_dogs: Vec::new(),
            integrity_hash: Some("deadbeef".into()),
            prev_hash: None,
        }
    }

    #[test]
    fn verdict_sql_is_valid() {
        let v = test_verdict();
        let sql = verdict_to_sql(&v);
        assert!(sql.contains("CREATE verdict SET"));
        assert!(sql.contains("verdict_id = 'test-001'"));
        assert!(sql.contains("kind = 'Wag'"));
        assert!(sql.contains("fidelity = 0.5"));
        assert!(sql.contains("time::now()"));
        assert!(sql.contains("integrity_hash = 'deadbeef'"));
        assert!(sql.contains("prev_hash = ''"));
    }

    #[test]
    fn row_to_verdict_parses_correctly() {
        let row = serde_json::json!({
            "verdict_id": "v-123",
            "kind": "Howl",
            "total": 0.82,
            "fidelity": 0.9,
            "phi": 0.85,
            "verify": 0.88,
            "culture": 0.80,
            "burn": 0.75,
            "sovereignty": 0.70,
            "reasoning_fidelity": "strong evidence",
            "reasoning_phi": "appropriately humble",
            "reasoning_verify": "falsifiable",
            "reasoning_culture": "culturally aware",
            "reasoning_burn": "concise",
            "reasoning_sovereignty": "no vendor lock",
            "dog_id": "inference-gemini",
            "stimulus": "test claim",
            "created_at": "2026-03-13T10:00:00Z"
        });

        let v = row_to_verdict(&row);
        assert_eq!(v.id, "v-123");
        assert_eq!(v.kind, VerdictKind::Howl);
        assert_eq!(v.q_score.total, 0.82);
        assert_eq!(v.q_score.sovereignty, 0.70);
        assert_eq!(v.reasoning.burn, "concise");
    }

    #[test]
    fn sql_escapes_quotes() {
        let mut v = test_verdict();
        v.stimulus_summary = "it's a \"test\"".into();
        let sql = verdict_to_sql(&v);
        assert!(sql.contains("it\\'s a "));
    }

    #[test]
    fn sanitize_id_rejects_injection() {
        assert!(sanitize_id("'; DROP TABLE verdict; --").is_err());
        assert!(sanitize_id("abc\0def").is_err());
        assert!(sanitize_id("abc`def").is_err());
        assert!(sanitize_id("abc;def").is_err());
        assert!(sanitize_id("abc def").is_err());
        assert!(sanitize_id("").is_err());
        assert!(sanitize_id(&"a".repeat(129)).is_err());
    }

    #[test]
    fn sanitize_id_accepts_valid() {
        assert!(sanitize_id("abc-123_def").is_ok());
        assert!(sanitize_id("verdict-001").is_ok());
        assert!(sanitize_id("a1b2c3d4e5f6").is_ok());
        assert!(sanitize_id(&"a".repeat(128)).is_ok());
    }

    #[test]
    fn escape_surreal_handles_special_chars() {
        assert_eq!(escape_surreal("it's"), "it\\'s");
        assert_eq!(escape_surreal("a\\b"), "a\\\\b");
        assert_eq!(escape_surreal("a\0b"), "ab");
        assert_eq!(escape_surreal("a\nb"), "a\\nb");
        assert_eq!(escape_surreal("a\rb"), "a\\rb");
        assert_eq!(escape_surreal("a\tb"), "a\\tb");
    }

    #[test]
    fn safe_limit_clamps() {
        assert_eq!(safe_limit(20), 20);
        assert_eq!(safe_limit(100), 100);
        assert_eq!(safe_limit(101), 100);
        assert_eq!(safe_limit(u32::MAX), 100);
    }

    #[test]
    fn who_expiry_sql_is_valid() {
        // Verify the TTL expiry SQL used in who() is syntactically sound.
        // SurrealDB uses `(time::now() - last_seen) > 5m` — the `5m` is a duration literal.
        let expiry_sql = "UPDATE agent_session SET active = false WHERE active = true AND (time::now() - last_seen) > 5m;";
        assert!(expiry_sql.contains("time::now()"));
        assert!(expiry_sql.contains("> 5m"));
        assert!(expiry_sql.contains("active = true"));
        // Verify the cascade SQL that deactivates orphaned claims
        let cascade_sql = "UPDATE work_claim SET active = false WHERE active = true AND agent_id NOT IN (SELECT VALUE agent_id FROM agent_session WHERE active = true);";
        assert!(cascade_sql.contains("NOT IN"));
        assert!(cascade_sql.contains("SELECT VALUE agent_id"));
    }

    #[test]
    fn who_with_agent_id_filters_active() {
        // Regression: who(Some(id)) must include `active = true` filter.
        let id = "test-agent";
        let sql = format!("SELECT * FROM agent_session WHERE agent_id = '{}' AND active = true;", escape_surreal(id));
        assert!(sql.contains("AND active = true"));
    }

    #[tokio::test]
    #[ignore] // Requires running SurrealDB instance
    async fn store_and_retrieve_verdict() {
        let storage = SurrealHttpStorage::init_with(
            "http://localhost:8000", "test_cynic", "ci"
        ).await.expect("SurrealDB must be reachable");

        let v = test_verdict();
        storage.store_verdict(&v).await.expect("store must succeed");

        let retrieved = storage.get_verdict("test-001").await.expect("get must succeed");
        assert!(retrieved.is_some());
        let r = retrieved.unwrap();
        assert_eq!(r.id, "test-001");
        assert_eq!(r.kind, VerdictKind::Wag);

        // Cleanup
        let _ = storage.query_one("DELETE verdict WHERE verdict_id = 'test-001'").await;
    }
}
