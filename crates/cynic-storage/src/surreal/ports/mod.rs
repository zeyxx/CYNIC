//! Ports: Abstract interfaces for SurrealDB interactions.

use crate::surreal::domain::memory::EpistemicFactRecord;
use crate::surreal::domain::session::AgentSessionRecord;

/// Structured report of SurrealDB health and connectivity.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SurrealHealthReport {
    pub schema: &'static str,
    pub healthy: bool,
    pub endpoint: String,
    pub version: Option<String>,
    pub status: String,
}

impl SurrealHealthReport {
    pub const SCHEMA: &'static str = "cynic.surreal.health_report.v1";

    pub fn healthy(endpoint: impl Into<String>, version: Option<String>) -> Self {
        Self {
            schema: Self::SCHEMA,
            healthy: true,
            endpoint: endpoint.into(),
            version,
            status: "pass".to_string(),
        }
    }

    pub fn unhealthy(endpoint: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            schema: Self::SCHEMA,
            healthy: false,
            endpoint: endpoint.into(),
            version: None,
            status: format!("fail: {}", reason.into()),
        }
    }
}

/// Port trait defining core interactions with SurrealDB.
pub trait SurrealStoragePort: Send + Sync {
    /// Probe health and connectivity of the remote SurrealDB instance.
    fn check_health(&self) -> Result<SurrealHealthReport, String>;

    /// Execute raw SurrealQL statement(s) against the configured namespace and database.
    fn execute_sql(&self, query: &str) -> Result<Vec<serde_json::Value>, String>;

    /// Save an agent session aggregate record.
    fn save_session(&self, session: &AgentSessionRecord) -> Result<(), String>;

    /// Save an epistemic fact record into associative memory.
    fn save_fact(&self, fact: &EpistemicFactRecord) -> Result<(), String>;
}
