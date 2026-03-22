//! HTTP Storage Adapter — connects to SurrealDB 3.x via POST /sql.
//! Replaces the surrealdb crate (which has compilation bugs with 3.x)
//! with direct HTTP calls using reqwest. Sovereign and dependency-light.
//!
//! SurrealDB HTTP API:
//!   POST /sql  — raw SurrealQL, headers: surreal-ns, surreal-db, Authorization: Basic
//!   Response: JSON array of statement results

pub mod surreal;

use reqwest::Client;
use serde::Deserialize;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::domain::storage::StorageError;

/// Slow query threshold — anything above this logs a warning.
const SLOW_QUERY_MS: u64 = 100;

/// HTTP-based SurrealDB client. No surrealdb crate needed.
pub struct SurrealHttpStorage {
    pub(crate) client: Client,
    pub(crate) url: String,
    pub(crate) ns: String,
    pub(crate) db: String,
    pub(crate) auth: String, // "Basic base64(user:pass)"
    // ── Storage observability (T1/T4 from crystallization) ──
    pub(crate) metrics: StorageMetrics,
}

/// Atomic counters for storage observability. Zero-cost when not read.
pub struct StorageMetrics {
    pub queries: AtomicU64,
    pub errors: AtomicU64,
    pub total_latency_us: AtomicU64,
    pub slow_queries: AtomicU64,
    pub started_at: std::time::Instant,
}

impl StorageMetrics {
    fn new() -> Self {
        Self {
            queries: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            slow_queries: AtomicU64::new(0),
            started_at: std::time::Instant::now(),
        }
    }

    /// Snapshot for /health — lock-free read.
    pub fn snapshot(&self) -> StorageMetricsSnapshot {
        let queries = self.queries.load(Ordering::Relaxed);
        let total_us = self.total_latency_us.load(Ordering::Relaxed);
        StorageMetricsSnapshot {
            queries,
            errors: self.errors.load(Ordering::Relaxed),
            slow_queries: self.slow_queries.load(Ordering::Relaxed),
            avg_latency_ms: if queries > 0 { (total_us / queries) as f64 / 1000.0 } else { 0.0 },
            uptime_secs: self.started_at.elapsed().as_secs(),
        }
    }
}

#[derive(serde::Serialize)]
pub struct StorageMetricsSnapshot {
    pub queries: u64,
    pub errors: u64,
    pub slow_queries: u64,
    pub avg_latency_ms: f64,
    pub uptime_secs: u64,
}

#[derive(Deserialize, Debug)]
pub(crate) struct SurrealResponse {
    pub(crate) result: Option<serde_json::Value>,
    pub(crate) status: Option<String>,
}

impl SurrealHttpStorage {
    /// Expose database name for test teardown and /health introspection.
    pub fn db_name(&self) -> &str {
        &self.db
    }

    /// Expose namespace for /health introspection.
    pub fn ns_name(&self) -> &str {
        &self.ns
    }

    pub async fn init(config: &crate::infra::config::StorageConfig) -> Result<Self, StorageError> {
        Self::init_with(&config.url, &config.namespace, &config.database).await
    }

    pub async fn init_with(url: &str, ns: &str, db: &str) -> Result<Self, StorageError> {
        let user = std::env::var("SURREALDB_USER").unwrap_or_else(|_| "root".to_string());
        let pass = std::env::var("SURREALDB_PASS")
            .map_err(|_| StorageError::ConnectionFailed("SURREALDB_PASS must be set".into()))?;

        use base64::Engine;
        let credentials = format!("{}:{}", user, pass);
        let auth = format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(&credentials));

        let storage = Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client for SurrealDB"),
            url: url.trim_end_matches('/').to_string(),
            ns: ns.to_string(),
            db: db.to_string(),
            auth,

            metrics: StorageMetrics::new(),
        };

        // Bootstrap namespace and database (SurrealDB 3.x doesn't auto-create)
        let bootstrap = Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .expect("Failed to build bootstrap HTTP client"),
            url: storage.url.clone(),
            ns: String::new(),
            db: String::new(),
            auth: storage.auth.clone(),

            metrics: StorageMetrics::new(),
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
            DEFINE INDEX IF NOT EXISTS crystal_vec_idx ON crystal FIELDS embedding HNSW DIMENSION 1024 DIST COSINE TYPE F32;\
            DEFINE FIELD IF NOT EXISTS project ON observation TYPE string;\
            DEFINE FIELD IF NOT EXISTS agent_id ON observation TYPE string;\
            DEFINE FIELD IF NOT EXISTS tool ON observation TYPE string;\
            DEFINE FIELD IF NOT EXISTS target ON observation TYPE string;\
            DEFINE FIELD IF NOT EXISTS domain ON observation TYPE string;\
            DEFINE FIELD IF NOT EXISTS status ON observation TYPE string;\
            DEFINE FIELD IF NOT EXISTS context ON observation TYPE string;\
            DEFINE FIELD IF NOT EXISTS session_id ON observation TYPE string;\
            DEFINE FIELD IF NOT EXISTS created_at ON observation TYPE datetime;\
            DEFINE INDEX IF NOT EXISTS obs_project_idx ON observation FIELDS project;\
            DEFINE INDEX IF NOT EXISTS obs_domain_idx ON observation FIELDS domain;\
            DEFINE INDEX IF NOT EXISTS obs_target_idx ON observation FIELDS target;\
            DEFINE INDEX IF NOT EXISTS obs_created_idx ON observation FIELDS created_at;\
            DEFINE INDEX IF NOT EXISTS obs_agent_idx ON observation FIELDS agent_id;\
            DEFINE FIELD IF NOT EXISTS agent_id ON agent_session TYPE string;\
            DEFINE FIELD IF NOT EXISTS agent_type ON agent_session TYPE string;\
            DEFINE FIELD IF NOT EXISTS intent ON agent_session TYPE string;\
            DEFINE FIELD IF NOT EXISTS registered_at ON agent_session TYPE datetime;\
            DEFINE FIELD IF NOT EXISTS last_seen ON agent_session TYPE datetime;\
            DEFINE FIELD IF NOT EXISTS active ON agent_session TYPE bool;\
            DEFINE INDEX IF NOT EXISTS agent_session_active_idx ON agent_session FIELDS active;\
            DEFINE FIELD IF NOT EXISTS agent_id ON work_claim TYPE string;\
            DEFINE FIELD IF NOT EXISTS target ON work_claim TYPE string;\
            DEFINE FIELD IF NOT EXISTS claim_type ON work_claim TYPE string;\
            DEFINE FIELD IF NOT EXISTS claimed_at ON work_claim TYPE datetime;\
            DEFINE FIELD IF NOT EXISTS active ON work_claim TYPE bool;\
            DEFINE INDEX IF NOT EXISTS work_claim_active_idx ON work_claim FIELDS active;\
            DEFINE INDEX IF NOT EXISTS work_claim_target_idx ON work_claim FIELDS target;\
            DEFINE FIELD IF NOT EXISTS ts ON mcp_audit TYPE datetime;\
            DEFINE FIELD IF NOT EXISTS tool ON mcp_audit TYPE string;\
            DEFINE FIELD IF NOT EXISTS agent_id ON mcp_audit TYPE string;\
            DEFINE FIELD IF NOT EXISTS details ON mcp_audit TYPE string;\
            DEFINE INDEX IF NOT EXISTS mcp_audit_ts_idx ON mcp_audit FIELDS ts;\
            DEFINE FIELD IF NOT EXISTS dog_id ON dog_usage TYPE string;\
            DEFINE FIELD IF NOT EXISTS prompt_tokens ON dog_usage TYPE int;\
            DEFINE FIELD IF NOT EXISTS completion_tokens ON dog_usage TYPE int;\
            DEFINE FIELD IF NOT EXISTS requests ON dog_usage TYPE int;\
            DEFINE FIELD IF NOT EXISTS failures ON dog_usage TYPE int;\
            DEFINE FIELD IF NOT EXISTS total_latency_ms ON dog_usage TYPE int;\
            DEFINE FIELD IF NOT EXISTS updated_at ON dog_usage TYPE datetime;\
            DEFINE INDEX IF NOT EXISTS dog_usage_id_idx ON dog_usage FIELDS dog_id UNIQUE;\
            DEFINE FIELD IF NOT EXISTS session_id ON session_summary TYPE string;\
            DEFINE FIELD IF NOT EXISTS agent_id ON session_summary TYPE string;\
            DEFINE FIELD IF NOT EXISTS summary ON session_summary TYPE string;\
            DEFINE FIELD IF NOT EXISTS observations_count ON session_summary TYPE int;\
            DEFINE FIELD IF NOT EXISTS created_at ON session_summary TYPE datetime;\
            DEFINE INDEX IF NOT EXISTS session_summary_session_idx ON session_summary FIELDS session_id UNIQUE;\
            DEFINE INDEX IF NOT EXISTS session_summary_created_idx ON session_summary FIELDS created_at;\
        ";
        if let Err(e) = storage.query(schema_sql).await {
            // CRITICAL, not WARNING: HNSW vector index (crystal_vec_idx) is in this batch.
            // Without it, search_crystals_semantic fails silently and the crystal feedback
            // loop falls back to list_crystals_for_domain (wrong crystals, no semantic match).
            // We continue because the DB may have a prior schema — but this MUST be investigated.
            tracing::error!(error = %e, "schema bootstrap failed — vector search may not work");
        }

        klog!("[Ring 1 / UAL] Linked to SurrealDB (HTTP) at {}", url);
        Ok(storage)
    }

    /// Execute raw SurrealQL and return results.
    pub async fn query(&self, sql: &str) -> Result<Vec<Vec<serde_json::Value>>, StorageError> {
        let start = std::time::Instant::now();
        self.metrics.queries.fetch_add(1, Ordering::Relaxed);

        let result = self.query_inner(sql).await;

        let elapsed_us = start.elapsed().as_micros() as u64;
        self.metrics.total_latency_us.fetch_add(elapsed_us, Ordering::Relaxed);

        let elapsed_ms = elapsed_us / 1000;
        if elapsed_ms > SLOW_QUERY_MS {
            self.metrics.slow_queries.fetch_add(1, Ordering::Relaxed);
            let preview: String = sql.chars().take(80).collect();
            tracing::warn!(latency_ms = elapsed_ms, query = %preview, "slow query");
        }

        if result.is_err() {
            self.metrics.errors.fetch_add(1, Ordering::Relaxed);
        }

        result
    }

    /// Inner query with retry on transient 401 (SurrealDB 3.x bug under rapid sequential writes).
    async fn query_inner(&self, sql: &str) -> Result<Vec<Vec<serde_json::Value>>, StorageError> {
        let mut last_err = String::new();
        for attempt in 0..3u64 {
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

            if resp.status().as_u16() == 401 && attempt < 2 {
                // SurrealDB 3.x intermittent 401 under rapid writes — retry with backoff
                last_err = resp.text().await.unwrap_or_default();
                tokio::time::sleep(std::time::Duration::from_millis(50 * (attempt + 1))).await;
                continue;
            }

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(StorageError::QueryFailed(format!("HTTP {}: {}", status, body)));
            }

            let results: Vec<SurrealResponse> = resp.json().await
                .map_err(|e| StorageError::QueryFailed(format!("JSON parse error: {}", e)))?;

            for r in &results {
                if r.status.as_deref() == Some("ERR") {
                    let msg = r.result.as_ref()
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown SurrealDB error");
                    return Err(StorageError::QueryFailed(msg.to_string()));
                }
            }

            return Ok(results.into_iter().map(|r| {
                match r.result {
                    Some(serde_json::Value::Array(arr)) => arr,
                    Some(val) => vec![val],
                    None => Vec::new(),
                }
            }).collect());
        }
        // All retries exhausted
        Err(StorageError::QueryFailed(format!("401 after 3 retries: {}", last_err)))
    }

    /// Execute a single-statement query and return first result set.
    pub async fn query_one(&self, sql: &str) -> Result<Vec<serde_json::Value>, StorageError> {
        let mut results = self.query(sql).await?;
        Ok(results.pop().unwrap_or_default())
    }
}

// ── INPUT VALIDATION ──────────────────────────────────────────

/// Validate IDs: alphanumeric, hyphens, underscores only. Max 128 chars.
pub(crate) fn sanitize_id(id: &str) -> Result<&str, StorageError> {
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
pub(crate) fn escape_surreal(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('\'', "\\'")
     .replace('\0', "")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}

/// Clamp query limit to prevent resource exhaustion.
pub(crate) fn safe_limit(limit: u32) -> u32 {
    limit.min(100)
}
