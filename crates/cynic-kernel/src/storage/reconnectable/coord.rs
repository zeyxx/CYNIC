//! Reconnectable wrapper for CoordPort.

use async_trait::async_trait;
use std::sync::{Arc, RwLock};

use crate::domain::coord::{
    AuditEntry, BatchClaimResult, ClaimResult, CoordError, CoordPort, CoordSnapshot,
};

/// CoordPort proxy — delegates all trait methods through an Arc<RwLock<Arc<dyn CoordPort>>>.
pub struct ReconnectableCoord {
    inner: Arc<RwLock<Arc<dyn CoordPort>>>,
}

impl ReconnectableCoord {
    pub fn new(coord: Arc<dyn CoordPort>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(coord)),
        }
    }

    pub fn shared_lock(&self) -> Arc<RwLock<Arc<dyn CoordPort>>> {
        Arc::clone(&self.inner)
    }

    fn current(&self) -> Arc<dyn CoordPort> {
        self.inner.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl std::fmt::Debug for ReconnectableCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReconnectableCoord").finish()
    }
}

#[async_trait]
impl CoordPort for ReconnectableCoord {
    async fn register_agent(
        &self,
        agent_id: &str,
        agent_type: &str,
        intent: &str,
    ) -> Result<(), CoordError> {
        self.current()
            .register_agent(agent_id, agent_type, intent)
            .await
    }
    async fn claim(
        &self,
        agent_id: &str,
        target: &str,
        claim_type: &str,
    ) -> Result<ClaimResult, CoordError> {
        self.current().claim(agent_id, target, claim_type).await
    }
    async fn verify_lease(
        &self,
        guard: &crate::domain::coord::LeaseGuard,
    ) -> Result<cynic_protocol::LeaseVerification, CoordError> {
        self.current().verify_lease(guard).await
    }
    async fn renew_lease(
        &self,
        guard: &crate::domain::coord::LeaseGuard,
    ) -> Result<crate::domain::coord::LeaseMutationResult, CoordError> {
        self.current().renew_lease(guard).await
    }
    async fn release(
        &self,
        guard: &crate::domain::coord::LeaseGuard,
    ) -> Result<crate::domain::coord::LeaseMutationResult, CoordError> {
        self.current().release(guard).await
    }
    async fn finalize_lease(
        &self,
        guard: &crate::domain::coord::LeaseGuard,
        proof: &crate::domain::lease_proof::ValidatedLeaseProof,
    ) -> Result<crate::domain::coord::LeaseMutationResult, CoordError> {
        self.current().finalize_lease(guard, proof).await
    }
    async fn who(&self, agent_id_filter: Option<&str>) -> Result<CoordSnapshot, CoordError> {
        self.current().who(agent_id_filter).await
    }
    async fn store_audit(
        &self,
        tool: &str,
        agent_id: &str,
        details: &str,
    ) -> Result<(), CoordError> {
        self.current().store_audit(tool, agent_id, details).await
    }
    async fn query_audit(
        &self,
        tool_filter: Option<&str>,
        agent_filter: Option<&str>,
        since: Option<&str>,
        until: Option<&str>,
        limit: u32,
    ) -> Result<Vec<AuditEntry>, CoordError> {
        self.current()
            .query_audit(tool_filter, agent_filter, since, until, limit)
            .await
    }
    async fn heartbeat(&self, agent_id: &str) -> Result<(), CoordError> {
        self.current().heartbeat(agent_id).await
    }
    async fn scope_update(&self, agent_id: &str, scope: &str) -> Result<(), CoordError> {
        self.current().scope_update(agent_id, scope).await
    }
    async fn deactivate_agent(&self, agent_id: &str) -> Result<(), CoordError> {
        self.current().deactivate_agent(agent_id).await
    }
    async fn expire_stale(&self) -> Result<(), CoordError> {
        self.current().expire_stale().await
    }
    async fn claim_batch(
        &self,
        agent_id: &str,
        targets: &[String],
        claim_type: &str,
    ) -> Result<BatchClaimResult, CoordError> {
        self.current()
            .claim_batch(agent_id, targets, claim_type)
            .await
    }
}
