//! Imperative Shell / Adapter: Native HTTP/REST client adapter for SurrealDB.
//!
//! Side-effects, network I/O, and secret resolution are strictly confined here.

use reqwest::blocking::Client;
use std::time::Duration;

use crate::domain::memory::EpistemicFactRecord;
use crate::domain::query::SurrealQuery;
use crate::domain::response::{SurrealQueryResult, parse_and_validate_results};
use crate::domain::session::AgentSessionRecord;
use crate::ports::{SurrealHealthReport, SurrealStoragePort};

/// Connection configuration for SurrealDB.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SurrealConfig {
    pub endpoint: String,
    pub namespace: String,
    pub database: String,
    pub username: String,
}

impl Default for SurrealConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://10.10.10.103:8000".to_string(),
            namespace: "cynic".to_string(),
            database: "runtime".to_string(),
            username: "root".to_string(),
        }
    }
}

impl SurrealConfig {
    pub fn new(
        endpoint: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
        username: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            namespace: namespace.into(),
            database: database.into(),
            username: username.into(),
        }
    }
}

/// Thread-safe blocking HTTP client communicating with SurrealDB via REST.
#[derive(Debug, Clone)]
pub struct SurrealHttpClient {
    config: SurrealConfig,
    password: Option<String>,
    timeout: Duration,
}

impl SurrealHttpClient {
    /// Create a client resolving credentials dynamically via `cynic-secrets`.
    pub fn new(config: SurrealConfig) -> Self {
        let password = Self::resolve_password();
        Self {
            config,
            password,
            timeout: Duration::from_secs(5),
        }
    }

    /// Explicitly provide credentials (for testing or bounded caller contexts).
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Resolve runtime storage password through standard SSOT secret paths.
    fn resolve_password() -> Option<String> {
        cynic_secrets::resolve_secret("storage-runtime-password", "CYNIC_STORAGE_PASSWORD")
            .or_else(|| {
                cynic_secrets::resolve_secret("storage_runtime", "CYNIC_STORAGE_RUNTIME_PASSWORD")
            })
            .or_else(|| std::env::var("CYNIC_STORAGE_PASSWORD").ok())
            .or_else(|| std::env::var("SURREALDB_PASSWORD").ok())
    }

    fn http_client(&self) -> Result<Client, String> {
        Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| format!("failed to build HTTP client: {e}"))
    }
}

impl SurrealStoragePort for SurrealHttpClient {
    fn check_health(&self) -> Result<SurrealHealthReport, String> {
        let client = self.http_client()?;
        let ep = self.config.endpoint.trim_end_matches('/');

        // 1. Probe /health
        let health_url = format!("{ep}/health");
        let health_resp = client.get(&health_url).send();

        match health_resp {
            Ok(resp) if resp.status().is_success() => {
                // 2. Fetch /version
                let version_url = format!("{ep}/version");
                let version = client
                    .get(&version_url)
                    .send()
                    .ok()
                    .and_then(|r| r.text().ok())
                    .map(|v| v.trim().to_string());

                Ok(SurrealHealthReport::healthy(&self.config.endpoint, version))
            }
            Ok(resp) => Ok(SurrealHealthReport::unhealthy(
                &self.config.endpoint,
                format!("status {}", resp.status()),
            )),
            Err(e) => Ok(SurrealHealthReport::unhealthy(
                &self.config.endpoint,
                format!("network error: {e}"),
            )),
        }
    }

    fn execute_sql(&self, query: &str) -> Result<Vec<serde_json::Value>, String> {
        let client = self.http_client()?;
        let sql_url = format!("{}/sql", self.config.endpoint.trim_end_matches('/'));

        let mut req = client
            .post(&sql_url)
            .header("surreal-ns", &self.config.namespace)
            .header("surreal-db", &self.config.database)
            .header("Accept", "application/json")
            .body(query.to_string());

        if let Some(pw) = &self.password {
            req = req.basic_auth(&self.config.username, Some(pw));
        }

        let resp = req
            .send()
            .map_err(|e| format!("SurrealDB request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("SurrealDB HTTP {status}: {body}"));
        }

        let results: Vec<SurrealQueryResult> = resp
            .json()
            .map_err(|e| format!("Failed to parse SurrealDB response JSON: {e}"))?;

        // Delegate parsing & validation to Functional Core
        parse_and_validate_results(results)
    }

    fn save_session(&self, session: &AgentSessionRecord) -> Result<(), String> {
        let query = SurrealQuery::create_record(
            AgentSessionRecord::TABLE_NAME,
            Some(&session.session_id),
            session,
        )?;
        self.execute_sql(query.as_str())?;
        Ok(())
    }

    fn save_fact(&self, fact: &EpistemicFactRecord) -> Result<(), String> {
        let query = SurrealQuery::create_record(
            EpistemicFactRecord::TABLE_NAME,
            Some(&fact.fact_id),
            fact,
        )?;
        self.execute_sql(query.as_str())?;
        Ok(())
    }
}
