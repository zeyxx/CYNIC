//! HTTP Storage Adapter — connects to SurrealDB 3.x via POST /sql.
//! Replaces the surrealdb crate (which has compilation bugs with 3.x)
//! with direct HTTP calls using reqwest. Sovereign and dependency-light.
//!
//! SurrealDB HTTP API:
//!   POST /sql  — raw SurrealQL, headers: surreal-ns, surreal-db, Authorization: Basic
//!   Response: JSON array of statement results

use reqwest::Client;
use serde::Deserialize;
use crate::coord_port::{CoordPort, CoordError, ClaimResult, ConflictInfo, CoordSnapshot};
use crate::dog::{Verdict, VerdictKind, QScore, AxiomReasoning};
use crate::ccm::{Crystal, CrystalState};
use crate::storage_port::{StoragePort, StorageError};

/// HTTP-based SurrealDB client. No surrealdb crate needed.
pub struct SurrealHttpStorage {
    client: Client,
    url: String,
    ns: String,
    db: String,
    auth: String, // "Basic base64(user:pass)"
}

#[derive(Deserialize, Debug)]
struct SurrealResponse {
    result: Option<serde_json::Value>,
    #[allow(dead_code)]
    status: Option<String>,
}

impl SurrealHttpStorage {
    pub async fn init() -> Result<Self, StorageError> {
        let url = std::env::var("SURREALDB_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());
        let ns = "cynic";
        let db = "v2";
        Self::init_with(&url, ns, db).await
    }

    pub async fn init_with(url: &str, ns: &str, db: &str) -> Result<Self, StorageError> {
        let user = std::env::var("SURREALDB_USER").unwrap_or_else(|_| "root".to_string());
        let pass = std::env::var("SURREALDB_PASS")
            .map_err(|_| StorageError::ConnectionFailed("SURREALDB_PASS must be set".into()))?;

        use base64::Engine;
        let credentials = format!("{}:{}", user, pass);
        let auth = format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(&credentials));

        let storage = Self {
            client: Client::new(),
            url: url.trim_end_matches('/').to_string(),
            ns: ns.to_string(),
            db: db.to_string(),
            auth,
        };

        // Bootstrap namespace and database (SurrealDB 3.x doesn't auto-create)
        let bootstrap = Self {
            client: Client::new(),
            url: storage.url.clone(),
            ns: String::new(),
            db: String::new(),
            auth: storage.auth.clone(),
        };
        // Use root-level query to define ns/db
        let bootstrap_sql = format!(
            "DEFINE NAMESPACE IF NOT EXISTS `{ns}`; USE NS `{ns}`; DEFINE DATABASE IF NOT EXISTS `{db}`;",
            ns = ns, db = db
        );
        let resp = bootstrap.client
            .post(format!("{}/sql", bootstrap.url))
            .header("Accept", "application/json")
            .header("Authorization", &bootstrap.auth)
            .body(bootstrap_sql)
            .send()
            .await
            .map_err(|e| StorageError::ConnectionFailed(format!("SurrealDB unreachable at {}: {}", url, e)))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(StorageError::ConnectionFailed(format!("Bootstrap failed: {}", body)));
        }

        // Health check on the actual ns/db
        storage.query("RETURN true").await
            .map_err(|e| StorageError::ConnectionFailed(format!("SurrealDB unreachable at {}: {}", url, e)))?;

        // Bootstrap schema + indexes (idempotent — IF NOT EXISTS)
        let schema_sql = "\
            DEFINE FIELD IF NOT EXISTS verdict_id ON verdict TYPE string;\
            DEFINE FIELD IF NOT EXISTS kind ON verdict TYPE string;\
            DEFINE FIELD IF NOT EXISTS total ON verdict TYPE float;\
            DEFINE FIELD IF NOT EXISTS fidelity ON verdict TYPE float;\
            DEFINE FIELD IF NOT EXISTS phi ON verdict TYPE float;\
            DEFINE FIELD IF NOT EXISTS verify ON verdict TYPE float;\
            DEFINE FIELD IF NOT EXISTS culture ON verdict TYPE float;\
            DEFINE FIELD IF NOT EXISTS burn ON verdict TYPE float;\
            DEFINE FIELD IF NOT EXISTS sovereignty ON verdict TYPE float;\
            DEFINE FIELD IF NOT EXISTS dog_id ON verdict TYPE string;\
            DEFINE FIELD IF NOT EXISTS stimulus ON verdict TYPE string;\
            DEFINE FIELD IF NOT EXISTS created_at ON verdict TYPE datetime;\
            DEFINE FIELD IF NOT EXISTS content ON crystal TYPE string;\
            DEFINE FIELD IF NOT EXISTS domain ON crystal TYPE string;\
            DEFINE FIELD IF NOT EXISTS confidence ON crystal TYPE float;\
            DEFINE FIELD IF NOT EXISTS observations ON crystal TYPE int;\
            DEFINE FIELD IF NOT EXISTS state ON crystal TYPE string;\
            DEFINE INDEX IF NOT EXISTS verdict_id_idx ON verdict FIELDS verdict_id UNIQUE;\
            DEFINE INDEX IF NOT EXISTS verdict_created_idx ON verdict FIELDS created_at;\
            DEFINE INDEX IF NOT EXISTS crystal_obs_idx ON crystal FIELDS observations;\
            DEFINE INDEX IF NOT EXISTS crystal_domain_idx ON crystal FIELDS domain;\
        ";
        if let Err(e) = storage.query(schema_sql).await {
            eprintln!("[Ring 1 / UAL] WARNING: Schema bootstrap failed (non-fatal): {}", e);
        }

        klog!("[Ring 1 / UAL] Linked to SurrealDB (HTTP) at {}", url);
        Ok(storage)
    }

    /// Execute raw SurrealQL and return results.
    pub async fn query(&self, sql: &str) -> Result<Vec<Vec<serde_json::Value>>, StorageError> {
        let resp = self.client
            .post(format!("{}/sql", self.url))
            .header("Accept", "application/json")
            .header("surreal-ns", &self.ns)
            .header("surreal-db", &self.db)
            .header("Authorization", &self.auth)
            .body(sql.to_string())
            .send()
            .await
            .map_err(|e| StorageError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(StorageError::QueryFailed(format!("HTTP {}: {}", status, body)));
        }

        let results: Vec<SurrealResponse> = resp.json().await
            .map_err(|e| StorageError::QueryFailed(format!("JSON parse error: {}", e)))?;

        // Check for SurrealDB-level errors (status: "ERR")
        for r in &results {
            if r.status.as_deref() == Some("ERR") {
                let msg = r.result.as_ref()
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown SurrealDB error");
                return Err(StorageError::QueryFailed(msg.to_string()));
            }
        }

        Ok(results.into_iter().map(|r| {
            match r.result {
                Some(serde_json::Value::Array(arr)) => arr,
                Some(val) => vec![val],
                None => Vec::new(),
            }
        }).collect())
    }

    /// Execute a single-statement query and return first result set.
    pub async fn query_one(&self, sql: &str) -> Result<Vec<serde_json::Value>, StorageError> {
        let mut results = self.query(sql).await?;
        Ok(results.pop().unwrap_or_default())
    }
}

// ── INPUT VALIDATION ──────────────────────────────────────────

/// Validate IDs: alphanumeric, hyphens, underscores only. Max 128 chars.
fn sanitize_id(id: &str) -> Result<&str, StorageError> {
    if id.is_empty() || id.len() > 128 {
        return Err(StorageError::QueryFailed("ID must be 1-128 characters".into()));
    }
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(StorageError::QueryFailed("ID contains invalid characters".into()));
    }
    Ok(id)
}

/// Escape string for SurrealQL string literals.
/// Handles: backslashes, single quotes, null bytes, newlines, carriage returns, tabs.
fn escape_surreal(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('\'', "\\'")
     .replace('\0', "")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}

/// Clamp query limit to prevent resource exhaustion.
fn safe_limit(limit: u32) -> u32 {
    limit.min(100)
}

// ── VERDICT SERIALIZATION ────────────────────────────────────

fn verdict_to_sql(v: &Verdict) -> String {
    let escape = |s: &str| escape_surreal(s);

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
        anomaly_detected: false,
        max_disagreement: 0.0,
        anomaly_axiom: None,
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
            "UPSERT crystal:{} SET content = '{}', domain = '{}', confidence = {}, observations = {}, state = '{}', created_at = '{}', updated_at = '{}'",
            escape(&crystal.id), escape(&crystal.content), escape(&crystal.domain),
            crystal.confidence, crystal.observations, state_str,
            escape(&crystal.created_at), escape(&crystal.updated_at)
        );
        self.query_one(&sql).await?;
        Ok(())
    }

    async fn get_crystal(&self, id: &str) -> Result<Option<Crystal>, StorageError> {
        let id = sanitize_id(id)?;
        let sql = format!("SELECT * FROM crystal:{}", id);
        let rows = self.query_one(&sql).await?;
        Ok(rows.first().map(row_to_crystal))
    }

    async fn list_crystals(&self, limit: u32) -> Result<Vec<Crystal>, StorageError> {
        let sql = format!("SELECT * FROM crystal ORDER BY observations DESC LIMIT {}", safe_limit(limit));
        let rows = self.query_one(&sql).await?;
        Ok(rows.iter().map(row_to_crystal).collect())
    }

    async fn observe_crystal(&self, id: &str, content: &str, domain: &str, score: f64, timestamp: &str) -> Result<(), StorageError> {
        let escape = |s: &str| escape_surreal(s);
        // Atomic UPSERT: if crystal exists, update running mean + increment obs.
        // If not, create with initial values. All in one SurrealQL statement.
        // Running mean: new_conf = (old_conf * old_obs + score) / (old_obs + 1)
        // State classification thresholds: PHI_INV=0.618, PHI_INV2=0.382, MIN_CYCLES=21, CANONICAL=233
        let sql = format!(
            "UPSERT crystal:{id} SET \
                content = '{content}', \
                domain = '{domain}', \
                observations = IF observations THEN observations + 1 ELSE 1 END, \
                confidence = IF confidence THEN (confidence * observations + {score}) / (observations + 1) ELSE {score} END, \
                state = IF (IF confidence THEN (confidence * observations + {score}) / (observations + 1) ELSE {score} END) < 0.381966 THEN \
                    IF (IF observations THEN observations + 1 ELSE 1 END) > 21 THEN 'decaying' ELSE 'dissolved' END \
                ELSE IF (IF observations THEN observations + 1 ELSE 1 END) >= 233 AND (IF confidence THEN (confidence * observations + {score}) / (observations + 1) ELSE {score} END) >= 0.618034 THEN 'canonical' \
                ELSE IF (IF observations THEN observations + 1 ELSE 1 END) >= 21 AND (IF confidence THEN (confidence * observations + {score}) / (observations + 1) ELSE {score} END) >= 0.618034 THEN 'crystallized' \
                ELSE 'forming' END END END, \
                created_at = IF created_at THEN created_at ELSE '{ts}' END, \
                updated_at = '{ts}';",
            id = escape(id),
            content = escape(content),
            domain = escape(domain),
            score = score,
            ts = escape(timestamp),
        );
        self.query_one(&sql).await?;
        Ok(())
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
        let check_sql = format!(
            "SELECT * FROM work_claim WHERE target = '{}' AND agent_id != '{}' AND active = true;",
            escape_surreal(target), escape_surreal(agent_id)
        );
        let conflicts = self.query_one(&check_sql).await
            .map_err(|e| CoordError::StorageFailed(format!("Claim check: {}", e)))?;
        if !conflicts.is_empty() {
            let infos = conflicts.iter().map(|c| ConflictInfo {
                agent_id: c["agent_id"].as_str().unwrap_or("?").to_string(),
                claimed_at: c["claimed_at"].as_str().unwrap_or("?").to_string(),
            }).collect();
            return Ok(ClaimResult::Conflict(infos));
        }
        let claim_id = format!("{}_{}", agent_id, target.replace(['/', '.'], "_"));
        let sql = format!(
            "UPSERT work_claim:`{claim_id_key}` SET \
                agent_id = '{agent_id}', target = '{target}', claim_type = '{claim_type}', \
                claimed_at = time::now(), active = true;",
            claim_id_key = sanitize_record_id(&claim_id),
            agent_id = escape_surreal(agent_id),
            target = escape_surreal(target),
            claim_type = escape_surreal(claim_type),
        );
        self.query_one(&sql).await.map_err(|e| CoordError::StorageFailed(format!("Claim: {}", e)))?;
        let _ = self.heartbeat(agent_id).await;
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
        if target.is_none() { let _ = self.deactivate_agent(agent_id).await; }
        Ok(desc)
    }

    async fn who(&self, agent_id_filter: Option<&str>) -> Result<CoordSnapshot, CoordError> {
        let _ = self.query_one("UPDATE agent_session SET active = false WHERE active = true AND (time::now() - last_seen) > 5m;").await;
        let _ = self.query_one("UPDATE work_claim SET active = false WHERE active = true AND agent_id NOT IN (SELECT VALUE agent_id FROM agent_session WHERE active = true);").await;
        let session_sql = match agent_id_filter {
            Some(id) => format!("SELECT * FROM agent_session WHERE agent_id = '{}';", escape_surreal(id)),
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
        let query = format!(
            "CREATE mcp_audit SET ts = time::now(), tool = '{}', agent_id = '{}', details = '{}';\
             DELETE mcp_audit WHERE id NOT IN (SELECT VALUE id FROM mcp_audit ORDER BY ts DESC LIMIT 10000);",
            escape_surreal(tool), escape_surreal(agent_id), safe_details,
        );
        let _ = self.query(&query).await;
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
        let _ = self.query_one(&format!("UPDATE agent_session:`{}` SET last_seen = time::now();", sanitize_record_id(agent_id))).await;
        Ok(())
    }

    async fn deactivate_agent(&self, agent_id: &str) -> Result<(), CoordError> {
        let _ = self.query_one(&format!("UPDATE agent_session:`{}` SET active = false, last_seen = time::now();", sanitize_record_id(agent_id))).await;
        Ok(())
    }
}

// ── TESTS ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dog::{AxiomReasoning, VerdictKind, QScore};

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
        // SQL injection attempt
        assert!(sanitize_id("'; DROP TABLE verdict; --").is_err());
        // Null bytes
        assert!(sanitize_id("abc\0def").is_err());
        // Backticks
        assert!(sanitize_id("abc`def").is_err());
        // Semicolons
        assert!(sanitize_id("abc;def").is_err());
        // Spaces
        assert!(sanitize_id("abc def").is_err());
        // Empty
        assert!(sanitize_id("").is_err());
        // Too long (129 chars)
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
        assert_eq!(escape_surreal("a\0b"), "ab"); // null stripped
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
