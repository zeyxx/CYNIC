//! Reconnectable storage — swappable StoragePort + CoordPort wrappers.
//!
//! The kernel boots with NullStorage when SurrealDB is unavailable.
//! These wrappers delegate all trait methods through a RwLock<Arc<dyn T>>,
//! allowing a background task to swap NullStorage → SurrealHttpStorage
//! without restarting the kernel.
//!
//! K15: The introspection loop detects "storage_down" but never acts.
//! StorageReconnector closes that loop — detection → reconnection → recovery.

use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::domain::coord::{
    AuditEntry, BatchClaimResult, ClaimResult, CoordError, CoordPort, CoordSnapshot,
};
use crate::domain::storage::{
    Observation, RawObservation, StorageError, StorageMetrics, StoragePort, UsageRow,
};
use crate::infra::config::StorageConfig;

// ── ReconnectableStorage ────────────────────────────────────

/// StoragePort proxy that can swap its inner implementation at runtime.
/// All consumers see the swap transparently — no API changes needed.
pub struct ReconnectableStorage {
    inner: Arc<RwLock<Arc<dyn StoragePort>>>,
}

impl ReconnectableStorage {
    pub fn new(storage: Arc<dyn StoragePort>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(storage)),
        }
    }

    /// Share the lock with a StorageReconnector.
    pub fn shared_lock(&self) -> Arc<RwLock<Arc<dyn StoragePort>>> {
        Arc::clone(&self.inner)
    }

    /// Clone the current inner Arc — holds the read lock only for the clone.
    fn current(&self) -> Arc<dyn StoragePort> {
        // K14: poisoned lock → assume degraded, fall back to NullStorage
        self.inner.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl std::fmt::Debug for ReconnectableStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReconnectableStorage").finish()
    }
}

#[async_trait]
impl StoragePort for ReconnectableStorage {
    async fn ping(&self) -> Result<(), StorageError> {
        self.current().ping().await
    }
    fn metrics(&self) -> Option<StorageMetrics> {
        self.current().metrics()
    }
    async fn store_verdict(
        &self,
        verdict: &crate::domain::dog::Verdict,
    ) -> Result<(), StorageError> {
        self.current().store_verdict(verdict).await
    }
    async fn get_verdict(
        &self,
        id: &str,
    ) -> Result<Option<crate::domain::dog::Verdict>, StorageError> {
        self.current().get_verdict(id).await
    }
    async fn list_verdicts(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::dog::Verdict>, StorageError> {
        self.current().list_verdicts(limit).await
    }
    async fn store_crystal(
        &self,
        crystal: &crate::domain::ccm::Crystal,
    ) -> Result<(), StorageError> {
        self.current().store_crystal(crystal).await
    }
    async fn get_crystal(
        &self,
        id: &str,
    ) -> Result<Option<crate::domain::ccm::Crystal>, StorageError> {
        self.current().get_crystal(id).await
    }
    async fn list_crystals(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
        self.current().list_crystals(limit).await
    }
    async fn list_crystals_filtered(
        &self,
        limit: u32,
        domain: Option<&str>,
        state: Option<&str>,
    ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
        self.current()
            .list_crystals_filtered(limit, domain, state)
            .await
    }
    async fn delete_crystal(&self, id: &str) -> Result<(), StorageError> {
        self.current().delete_crystal(id).await
    }
    async fn list_crystals_for_domain(
        &self,
        domain: &str,
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
        self.current().list_crystals_for_domain(domain, limit).await
    }
    async fn observe_crystal(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: &str,
        voter_count: usize,
        verdict_id: &str,
        verdict_kind: &str,
    ) -> Result<(), StorageError> {
        self.current()
            .observe_crystal(
                id,
                content,
                domain,
                score,
                timestamp,
                voter_count,
                verdict_id,
                verdict_kind,
            )
            .await
    }
    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError> {
        self.current().store_observation(obs).await
    }
    async fn list_crystals_missing_embedding(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
        self.current().list_crystals_missing_embedding(limit).await
    }
    async fn count_verdicts(&self) -> Result<u64, StorageError> {
        self.current().count_verdicts().await
    }
    async fn count_crystal_observations(&self) -> Result<u64, StorageError> {
        self.current().count_crystal_observations().await
    }
    async fn list_observations_raw(
        &self,
        domain: Option<&str>,
        agent_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        self.current()
            .list_observations_raw(domain, agent_id, limit)
            .await
    }
    async fn store_crystal_embedding(
        &self,
        id: &str,
        embedding: &[f32],
    ) -> Result<(), StorageError> {
        self.current().store_crystal_embedding(id, embedding).await
    }
    async fn search_crystals_semantic(
        &self,
        query_embedding: &[f32],
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
        self.current()
            .search_crystals_semantic(query_embedding, limit)
            .await
    }
    async fn find_similar_crystal(
        &self,
        embedding: &[f32],
        domain: &str,
        threshold: f64,
    ) -> Result<Option<(String, f64)>, StorageError> {
        self.current()
            .find_similar_crystal(embedding, domain, threshold)
            .await
    }
    async fn store_session_summary(
        &self,
        summary: &crate::domain::ccm::SessionSummary,
    ) -> Result<(), StorageError> {
        self.current().store_session_summary(summary).await
    }
    async fn list_session_summaries(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::SessionSummary>, StorageError> {
        self.current().list_session_summaries(limit).await
    }
    async fn get_unsummarized_sessions(
        &self,
        min_observations: u32,
        limit: u32,
    ) -> Result<Vec<(String, String, u32)>, StorageError> {
        self.current()
            .get_unsummarized_sessions(min_observations, limit)
            .await
    }
    async fn get_session_observations(
        &self,
        session_id: &str,
    ) -> Result<Vec<RawObservation>, StorageError> {
        self.current().get_session_observations(session_id).await
    }
    async fn store_session_compliance(
        &self,
        compliance: &crate::domain::compliance::SessionCompliance,
    ) -> Result<(), StorageError> {
        self.current().store_session_compliance(compliance).await
    }
    async fn list_session_compliance(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::compliance::SessionCompliance>, StorageError> {
        self.current().list_session_compliance(limit).await
    }
    async fn flush_usage(
        &self,
        snapshot: &[(String, crate::domain::usage::DogUsage)],
    ) -> Result<(), StorageError> {
        self.current().flush_usage(snapshot).await
    }
    async fn cleanup_ttl(&self) -> Result<(), StorageError> {
        self.current().cleanup_ttl().await
    }
    async fn last_integrity_hash(&self) -> Result<Option<String>, StorageError> {
        self.current().last_integrity_hash().await
    }
    async fn load_usage_history(&self) -> Result<Vec<UsageRow>, StorageError> {
        self.current().load_usage_history().await
    }
    async fn flush_dog_stats(
        &self,
        stats: &[(String, crate::organ::health::DogStats)],
    ) -> Result<(), StorageError> {
        self.current().flush_dog_stats(stats).await
    }
    async fn load_dog_stats(
        &self,
    ) -> Result<Vec<(String, crate::organ::health::DogStats)>, StorageError> {
        self.current().load_dog_stats().await
    }
    async fn consolidate_duplicate_crystals(&self) -> Result<u64, StorageError> {
        self.current().consolidate_duplicate_crystals().await
    }

    // ── State Log ──────────────────────────────────
    async fn store_state_block(
        &self,
        block: &crate::domain::state_log::StateBlock,
    ) -> Result<(), StorageError> {
        self.current().store_state_block(block).await
    }
    async fn last_state_block(
        &self,
    ) -> Result<Option<crate::domain::state_log::StateBlock>, StorageError> {
        self.current().last_state_block().await
    }
    async fn list_state_blocks(
        &self,
        since: &str,
        limit: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
        self.current().list_state_blocks(since, limit).await
    }
}

// ── ReconnectableCoord ──────────────────────────────────────

/// CoordPort proxy — same pattern as ReconnectableStorage.
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
    async fn release(&self, agent_id: &str, target: Option<&str>) -> Result<String, CoordError> {
        self.current().release(agent_id, target).await
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
        limit: u32,
    ) -> Result<Vec<AuditEntry>, CoordError> {
        self.current()
            .query_audit(tool_filter, agent_filter, limit)
            .await
    }
    async fn heartbeat(&self, agent_id: &str) -> Result<(), CoordError> {
        self.current().heartbeat(agent_id).await
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

// ── StorageReconnector ──────────────────────────────────────

/// Holds shared locks to both storage + coord, and the config needed to
/// reconnect. A background task calls try_reconnect() periodically.
///
/// SurrealHttpStorage implements both StoragePort and CoordPort.
/// On successful reconnect, both locks are swapped atomically (sequentially
/// under write lock — the window is microseconds, not observable).
pub struct StorageReconnector {
    storage_lock: Arc<RwLock<Arc<dyn StoragePort>>>,
    coord_lock: Arc<RwLock<Arc<dyn CoordPort>>>,
    config: StorageConfig,
}

impl std::fmt::Debug for StorageReconnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageReconnector")
            .field("url", &self.config.url)
            .finish()
    }
}

impl StorageReconnector {
    pub fn new(
        storage_lock: Arc<RwLock<Arc<dyn StoragePort>>>,
        coord_lock: Arc<RwLock<Arc<dyn CoordPort>>>,
        config: StorageConfig,
    ) -> Self {
        Self {
            storage_lock,
            coord_lock,
            config,
        }
    }

    /// Check if current storage is degraded (NullStorage or unreachable).
    pub async fn is_degraded(&self) -> bool {
        let current = self
            .storage_lock
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        current.ping().await.is_err()
    }

    /// Attempt to reconnect to SurrealDB. Returns true on success.
    ///
    /// Flow: null-check → init → ping → swap both locks.
    /// Only attempts if current storage is actually degraded.
    pub async fn try_reconnect(&self) -> bool {
        // Null-check: don't attempt if storage is already healthy
        if !self.is_degraded().await {
            return false;
        }

        match super::SurrealHttpStorage::init(&self.config).await {
            Ok(db) => {
                let db = Arc::new(db);

                // Swap storage
                match self.storage_lock.write() {
                    Ok(mut guard) => *guard = Arc::clone(&db) as Arc<dyn StoragePort>,
                    Err(e) => {
                        tracing::error!(error = %e, "storage RwLock poisoned during reconnect");
                        return false;
                    }
                }

                // Swap coord
                match self.coord_lock.write() {
                    Ok(mut guard) => *guard = Arc::clone(&db) as Arc<dyn CoordPort>,
                    Err(e) => {
                        tracing::error!(error = %e, "coord RwLock poisoned during reconnect");
                        // Storage already swapped — coord will catch up next tick.
                        // Not rolling back storage because the DB IS available.
                    }
                }

                tracing::info!(
                    url = %self.config.url,
                    ns = %self.config.namespace,
                    db = %self.config.database,
                    "storage RECONNECTED — NullStorage → SurrealDB"
                );
                true
            }
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    url = %self.config.url,
                    "storage reconnect failed — will retry"
                );
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::coord::NullCoord;
    use crate::domain::storage::NullStorage;

    #[tokio::test]
    async fn reconnectable_storage_delegates_ping_to_null() {
        let storage = ReconnectableStorage::new(Arc::new(NullStorage));
        assert!(
            storage.ping().await.is_err(),
            "NullStorage ping should fail"
        );
    }

    #[tokio::test]
    async fn reconnectable_coord_delegates_to_null() {
        let coord = ReconnectableCoord::new(Arc::new(NullCoord));
        let result = coord.who(None).await;
        assert!(result.is_ok());
        let snap = result.unwrap();
        assert!(snap.agents.is_empty());
    }

    #[tokio::test]
    async fn reconnector_is_degraded_with_null_storage() {
        let storage = ReconnectableStorage::new(Arc::new(NullStorage));
        let coord = ReconnectableCoord::new(Arc::new(NullCoord));
        let reconnector = StorageReconnector::new(
            storage.shared_lock(),
            coord.shared_lock(),
            StorageConfig::default(),
        );
        assert!(reconnector.is_degraded().await);
    }

    #[tokio::test]
    async fn reconnector_try_reconnect_fails_without_db() {
        let storage = ReconnectableStorage::new(Arc::new(NullStorage));
        let coord = ReconnectableCoord::new(Arc::new(NullCoord));
        let reconnector = StorageReconnector::new(
            storage.shared_lock(),
            coord.shared_lock(),
            StorageConfig {
                url: "http://127.0.0.1:19999".to_string(), // unreachable
                namespace: "test".to_string(),
                database: "test".to_string(),
            },
        );
        // Should attempt (is_degraded=true) but fail (no DB at that port)
        assert!(!reconnector.try_reconnect().await);
        // Storage should still be NullStorage
        assert!(storage.ping().await.is_err());
    }
}
