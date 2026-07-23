//! CoordPort — domain contract for agent coordination and audit.
//! Covers agent sessions, work claims, and MCP audit trail.
//! SurrealHttpStorage implements this. REST and MCP use Arc<dyn CoordPort>.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::lease_proof::ValidatedLeaseProof;

pub use cynic_protocol::{
    BatchClaimResult, LeaseAction, LeaseConflict as ConflictInfo, LeaseGrant as ClaimGrant,
    LeaseGuard, LeaseMutationOutcome as LeaseMutationResult, TargetClaimOutcome,
};

#[derive(Debug, thiserror::Error)]
pub enum CoordError {
    #[error("Coordination storage failed: {0}")]
    StorageFailed(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClaimResult {
    Claimed(ClaimGrant),
    Conflict(Vec<ConflictInfo>),
}

/// Agent session info returned by who().
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: String,
    #[serde(default)]
    pub agent_type: String,
    #[serde(default)]
    pub intent: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub registered_at: String,
    #[serde(default)]
    pub last_seen: String,
    /// SurrealDB record ID — preserved for API compatibility.
    #[serde(default)]
    pub id: String,
}

/// Work claim entry returned by who().
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimEntry {
    pub agent_id: String,
    pub target: String,
    #[serde(default)]
    pub claim_type: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub claimed_at: String,
    #[serde(default)]
    pub lease_id: String,
    #[serde(default)]
    pub fencing_token: u64,
    #[serde(default)]
    pub expires_at: String,
    #[serde(default)]
    pub id: String,
}

/// MCP audit trail entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    #[serde(default)]
    pub ts: String,
    pub tool: String,
    pub agent_id: String,
    #[serde(default)]
    pub details: String,
    #[serde(default)]
    pub id: String,
}

/// Validate an agent_id: 1-64 characters, no control chars.
/// Returns the error message on failure — callers map to their own error type.
pub fn validate_agent_id(id: &str) -> Result<(), &'static str> {
    if id.is_empty() || id.chars().count() > 64 {
        return Err("agent_id must be 1-64 characters");
    }
    if id.chars().any(|c| c.is_control()) {
        return Err("agent_id contains invalid characters");
    }
    Ok(())
}

/// Serializable summary of coordination state — used by both REST and MCP.
#[derive(Debug, Serialize)]
pub struct CoordSummary {
    pub active_agents: usize,
    pub active_claims: usize,
    pub agents: Vec<AgentInfo>,
    pub claims: Vec<ClaimEntry>,
}

#[derive(Debug)]
pub struct CoordSnapshot {
    pub agents: Vec<AgentInfo>,
    pub claims: Vec<ClaimEntry>,
}

impl CoordSnapshot {
    /// Convert to a serializable summary with computed counts.
    pub fn into_summary(self) -> CoordSummary {
        CoordSummary {
            active_agents: self.agents.len(),
            active_claims: self.claims.len(),
            agents: self.agents,
            claims: self.claims,
        }
    }
}

#[async_trait]
pub trait CoordPort: Send + Sync {
    async fn register_agent(
        &self,
        agent_id: &str,
        agent_type: &str,
        intent: &str,
    ) -> Result<(), CoordError>;
    async fn claim(
        &self,
        agent_id: &str,
        target: &str,
        claim_type: &str,
    ) -> Result<ClaimResult, CoordError>;
    async fn renew_lease(&self, guard: &LeaseGuard) -> Result<LeaseMutationResult, CoordError>;
    async fn release(&self, guard: &LeaseGuard) -> Result<LeaseMutationResult, CoordError>;
    async fn finalize_lease(
        &self,
        guard: &LeaseGuard,
        proof: &ValidatedLeaseProof,
    ) -> Result<LeaseMutationResult, CoordError>;
    async fn who(&self, agent_id_filter: Option<&str>) -> Result<CoordSnapshot, CoordError>;
    async fn store_audit(
        &self,
        tool: &str,
        agent_id: &str,
        details: &str,
    ) -> Result<(), CoordError>;
    async fn query_audit(
        &self,
        tool_filter: Option<&str>,
        agent_filter: Option<&str>,
        since: Option<&str>,
        until: Option<&str>,
        limit: u32,
    ) -> Result<Vec<AuditEntry>, CoordError>;
    async fn heartbeat(&self, agent_id: &str) -> Result<(), CoordError>;
    async fn deactivate_agent(&self, agent_id: &str) -> Result<(), CoordError>;
    /// Expire stale sessions (>5 min no heartbeat) and their orphaned claims.
    /// Lighter than who() — no SELECT, just the two UPDATEs.
    async fn expire_stale(&self) -> Result<(), CoordError>;
    /// Update the task scope for an existing active session.
    /// Preserves registered_at — does NOT reset the session clock.
    async fn scope_update(&self, agent_id: &str, scope: &str) -> Result<(), CoordError>;

    /// Claim multiple targets at once. Default impl calls claim() in a loop.
    /// SurrealHttpStorage overrides with a single SQL transaction.
    async fn claim_batch(
        &self,
        agent_id: &str,
        targets: &[String],
        claim_type: &str,
    ) -> Result<BatchClaimResult, CoordError> {
        let mut result = BatchClaimResult::default();
        for target in targets {
            match self.claim(agent_id, target, claim_type).await? {
                ClaimResult::Claimed(grant) => result.outcomes.push(TargetClaimOutcome::Claimed {
                    target: target.clone(),
                    grant,
                }),
                ClaimResult::Conflict(conflicts) => {
                    result.outcomes.push(TargetClaimOutcome::Conflict {
                        target: target.clone(),
                        conflicts,
                    })
                }
            }
        }
        Ok(result)
    }
}

#[derive(Debug)]
pub struct NullCoord;

#[async_trait]
impl CoordPort for NullCoord {
    async fn register_agent(&self, _: &str, _: &str, _: &str) -> Result<(), CoordError> {
        Ok(())
    }
    async fn claim(&self, _: &str, _: &str, _: &str) -> Result<ClaimResult, CoordError> {
        Ok(ClaimResult::Claimed(ClaimGrant {
            lease_id: "degraded-no-authority".into(),
            fencing_token: 1,
            expires_at: String::new(),
        }))
    }
    async fn renew_lease(&self, guard: &LeaseGuard) -> Result<LeaseMutationResult, CoordError> {
        Ok(LeaseMutationResult::applied(
            LeaseAction::Renew,
            &guard.target,
            guard.fencing_token,
            Some(String::new()),
            None,
        ))
    }
    async fn release(&self, guard: &LeaseGuard) -> Result<LeaseMutationResult, CoordError> {
        Ok(LeaseMutationResult::applied(
            LeaseAction::Release,
            &guard.target,
            guard.fencing_token,
            None,
            None,
        ))
    }
    async fn finalize_lease(
        &self,
        guard: &LeaseGuard,
        proof: &ValidatedLeaseProof,
    ) -> Result<LeaseMutationResult, CoordError> {
        Ok(LeaseMutationResult::applied(
            LeaseAction::Finalize,
            &guard.target,
            guard.fencing_token,
            None,
            Some(proof.proof_ref().into()),
        ))
    }
    async fn who(&self, _: Option<&str>) -> Result<CoordSnapshot, CoordError> {
        Ok(CoordSnapshot {
            agents: vec![],
            claims: vec![],
        })
    }
    async fn store_audit(&self, _: &str, _: &str, _: &str) -> Result<(), CoordError> {
        Ok(())
    }
    async fn query_audit(
        &self,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<&str>,
        _: u32,
    ) -> Result<Vec<AuditEntry>, CoordError> {
        Ok(vec![])
    }
    async fn heartbeat(&self, _: &str) -> Result<(), CoordError> {
        Ok(())
    }
    async fn deactivate_agent(&self, _: &str) -> Result<(), CoordError> {
        Ok(())
    }
    async fn expire_stale(&self) -> Result<(), CoordError> {
        Ok(())
    }
    async fn scope_update(&self, _: &str, _: &str) -> Result<(), CoordError> {
        Ok(())
    }
}

/// Production degraded adapter. Unlike `NullCoord`, it never fabricates
/// coordination success when the durable store is unavailable.
#[derive(Debug)]
pub struct UnavailableCoord {
    reason: String,
}

impl UnavailableCoord {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    fn error(&self) -> CoordError {
        CoordError::StorageFailed(format!("coordination unavailable: {}", self.reason))
    }
}

#[async_trait]
impl CoordPort for UnavailableCoord {
    async fn register_agent(&self, _: &str, _: &str, _: &str) -> Result<(), CoordError> {
        Err(self.error())
    }

    async fn claim(&self, _: &str, _: &str, _: &str) -> Result<ClaimResult, CoordError> {
        Err(self.error())
    }

    async fn renew_lease(&self, _: &LeaseGuard) -> Result<LeaseMutationResult, CoordError> {
        Err(self.error())
    }
    async fn release(&self, _: &LeaseGuard) -> Result<LeaseMutationResult, CoordError> {
        Err(self.error())
    }
    async fn finalize_lease(
        &self,
        _: &LeaseGuard,
        _: &ValidatedLeaseProof,
    ) -> Result<LeaseMutationResult, CoordError> {
        Err(self.error())
    }

    async fn who(&self, _: Option<&str>) -> Result<CoordSnapshot, CoordError> {
        Err(self.error())
    }

    async fn store_audit(&self, _: &str, _: &str, _: &str) -> Result<(), CoordError> {
        Err(self.error())
    }

    async fn query_audit(
        &self,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<&str>,
        _: u32,
    ) -> Result<Vec<AuditEntry>, CoordError> {
        Err(self.error())
    }

    async fn heartbeat(&self, _: &str) -> Result<(), CoordError> {
        Err(self.error())
    }

    async fn deactivate_agent(&self, _: &str) -> Result<(), CoordError> {
        Err(self.error())
    }

    async fn expire_stale(&self) -> Result<(), CoordError> {
        Err(self.error())
    }

    async fn scope_update(&self, _: &str, _: &str) -> Result<(), CoordError> {
        Err(self.error())
    }
}
