//! Ports: Abstract interfaces for SurrealDB interactions.

use crate::domain::memory::EpistemicFactRecord;
use crate::domain::session::AgentSessionRecord;
use crate::domain::topology::TopologicalContext;

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

pub trait SurrealStoragePort: Send + Sync {
    fn check_health(&self) -> Result<SurrealHealthReport, String>;
    fn execute_sql(&self, query: &str) -> Result<Vec<serde_json::Value>, String>;
    fn save_session(&self, session: &AgentSessionRecord) -> Result<(), String>;
    fn save_fact(&self, fact: &EpistemicFactRecord) -> Result<(), String>;
    fn fetch_topological_context(
        &self,
        chunk_ids: &[String],
    ) -> Result<std::collections::HashMap<String, TopologicalContext>, String>;
}
