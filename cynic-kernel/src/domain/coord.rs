//! CoordPort — domain contract for agent coordination and audit.
//! Covers agent sessions, work claims, and MCP audit trail.
//! SurrealHttpStorage implements this. REST and MCP use Arc<dyn CoordPort>.

use async_trait::async_trait;

#[derive(Debug)]
pub enum CoordError {
    StorageFailed(String),
    InvalidInput(String),
}

impl std::fmt::Display for CoordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StorageFailed(m) => write!(f, "Coordination storage failed: {}", m),
            Self::InvalidInput(m) => write!(f, "Invalid input: {}", m),
        }
    }
}

impl std::error::Error for CoordError {}

pub enum ClaimResult {
    Claimed,
    Conflict(Vec<ConflictInfo>),
}

/// Result of a batch claim — per-target outcome.
pub struct BatchClaimResult {
    pub claimed: Vec<String>,
    pub conflicts: Vec<(String, Vec<ConflictInfo>)>,
}

pub struct ConflictInfo {
    pub agent_id: String,
    pub claimed_at: String,
}

pub struct CoordSnapshot {
    pub agents: Vec<serde_json::Value>,
    pub claims: Vec<serde_json::Value>,
}

#[async_trait]
pub trait CoordPort: Send + Sync {
    async fn register_agent(&self, agent_id: &str, agent_type: &str, intent: &str) -> Result<(), CoordError>;
    async fn claim(&self, agent_id: &str, target: &str, claim_type: &str) -> Result<ClaimResult, CoordError>;
    async fn release(&self, agent_id: &str, target: Option<&str>) -> Result<String, CoordError>;
    async fn who(&self, agent_id_filter: Option<&str>) -> Result<CoordSnapshot, CoordError>;
    async fn store_audit(&self, tool: &str, agent_id: &str, details: &serde_json::Value) -> Result<(), CoordError>;
    async fn query_audit(&self, tool_filter: Option<&str>, agent_filter: Option<&str>, limit: u32) -> Result<Vec<serde_json::Value>, CoordError>;
    async fn heartbeat(&self, agent_id: &str) -> Result<(), CoordError>;
    async fn deactivate_agent(&self, agent_id: &str) -> Result<(), CoordError>;
    /// Expire stale sessions (>5 min no heartbeat) and their orphaned claims.
    /// Lighter than who() — no SELECT, just the two UPDATEs.
    async fn expire_stale(&self) -> Result<(), CoordError>;

    /// Claim multiple targets at once. Default impl calls claim() in a loop.
    /// SurrealHttpStorage overrides with a single SQL transaction.
    async fn claim_batch(&self, agent_id: &str, targets: &[String], claim_type: &str) -> Result<BatchClaimResult, CoordError> {
        let mut result = BatchClaimResult { claimed: Vec::new(), conflicts: Vec::new() };
        for target in targets {
            match self.claim(agent_id, target, claim_type).await? {
                ClaimResult::Claimed => result.claimed.push(target.clone()),
                ClaimResult::Conflict(infos) => result.conflicts.push((target.clone(), infos)),
            }
        }
        Ok(result)
    }
}

pub struct NullCoord;

#[async_trait]
impl CoordPort for NullCoord {
    async fn register_agent(&self, _: &str, _: &str, _: &str) -> Result<(), CoordError> { Ok(()) }
    async fn claim(&self, _: &str, _: &str, _: &str) -> Result<ClaimResult, CoordError> { Ok(ClaimResult::Claimed) }
    async fn release(&self, _: &str, _: Option<&str>) -> Result<String, CoordError> { Ok("Released (degraded mode)".into()) }
    async fn who(&self, _: Option<&str>) -> Result<CoordSnapshot, CoordError> { Ok(CoordSnapshot { agents: vec![], claims: vec![] }) }
    async fn store_audit(&self, _: &str, _: &str, _: &serde_json::Value) -> Result<(), CoordError> { Ok(()) }
    async fn query_audit(&self, _: Option<&str>, _: Option<&str>, _: u32) -> Result<Vec<serde_json::Value>, CoordError> { Ok(vec![]) }
    async fn heartbeat(&self, _: &str) -> Result<(), CoordError> { Ok(()) }
    async fn deactivate_agent(&self, _: &str) -> Result<(), CoordError> { Ok(()) }
    async fn expire_stale(&self) -> Result<(), CoordError> { Ok(()) }
}
