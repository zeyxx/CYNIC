//! StoragePort + CoordPort implementations for SurrealHttpStorage.

mod activity;
mod coord;

use super::{SurrealHttpStorage, escape_surreal, safe_limit, sanitize_id};
use crate::domain::ccm::{CANONICAL_CYCLES, Crystal, CrystalState, MIN_CRYSTALLIZATION_CYCLES};
use crate::domain::dog::{
    AxiomReasoning, DogScore, PHI_INV, PHI_INV2, QScore, Verdict, VerdictKind,
};
use crate::domain::storage::{
    Observation, ObservationFrequency, RawObservation, SessionTarget, StorageError, StoragePort,
    UsageRow,
};

// ── VERDICT SERIALIZATION ────────────────────────────────────

fn verdict_to_sql(v: &Verdict) -> String {
    let escape = |s: &str| escape_surreal(s);

    let integrity = v.integrity_hash.as_deref().unwrap_or("");
    let prev = v.prev_hash.as_deref().unwrap_or("");

    format!(
        "CREATE verdict SET \
            verdict_id = '{}', \
            domain = '{}', \
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
            voter_count = {}, \
            dog_scores_json = '{}', \
            created_at = d'{}'",
        escape(&v.id),
        escape(&v.domain),
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
        v.voter_count,
        escape(&serde_json::to_string(&v.dog_scores).unwrap_or_else(|_| "[]".to_string())),
        escape(&v.timestamp),
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
        domain: row["domain"].as_str().unwrap_or("general").to_string(),
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
            sovereignty: row["reasoning_sovereignty"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        },
        dog_id: row["dog_id"].as_str().unwrap_or("").to_string(),
        stimulus_summary: row["stimulus"].as_str().unwrap_or("").to_string(),
        timestamp: row["created_at"].as_str().unwrap_or("").to_string(),
        dog_scores: {
            let verdict_id_for_log = row["verdict_id"].as_str().unwrap_or("?");
            let voter_count_for_log = row["voter_count"].as_u64().unwrap_or(0);
            let scores: Vec<DogScore> = row["dog_scores_json"]
                .as_str()
                .filter(|s| !s.is_empty())
                .and_then(|s| {
                    serde_json::from_str(s)
                        .map_err(|e| {
                            tracing::error!(
                                verdict_id = %verdict_id_for_log,
                                error = %e,
                                "dog_scores_json parse failed — verdict provenance corrupted"
                            );
                            e
                        })
                        .ok()
                })
                .unwrap_or_default();
            // Detect inconsistency: voter_count says N dogs contributed but scores are empty
            if scores.is_empty() && voter_count_for_log > 0 {
                tracing::warn!(
                    verdict_id = %verdict_id_for_log,
                    voter_count = voter_count_for_log,
                    "dog_scores empty but voter_count > 0 — per-dog provenance lost"
                );
            }
            scores
        },
        // K14: missing/poison = assume degraded (anomaly present, not absent)
        anomaly_detected: row["anomaly_detected"].as_bool().unwrap_or(true),
        max_disagreement: row["max_disagreement"].as_f64().unwrap_or(0.0),
        anomaly_axiom: row["anomaly_axiom"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        voter_count: row["voter_count"].as_u64().unwrap_or(0) as usize,
        failed_dogs: Vec::new(),
        failed_dog_errors: Default::default(),
        integrity_hash: row["integrity_hash"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        prev_hash: row["prev_hash"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
    }
}

fn row_to_crystal(row: &serde_json::Value) -> Crystal {
    let state: CrystalState = row["state"]
        .as_str()
        .unwrap_or("forming")
        .parse()
        .unwrap_or(CrystalState::Forming);
    // SurrealDB record IDs: "crystal:abc123" or "crystal:`hyphen-id`" — strip table prefix + backticks
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

// ── SHARED HELPERS ─────────────────────────────────────────────

/// Serialize an embedding as a SurrealQL array literal. NaN/Inf → 0.0.
fn embedding_to_surreal_literal(embedding: &[f32]) -> String {
    embedding
        .iter()
        .map(|v| if v.is_finite() { *v } else { 0.0f32 })
        .map(|v| format!("{v}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Build a SQL WHERE clause from optional conditions.
/// Returns empty string if no conditions, or ` WHERE cond1 AND cond2` (leading space).
fn build_where_clause(conditions: &[String]) -> String {
    if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    }
}

/// Sanitize string for use as SurrealDB record ID.
/// Uses percent-encoding: unsafe chars -> `%XX`, `%` itself -> `%25`.
/// Collision-free: injective mapping (different inputs -> different outputs).
/// Length-limited to 256 chars (char-aware, no UTF-8 boundary panic).
fn sanitize_record_id(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().take(256) {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else if c == '%' {
            out.push_str("%25");
        } else if c.is_ascii() {
            out.push_str(&format!("%{:02x}", c as u8));
        } else {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).bytes() {
                out.push_str(&format!("%{b:02x}"));
            }
        }
    }
    out
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
        let sql = format!("SELECT * FROM verdict WHERE verdict_id = '{id}' LIMIT 1");
        let rows = self.query_one(&sql).await?;
        Ok(rows.first().map(row_to_verdict))
    }

    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError> {
        let sql = format!(
            "SELECT * FROM verdict ORDER BY created_at DESC LIMIT {}",
            safe_limit(limit)
        );
        let rows = self.query_one(&sql).await?;
        Ok(rows.iter().map(row_to_verdict).collect())
    }

    async fn store_crystal(&self, crystal: &Crystal) -> Result<(), StorageError> {
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
        self.query_one(&sql).await?;
        Ok(())
    }

    async fn get_crystal(&self, id: &str) -> Result<Option<Crystal>, StorageError> {
        let id = sanitize_id(id)?;
        let sql = format!("SELECT * FROM crystal:`{id}`");
        let rows = self.query_one(&sql).await?;
        Ok(rows.first().map(row_to_crystal))
    }

    async fn list_crystals(&self, limit: u32) -> Result<Vec<Crystal>, StorageError> {
        // Sort by state ASC (alphabetical: canonical < crystallized < decaying < dissolved < forming)
        // then confidence DESC. Shows mature crystals first, noise last.
        // Previous sort (observations DESC) surfaced workflow noise, hiding 29 mature crystals.
        let sql = format!(
            "SELECT * FROM crystal ORDER BY state ASC, confidence DESC LIMIT {}",
            safe_limit(limit),
        );
        let rows = self.query_one(&sql).await?;
        Ok(rows.iter().map(row_to_crystal).collect())
    }

    async fn list_crystals_filtered(
        &self,
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
        let rows = self.query_one(&sql).await?;
        Ok(rows.iter().map(row_to_crystal).collect())
    }

    async fn delete_crystal(&self, id: &str) -> Result<(), StorageError> {
        let safe_id = sanitize_id(id)?;
        let sql = format!("DELETE crystal:`{safe_id}`");
        self.query_one(&sql).await?;
        Ok(())
    }

    async fn list_crystals_for_domain(
        &self,
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
        let rows = self.query_one(&sql).await?;
        Ok(rows.iter().map(row_to_crystal).collect())
    }

    async fn observe_crystal(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: &str,
        voter_count: usize,
        verdict_id: &str,
    ) -> Result<(), StorageError> {
        // T5+T8: quorum enforcement — reject observations from single-Dog or no-Dog sources.
        // This gate covers ALL callers (pipeline + REST + introspection).
        if voter_count < crate::domain::dog::MIN_QUORUM {
            return Err(StorageError::QueryFailed(format!(
                "quorum not met: voter_count={voter_count}, min={}",
                crate::domain::dog::MIN_QUORUM
            )));
        }

        let safe_id = sanitize_id(id)?;

        // T7: sanitize content BEFORE storage — prevents prompt injection via crystal content.
        // This is the PRIMARY defense. format_crystal_context delimiters are defense-in-depth.
        // Covers ALL callers: pipeline (observe_crystal_for_verdict) AND REST (observe_crystal_handler).
        let sanitized_content = crate::domain::sanitize::sanitize_crystal_content(content);

        // Atomic observe: LET binds + UPDATE avoids TOCTOU race.
        // SurrealDB 3.x doesn't support nested IF...END — use LET variables.
        // All LET expressions read pre-update snapshot (confirmed via probe).
        //
        // State classification thresholds (from domain constants):
        //   Canonical:    obs >= CANONICAL_CYCLES AND conf >= PHI_INV
        //   Crystallized: obs >= MIN_CRYSTALLIZATION_CYCLES AND conf >= PHI_INV
        //   Decaying:     obs >= MIN_CRYSTALLIZATION_CYCLES AND conf < PHI_INV2
        //   Forming:      everything else
        let t_canon = CANONICAL_CYCLES;
        let t_cryst = MIN_CRYSTALLIZATION_CYCLES;
        let c_high = PHI_INV;
        let c_low = PHI_INV2;
        // Guard: NaN/Inf would produce invalid SurrealQL and corrupt the running mean.
        let score = if score.is_finite() { score } else { 0.0 };
        // Transaction: atomicity for the read-compute-write cycle.
        // Without this, two concurrent observe_crystal calls for the same crystal
        // can both LET-read the old count, both compute new_obs = old+1, and one
        // increment is lost. SurrealDB serializes conflicting transactions.
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
            ts = escape_surreal(timestamp),
        );
        self.query(&sql).await?;
        Ok(())
    }

    async fn store_crystal_embedding(
        &self,
        id: &str,
        embedding: &[f32],
    ) -> Result<(), StorageError> {
        let safe_id = sanitize_id(id)?;
        let vec_str = embedding_to_surreal_literal(embedding);
        let sql = format!("UPDATE crystal:`{safe_id}` SET embedding = [{vec_str}]",);
        self.query_one(&sql).await?;
        Ok(())
    }

    async fn search_crystals_semantic(
        &self,
        query_embedding: &[f32],
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        let k = safe_limit(limit).min(20); // Cap KNN at 20
        let vec_str = embedding_to_surreal_literal(query_embedding);
        // KNN search with HNSW index, filter to mature crystals only
        let sql = format!(
            "LET $q = [{vec_str}]; \
             SELECT *, vector::similarity::cosine(embedding, $q) AS similarity \
             FROM crystal \
             WHERE embedding <|{k},40|> $q \
             AND (state = 'crystallized' OR state = 'canonical') \
             ORDER BY similarity DESC;",
        );
        let results = self.query(&sql).await?;
        // Multi-statement: LET returns empty, SELECT is the second result set
        let rows = results.into_iter().nth(1).unwrap_or_default();
        Ok(rows.iter().map(row_to_crystal).collect())
    }

    async fn find_similar_crystal(
        &self,
        embedding: &[f32],
        domain: &str,
        threshold: f64,
    ) -> Result<Option<(String, f64)>, StorageError> {
        let safe_domain = escape_surreal(domain);
        let vec_str = embedding_to_surreal_literal(embedding);
        // KNN search across ALL crystal states (including Forming) within domain.
        // Used for merging: "1. e4 c5" and "1. e4 c5 — Sicilian Defense" should
        // accumulate observations on the same crystal, not fragment into separate ones.
        let sql = format!(
            "LET $q = [{vec_str}]; \
             SELECT meta::id(id) AS crystal_id, vector::similarity::cosine(embedding, $q) AS similarity \
             FROM crystal \
             WHERE embedding <|5,40|> $q \
             AND domain = '{safe_domain}' \
             ORDER BY similarity DESC \
             LIMIT 1;",
        );
        let results = self.query(&sql).await?;
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

    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError> {
        activity::store_observation(self, obs).await
    }

    async fn query_observations(
        &self,
        project: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ObservationFrequency>, StorageError> {
        activity::query_observations(self, project, domain, limit).await
    }

    async fn query_session_targets(
        &self,
        project: &str,
        limit: u32,
    ) -> Result<Vec<SessionTarget>, StorageError> {
        activity::query_session_targets(self, project, limit).await
    }

    async fn store_session_summary(
        &self,
        summary: &crate::domain::ccm::SessionSummary,
    ) -> Result<(), StorageError> {
        activity::store_session_summary(self, summary).await
    }

    async fn list_session_summaries(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::SessionSummary>, StorageError> {
        activity::list_session_summaries(self, limit).await
    }

    async fn get_unsummarized_sessions(
        &self,
        min_observations: u32,
        limit: u32,
    ) -> Result<Vec<(String, String, u32)>, StorageError> {
        activity::get_unsummarized_sessions(self, min_observations, limit).await
    }

    async fn get_session_observations(
        &self,
        session_id: &str,
    ) -> Result<Vec<RawObservation>, StorageError> {
        activity::get_session_observations(self, session_id).await
    }

    async fn store_session_compliance(
        &self,
        c: &crate::domain::compliance::SessionCompliance,
    ) -> Result<(), StorageError> {
        activity::store_session_compliance(self, c).await
    }

    async fn list_session_compliance(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::compliance::SessionCompliance>, StorageError> {
        activity::list_session_compliance(self, limit).await
    }

    async fn flush_usage(
        &self,
        snapshot: &[(String, crate::domain::usage::DogUsage)],
    ) -> Result<(), StorageError> {
        if snapshot.is_empty() {
            return Ok(());
        }
        // Idempotent: SET absolute totals, not += deltas.
        // Callers pass flush_snapshot() (historical + session merged).
        // On partial failure + retry, the same absolute values are re-written — no double-count.
        let mut sql = String::new();
        for (dog_id, u) in snapshot {
            use std::fmt::Write;
            let id_key = sanitize_record_id(dog_id);
            let id_val = escape_surreal(dog_id);
            let _ = write!(
                sql, // ok: fmt::Write on String is infallible
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
                pt = u.prompt_tokens,
                ct = u.completion_tokens,
                req = u.requests,
                fail = u.failures,
                lat = u.total_latency_ms,
            );
        }
        self.query(&sql).await?;
        Ok(())
    }

    async fn cleanup_ttl(&self) -> Result<(), StorageError> {
        self.query_one("DELETE observation WHERE created_at < time::now() - 30d;")
            .await
            .map_err(|e| StorageError::QueryFailed(format!("TTL cleanup observation: {e}")))?;
        self.query_one("DELETE mcp_audit WHERE ts < time::now() - 7d;")
            .await
            .map_err(|e| StorageError::QueryFailed(format!("TTL cleanup mcp_audit: {e}")))?;
        // K15: Complete crystal lifecycle — forming zombies and terminal decay.
        // Forming crystals with no update in 90 days → dissolved (never reached critical mass).
        // Decaying crystals with confidence < 0.1 → dissolved (irreversibly lost trust).
        // Dissolved crystals older than 30 days → deleted (final cleanup).
        let dissolved = self
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
        let purged = self
            .query_one(
                "DELETE crystal WHERE state = 'dissolved' AND updated_at < time::now() - 30d;",
            )
            .await;
        if let Err(e) = purged {
            tracing::warn!("dissolved crystal purge failed (non-fatal): {e}");
        }
        Ok(())
    }

    async fn last_integrity_hash(&self) -> Result<Option<String>, StorageError> {
        let rows = self
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

    async fn load_usage_history(&self) -> Result<Vec<UsageRow>, StorageError> {
        let rows = self.query_one("SELECT * FROM dog_usage;").await?;
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

    async fn flush_dog_stats(
        &self,
        stats: &[(String, crate::organ::health::DogStats)],
    ) -> Result<(), StorageError> {
        if stats.is_empty() {
            return Ok(());
        }
        // Idempotent: SET absolute totals (same pattern as flush_usage).
        let mut sql = String::new();
        for (dog_id, s) in stats {
            use std::fmt::Write;
            let id_key = sanitize_record_id(dog_id);
            let id_val = escape_surreal(dog_id);
            let last_success_sql = match &s.last_success {
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
                    updated_at = time::now(); ",
                id_key = id_key,
                id_val = id_val,
                tc = s.total_calls,
                sc = s.success_count,
                zf = s.zero_flood_count,
                cc = s.collapse_count,
                pe = s.parse_error_count,
                to = s.timeout_count,
                ae = s.api_error_count,
                ls = last_success_sql,
                lat = s.total_latency_ms,
            );
        }
        self.query(&sql).await?;
        Ok(())
    }

    async fn load_dog_stats(
        &self,
    ) -> Result<Vec<(String, crate::organ::health::DogStats)>, StorageError> {
        let rows = self.query_one("SELECT * FROM dog_stats;").await?;
        Ok(rows
            .iter()
            .filter_map(|r| {
                let dog_id = r["dog_id"].as_str()?.to_string();
                if dog_id.is_empty() {
                    return None;
                }
                Some((
                    dog_id,
                    crate::organ::health::DogStats {
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
                    },
                ))
            })
            .collect())
    }

    async fn list_crystals_missing_embedding(
        &self,
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        // Internal migration query — not user-facing. Cap at 500, not safe_limit(100).
        let sql = format!(
            "SELECT * FROM crystal WHERE embedding IS NONE OR embedding = [] LIMIT {}",
            limit.min(500),
        );
        let rows = self.query_one(&sql).await?;
        Ok(rows.iter().map(row_to_crystal).collect())
    }

    async fn count_verdicts(&self) -> Result<u64, StorageError> {
        let rows = self
            .query_one("SELECT count() AS total FROM verdict GROUP ALL;")
            .await?;
        Ok(rows.first().and_then(|r| r["total"].as_u64()).unwrap_or(0))
    }

    async fn count_crystal_observations(&self) -> Result<u64, StorageError> {
        let rows = self
            .query_one("SELECT math::sum(observations) AS total FROM crystal GROUP ALL;")
            .await?;
        Ok(rows.first().and_then(|r| r["total"].as_u64()).unwrap_or(0))
    }

    async fn list_observations_raw(
        &self,
        domain: Option<&str>,
        agent_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        activity::list_observations_raw(self, domain, agent_id, limit).await
    }

    async fn store_infra_snapshot(
        &self,
        snap: &crate::domain::probe::EnvironmentSnapshot,
    ) -> Result<(), StorageError> {
        let probes_json = serde_json::to_string(&snap.probes)
            .map_err(|e| StorageError::QueryFailed(format!("serialize probes: {e}")))?;
        let overall = format!("{:?}", snap.overall);
        let sql = format!(
            "INSERT INTO infra_snapshot {{ ts: time::now(), overall: '{}', probes: {} }}",
            escape_surreal(&overall),
            probes_json,
        );
        self.query(&sql).await?;
        Ok(())
    }

    async fn list_infra_snapshots(
        &self,
        hours: u32,
    ) -> Result<Vec<crate::domain::probe::EnvironmentSnapshot>, StorageError> {
        let sql = format!(
            "SELECT * FROM infra_snapshot WHERE ts > time::now() - {hours}h ORDER BY ts DESC LIMIT 100",
        );
        let rows = self.query_one(&sql).await?;
        Ok(rows
            .iter()
            .map(|row| {
                let timestamp = row["ts"].as_str().unwrap_or("").to_string();
                let overall_str = row["overall"].as_str().unwrap_or("Unavailable");
                let overall = match overall_str {
                    "Ok" => crate::domain::probe::ProbeStatus::Ok,
                    "Degraded" => crate::domain::probe::ProbeStatus::Degraded,
                    "Denied" => crate::domain::probe::ProbeStatus::Denied,
                    _ => crate::domain::probe::ProbeStatus::Unavailable,
                };
                let probes: Vec<crate::domain::probe::ProbeResult> = row["probes"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| {
                                serde_json::from_value(v.clone())
                                    .map_err(|e| {
                                        tracing::warn!(
                                            error = %e,
                                            "infra_snapshot probe deserialization failed"
                                        );
                                        e
                                    })
                                    .ok()
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                crate::domain::probe::EnvironmentSnapshot {
                    timestamp,
                    probes,
                    overall,
                }
            })
            .collect())
    }

    async fn cleanup_infra_snapshots(&self, older_than_days: u32) -> Result<u64, StorageError> {
        let sql = format!(
            "DELETE FROM infra_snapshot WHERE ts < time::now() - {older_than_days}d RETURN BEFORE",
        );
        let rows = self.query_one(&sql).await?;
        Ok(rows.len() as u64)
    }

    async fn consolidate_duplicate_crystals(&self) -> Result<u64, StorageError> {
        // Phase 1: find groups of crystals with identical (domain, content).
        // SurrealDB has no HAVING — filter duplicates (cnt > 1) in Rust.
        let groups_sql = "SELECT domain, content, \
                          array::group(meta::id(id)) AS ids, \
                          count() AS cnt \
                          FROM crystal \
                          GROUP BY domain, content";
        let all_groups = self.query_one(groups_sql).await?;
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
            // Pick survivor: the crystal with the most observations.
            // Read all crystals in the group to find it.
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
            let details = self.query_one(&detail_sql).await?;
            if details.len() < 2 {
                continue;
            }
            // Survivor = first (most observations). Merge the rest into it.
            let survivor_id = match details[0].get("cid").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };
            let mut total_obs: u64 = 0;
            let mut weighted_conf: f64 = 0.0;
            let mut all_verdicts: Vec<String> = Vec::new();
            let mut earliest_created = String::new();

            for d in &details {
                let obs = d.get("observations").and_then(|v| v.as_u64()).unwrap_or(0);
                let conf = d.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
                total_obs += obs;
                weighted_conf += conf * obs as f64;
                if let Some(arr) = d.get("contributing_verdicts").and_then(|v| v.as_array()) {
                    for v in arr {
                        if let Some(s) = v.as_str() {
                            all_verdicts.push(s.to_string());
                        }
                    }
                }
                if let Some(ca) = d.get("created_at").and_then(|v| v.as_str())
                    && (earliest_created.is_empty() || ca < earliest_created.as_str())
                {
                    earliest_created = ca.to_string();
                }
            }
            let merged_conf = if total_obs > 0 {
                weighted_conf / total_obs as f64
            } else {
                0.0
            };
            // Reclassify state based on merged totals.
            let t_canon = crate::domain::ccm::CANONICAL_CYCLES as u64;
            let t_cryst = crate::domain::ccm::MIN_CRYSTALLIZATION_CYCLES as u64;
            let c_high = crate::domain::dog::PHI_INV;
            let c_low = crate::domain::dog::PHI_INV2;
            let new_state = if total_obs >= t_canon && merged_conf >= c_high {
                "canonical"
            } else if total_obs >= t_cryst && merged_conf >= c_high {
                "crystallized"
            } else if total_obs >= t_cryst && merged_conf < c_low {
                "decaying"
            } else {
                "forming"
            };
            // Deduplicate verdict IDs.
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

            // Update survivor with merged totals.
            let update_sql = format!(
                "UPDATE crystal:`{safe_survivor}` SET \
                 observations = {total_obs}, \
                 confidence = {merged_conf}, \
                 state = '{new_state}', \
                 contributing_verdicts = [{verdicts_arr}], \
                 created_at = '{safe_created}', \
                 updated_at = '{safe_now}'"
            );
            self.query_one(&update_sql).await?;

            // Delete duplicates via SQL — avoids cross-query ID format mismatch.
            // SurrealDB compares meta::id(id) consistently within a single query.
            let safe_domain =
                escape_surreal(group.get("domain").and_then(|v| v.as_str()).unwrap_or(""));
            let safe_content =
                escape_surreal(group.get("content").and_then(|v| v.as_str()).unwrap_or(""));
            let del_sql = format!(
                "DELETE FROM crystal WHERE domain = '{safe_domain}' \
                 AND content = '{safe_content}' \
                 AND meta::id(id) != '{safe_survivor}' \
                 RETURN BEFORE"
            );
            let deleted = self.query_one(&del_sql).await?;
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
}

// ── TESTS ────────────────────────────────────────────────────

#[cfg(test)]
// WHY: Integration tests use eprintln! for SurrealDB connection diagnostics during
// local development — these are never reached in production code paths.
#[allow(clippy::print_stderr)]
mod tests {
    use super::*;
    use crate::domain::dog::{AxiomReasoning, QScore, VerdictKind};

    fn test_verdict() -> Verdict {
        Verdict {
            id: "test-001".into(),
            domain: "test".into(),
            kind: VerdictKind::Wag,
            q_score: QScore {
                total: 0.5,
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
            },
            reasoning: AxiomReasoning {
                fidelity: "solid".into(),
                phi: "humble".into(),
                verify: "checked".into(),
                culture: "neutral".into(),
                burn: "concise".into(),
                sovereignty: "independent".into(),
            },
            dog_id: "deterministic".into(),
            stimulus_summary: "test stimulus".into(),
            timestamp: "2026-03-13T12:00:00Z".into(),
            dog_scores: Vec::new(),
            anomaly_detected: false,
            max_disagreement: 0.0,
            anomaly_axiom: None,
            voter_count: 0,
            failed_dogs: Vec::new(),
            failed_dog_errors: Default::default(),
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
        assert!(sql.contains("created_at = d'2026-03-13T12:00:00Z'"));
        assert!(sql.contains("integrity_hash = 'deadbeef'"));
        assert!(sql.contains("prev_hash = ''"));
        // S4: voter_count must be in generated SQL
        assert!(sql.contains("voter_count = 0"));
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
        // I1: voter_count defaults to 0 when absent from DB row
        assert_eq!(v.voter_count, 0);
    }

    #[test]
    fn row_to_verdict_reads_voter_count() {
        let row = serde_json::json!({
            "verdict_id": "v-456",
            "kind": "Wag",
            "total": 0.5,
            "fidelity": 0.5, "phi": 0.5, "verify": 0.5,
            "culture": 0.5, "burn": 0.5, "sovereignty": 0.5,
            "reasoning_fidelity": "", "reasoning_phi": "", "reasoning_verify": "",
            "reasoning_culture": "", "reasoning_burn": "", "reasoning_sovereignty": "",
            "dog_id": "det+gemini+sovereign",
            "stimulus": "test",
            "voter_count": 3,
            "created_at": "2026-03-27T00:00:00Z"
        });

        let v = row_to_verdict(&row);
        assert_eq!(v.voter_count, 3, "voter_count must round-trip through DB");
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

    #[tokio::test]
    async fn store_and_retrieve_verdict() {
        let storage = match SurrealHttpStorage::init_with(
            "http://localhost:8000",
            "test_cynic",
            "ci",
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[SKIP] SurrealDB unavailable (localhost:8000): {e} — skipping test");
                return;
            }
        };

        let v = test_verdict();
        storage.store_verdict(&v).await.expect("store must succeed");

        let retrieved = storage
            .get_verdict("test-001")
            .await
            .expect("get must succeed");
        assert!(retrieved.is_some());
        let r = retrieved.unwrap();
        assert_eq!(r.id, "test-001");
        assert_eq!(r.kind, VerdictKind::Wag);

        // Cleanup
        let _ = storage
            .query_one("DELETE verdict WHERE verdict_id = 'test-001'")
            .await;
    }
}
