//! HTTP Storage Adapter — connects to SurrealDB 3.x via POST /sql.
//! Replaces the surrealdb crate (which has compilation bugs with 3.x)
//! with direct HTTP calls using reqwest. Sovereign and dependency-light.
//!
//! SurrealDB HTTP API:
//!   POST /sql  — raw SurrealQL, headers: surreal-ns, surreal-db, Authorization: Basic
//!   Response: JSON array of statement results

use reqwest::Client;
use serde::Deserialize;
use crate::dog::{Verdict, VerdictKind, QScore, AxiomReasoning};
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

        println!("[Ring 1 / UAL] Linked to SurrealDB (HTTP) at {}", url);
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

// ── VERDICT SERIALIZATION ────────────────────────────────────

fn verdict_to_sql(v: &Verdict) -> String {
    let escape = |s: &str| s.replace('\\', "\\\\").replace('\'', "\\'");

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

// ── STORAGE PORT IMPLEMENTATION ──────────────────────────────

#[async_trait::async_trait]
impl StoragePort for SurrealHttpStorage {
    async fn store_verdict(&self, verdict: &Verdict) -> Result<(), StorageError> {
        let sql = verdict_to_sql(verdict);
        self.query_one(&sql).await?;
        Ok(())
    }

    async fn get_verdict(&self, id: &str) -> Result<Option<Verdict>, StorageError> {
        let escaped_id = id.replace('\'', "\\'");
        let sql = format!("SELECT * FROM verdict WHERE verdict_id = '{}' LIMIT 1", escaped_id);
        let rows = self.query_one(&sql).await?;
        Ok(rows.first().map(row_to_verdict))
    }

    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError> {
        let sql = format!("SELECT * FROM verdict ORDER BY created_at DESC LIMIT {}", limit);
        let rows = self.query_one(&sql).await?;
        Ok(rows.iter().map(row_to_verdict).collect())
    }
}

// ── COGNITIVE MEMORY (gRPC service) via HTTP ─────────────────

use std::sync::Arc;
use tonic::{Request, Response, Status};
use crate::cynic_v2::cognitive_memory_server::CognitiveMemory;
use crate::cynic_v2::{Fact, PublishAck, TrustEntry, TrustVerifyResponse};

pub struct CognitiveMemoryService {
    storage: Option<Arc<SurrealHttpStorage>>,
}

impl CognitiveMemoryService {
    pub fn new(storage: Option<Arc<SurrealHttpStorage>>) -> Self {
        Self { storage }
    }

    #[allow(clippy::result_large_err)]
    fn require_storage(&self) -> Result<&Arc<SurrealHttpStorage>, Status> {
        self.storage.as_ref().ok_or_else(|| Status::unavailable("Storage unavailable (DEGRADED mode)"))
    }
}

#[tonic::async_trait]
impl CognitiveMemory for CognitiveMemoryService {
    async fn store_fact(
        &self,
        request: Request<Fact>,
    ) -> Result<Response<PublishAck>, Status> {
        let storage = self.require_storage()?;
        let fact = request.into_inner();
        let meta = fact.meta.clone();
        let node_id = meta.as_ref().map(|m| m.node_id.clone()).unwrap_or_default();
        let trace_id = meta.as_ref().map(|m| m.trace_id.clone()).unwrap_or_default();

        let escape = |s: &str| s.replace('\\', "\\\\").replace('\'', "\\'");
        let sql = format!(
            "CREATE fact SET agent = '{}', content = '{}', confidence = {}, trace_id = '{}', timestamp = time::now()",
            escape(&node_id), escape(&fact.content), fact.confidence, escape(&trace_id)
        );

        storage.query_one(&sql).await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(PublishAck {
            meta,
            success: true,
        }))
    }

    async fn register_trust(
        &self,
        request: Request<TrustEntry>,
    ) -> Result<Response<PublishAck>, Status> {
        let storage = self.require_storage()?;
        let entry = request.into_inner();
        let meta = entry.meta.clone();
        let trace_id = meta.as_ref().map(|m| m.trace_id.clone()).unwrap_or_default();

        let escape = |s: &str| s.replace('\\', "\\\\").replace('\'', "\\'");
        let sql = format!(
            "UPSERT trusted_model:['{name}'] SET sha256 = '{sha}', last_verified = time::now(), trace_id = '{trace}'",
            name = escape(&entry.model_name),
            sha = escape(&entry.sha256),
            trace = escape(&trace_id)
        );

        storage.query_one(&sql).await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(PublishAck {
            meta,
            success: true,
        }))
    }

    async fn verify_trust(
        &self,
        request: Request<TrustEntry>,
    ) -> Result<Response<TrustVerifyResponse>, Status> {
        let storage = self.require_storage()?;
        let entry = request.into_inner();
        let meta = entry.meta.clone();

        let escape = |s: &str| s.replace('\\', "\\\\").replace('\'', "\\'");
        let sql = format!(
            "SELECT * FROM trusted_model WHERE id = trusted_model:['{name}'] AND sha256 = '{sha}'",
            name = escape(&entry.model_name),
            sha = escape(&entry.sha256)
        );

        let results = storage.query_one(&sql).await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(TrustVerifyResponse {
            meta,
            is_trusted: !results.is_empty(),
        }))
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
