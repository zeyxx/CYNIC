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
use crate::domain::storage::StorageError;

/// HTTP-based SurrealDB client. No surrealdb crate needed.
pub struct SurrealHttpStorage {
    pub(crate) client: Client,
    pub(crate) url: String,
    pub(crate) ns: String,
    pub(crate) db: String,
    pub(crate) auth: String, // "Basic base64(user:pass)"
}

#[derive(Deserialize, Debug)]
pub(crate) struct SurrealResponse {
    pub(crate) result: Option<serde_json::Value>,
    #[allow(dead_code)]
    pub(crate) status: Option<String>,
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
