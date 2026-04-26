//! StoragePort + CoordPort implementations for SurrealHttpStorage.

mod activity;
mod agent_tasks;
mod coord;
mod crystals;
mod maintenance;
mod ops;
mod state_log;
mod verdict_queue;
mod verdicts;

use super::{SurrealHttpStorage, safe_limit};
use crate::domain::ccm::Crystal;
use crate::domain::dog::Verdict;
use crate::domain::storage::{
    AgentTask, Observation, RawObservation, StorageError, StoragePort, UsageRow,
};
use crate::domain::verdict_queue::QueuedVerdict;

// ── SHARED HELPERS ─────────────────────────────────────────────

/// Build a SQL WHERE clause from optional conditions.
/// Returns empty string if no conditions, or ` WHERE cond1 AND cond2` (leading space).
fn build_where_clause(conditions: &[String]) -> String {
    if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    }
}

/// Sanitize string for use as SurrealDB record ID.
/// Uses percent-encoding: unsafe chars -> `%XX`, `%` itself -> `%25`.
/// Collision-free: injective mapping (different inputs -> different outputs).
/// Length-limited to 256 chars (char-aware, no UTF-8 boundary panic).
fn sanitize_record_id(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().take(256) {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else if c == '%' {
            out.push_str("%25");
        } else if c.is_ascii() {
            out.push_str(&format!("%{:02x}", c as u8));
        } else {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).bytes() {
                out.push_str(&format!("%{b:02x}"));
            }
        }
    }
    out
}

// ── STORAGE PORT IMPLEMENTATION ──────────────────────────────

#[async_trait::async_trait]
impl StoragePort for SurrealHttpStorage {
    #[tracing::instrument(skip(self), err)]
    async fn ping(&self) -> Result<(), StorageError> {
        self.query("INFO FOR DB;").await?;
        Ok(())
    }

    fn metrics(&self) -> Option<crate::domain::storage::StorageMetrics> {
        let snap = self.metrics.snapshot();
        Some(crate::domain::storage::StorageMetrics {
            queries: snap.queries,
            errors: snap.errors,
            slow_queries: snap.slow_queries,
            avg_latency_ms: snap.avg_latency_ms,
            uptime_secs: snap.uptime_secs,
        })
    }

    #[tracing::instrument(skip(self), err)]
    async fn store_verdict(&self, verdict: &Verdict) -> Result<(), StorageError> {
        verdicts::store_verdict(self, verdict).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn get_verdict(&self, id: &str) -> Result<Option<Verdict>, StorageError> {
        verdicts::get_verdict(self, id).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError> {
        verdicts::list_verdicts(self, limit).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn store_crystal(&self, crystal: &Crystal) -> Result<(), StorageError> {
        crystals::store_crystal(self, crystal).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn get_crystal(&self, id: &str) -> Result<Option<Crystal>, StorageError> {
        crystals::get_crystal(self, id).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn list_crystals(&self, limit: u32) -> Result<Vec<Crystal>, StorageError> {
        crystals::list_crystals(self, limit).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn list_crystals_filtered(
        &self,
        limit: u32,
        domain: Option<&str>,
        state: Option<&str>,
    ) -> Result<Vec<Crystal>, StorageError> {
        crystals::list_crystals_filtered(self, limit, domain, state).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn delete_crystal(&self, id: &str) -> Result<(), StorageError> {
        crystals::delete_crystal(self, id).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn list_crystals_for_domain(
        &self,
        domain: &str,
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        crystals::list_crystals_for_domain(self, domain, limit).await
    }

    #[tracing::instrument(skip(self), err)]
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
        crystals::observe_crystal(
            self,
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

    #[tracing::instrument(skip(self), err)]
    async fn store_crystal_embedding(
        &self,
        id: &str,
        embedding: &[f32],
    ) -> Result<(), StorageError> {
        crystals::store_crystal_embedding(self, id, embedding).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn search_crystals_semantic(
        &self,
        query_embedding: &[f32],
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        crystals::search_crystals_semantic(self, query_embedding, limit).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn find_similar_crystal(
        &self,
        embedding: &[f32],
        domain: &str,
        threshold: f64,
    ) -> Result<Option<(String, f64)>, StorageError> {
        crystals::find_similar_crystal(self, embedding, domain, threshold).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError> {
        activity::store_observation(self, obs).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn store_session_summary(
        &self,
        summary: &crate::domain::ccm::SessionSummary,
    ) -> Result<(), StorageError> {
        activity::store_session_summary(self, summary).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn list_session_summaries(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::SessionSummary>, StorageError> {
        activity::list_session_summaries(self, limit).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn get_unsummarized_sessions(
        &self,
        min_observations: u32,
        limit: u32,
    ) -> Result<Vec<(String, String, u32)>, StorageError> {
        activity::get_unsummarized_sessions(self, min_observations, limit).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn get_session_observations(
        &self,
        session_id: &str,
    ) -> Result<Vec<RawObservation>, StorageError> {
        activity::get_session_observations(self, session_id).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn store_session_compliance(
        &self,
        c: &crate::domain::compliance::SessionCompliance,
    ) -> Result<(), StorageError> {
        activity::store_session_compliance(self, c).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn list_session_compliance(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::compliance::SessionCompliance>, StorageError> {
        activity::list_session_compliance(self, limit).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn flush_usage(
        &self,
        snapshot: &[(String, crate::domain::usage::DogUsage)],
    ) -> Result<(), StorageError> {
        ops::flush_usage(self, snapshot).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn cleanup_ttl(&self) -> Result<(), StorageError> {
        maintenance::cleanup_ttl(self).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn last_integrity_hash(&self) -> Result<Option<String>, StorageError> {
        maintenance::last_integrity_hash(self).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn load_usage_history(&self) -> Result<Vec<UsageRow>, StorageError> {
        ops::load_usage_history(self).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn flush_dog_stats(
        &self,
        stats: &[(String, crate::organ::health::DogStats)],
    ) -> Result<(), StorageError> {
        ops::flush_dog_stats(self, stats).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn load_dog_stats(
        &self,
    ) -> Result<Vec<(String, crate::organ::health::DogStats)>, StorageError> {
        ops::load_dog_stats(self).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn list_crystals_missing_embedding(
        &self,
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        maintenance::list_crystals_missing_embedding(self, limit).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn count_verdicts(&self) -> Result<u64, StorageError> {
        maintenance::count_verdicts(self).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn count_crystal_observations(&self) -> Result<u64, StorageError> {
        maintenance::count_crystal_observations(self).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn list_observations_raw(
        &self,
        domain: Option<&str>,
        agent_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        activity::list_observations_raw(self, domain, agent_id, limit).await
    }

    async fn last_observation_per_source(
        &self,
    ) -> Result<Vec<(String, String, u64)>, StorageError> {
        activity::last_observation_per_source(self).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn consolidate_duplicate_crystals(&self) -> Result<u64, StorageError> {
        maintenance::consolidate_duplicate_crystals(self).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn store_agent_task(&self, task: &AgentTask) -> Result<String, StorageError> {
        agent_tasks::store_agent_task(self, task).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn list_pending_agent_tasks(
        &self,
        kind: &str,
        limit: u32,
    ) -> Result<Vec<AgentTask>, StorageError> {
        agent_tasks::list_pending_agent_tasks(self, kind, limit).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn mark_agent_task_processing(&self, task_id: &str) -> Result<(), StorageError> {
        agent_tasks::mark_agent_task_processing(self, task_id).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn update_agent_task_result(
        &self,
        task_id: &str,
        result: Option<String>,
        error: Option<String>,
    ) -> Result<(), StorageError> {
        let result_ref = result.as_deref();
        let error_ref = error.as_deref();
        agent_tasks::update_agent_task_result(self, task_id, result_ref, error_ref).await
    }

    // ── State Log ──────────────────────────────────

    async fn store_state_block(
        &self,
        block: &crate::domain::state_log::StateBlock,
    ) -> Result<(), StorageError> {
        state_log::store_state_block(self, block).await
    }

    async fn last_state_block(
        &self,
    ) -> Result<Option<crate::domain::state_log::StateBlock>, StorageError> {
        state_log::last_state_block(self).await
    }

    async fn list_state_blocks(
        &self,
        since: &str,
        limit: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
        state_log::list_state_blocks(self, since, limit).await
    }

    // ── Verdict Submission Queue ──────────────────────────────

    #[tracing::instrument(skip(self), err)]
    async fn enqueue_verdict(
        &self,
        verdict_id: &str,
        content_hash: &str,
        q_score: f64,
    ) -> Result<(), StorageError> {
        verdict_queue::enqueue_verdict(self, verdict_id, content_hash, q_score).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn list_pending_verdicts(&self, limit: u32) -> Result<Vec<QueuedVerdict>, StorageError> {
        verdict_queue::list_pending_verdicts(self, limit).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn get_queued_verdict(
        &self,
        verdict_id: &str,
    ) -> Result<Option<QueuedVerdict>, StorageError> {
        verdict_queue::get_queued_verdict(self, verdict_id).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn update_verdict_submitted(
        &self,
        verdict_id: &str,
        tx_signature: &str,
    ) -> Result<(), StorageError> {
        // Default: no verdict PDA at submission time, only after confirmation
        verdict_queue::update_verdict_submitted(self, verdict_id, tx_signature, "").await
    }

    #[tracing::instrument(skip(self), err)]
    async fn update_verdict_confirmed(&self, verdict_id: &str) -> Result<(), StorageError> {
        verdict_queue::update_verdict_confirmed(self, verdict_id).await
    }

    #[tracing::instrument(skip(self), err)]
    async fn update_verdict_failed(
        &self,
        verdict_id: &str,
        error_reason: &str,
    ) -> Result<(), StorageError> {
        // Fetch current retry_count and increment
        if let Some(verdict) = verdict_queue::get_queued_verdict(self, verdict_id).await? {
            let new_count = verdict.retry_count + 1;
            verdict_queue::update_verdict_failed(self, verdict_id, error_reason, new_count).await
        } else {
            Err(StorageError::NotFound(format!(
                "queued verdict {} not found",
                verdict_id
            )))
        }
    }

    #[tracing::instrument(skip(self), err)]
    async fn queue_status_counts(&self) -> Result<(u32, u32, u32, u32), StorageError> {
        let (pending, submitted, confirmed, failed) =
            verdict_queue::queue_status_counts(self).await?;
        Ok((
            pending as u32,
            submitted as u32,
            confirmed as u32,
            failed as u32,
        ))
    }
}

// ── TESTS ────────────────────────────────────────────────────

#[cfg(test)]
// WHY: Integration tests use eprintln! for SurrealDB connection diagnostics during
// local development — these are never reached in production code paths.
#[allow(clippy::print_stderr)]
mod tests {
    use super::*;
    use crate::storage::{escape_surreal, sanitize_id};

    #[test]
    fn sanitize_id_rejects_injection() {
        assert!(sanitize_id("'; DROP TABLE verdict; --").is_err());
        assert!(sanitize_id("abc\0def").is_err());
        assert!(sanitize_id("abc`def").is_err());
        assert!(sanitize_id("abc;def").is_err());
        assert!(sanitize_id("abc def").is_err());
        assert!(sanitize_id("").is_err());
        assert!(sanitize_id(&"a".repeat(129)).is_err());
    }

    #[test]
    fn sanitize_id_accepts_valid() {
        assert!(sanitize_id("abc-123_def").is_ok());
        assert!(sanitize_id("verdict-001").is_ok());
        assert!(sanitize_id("a1b2c3d4e5f6").is_ok());
        assert!(sanitize_id(&"a".repeat(128)).is_ok());
    }

    #[test]
    fn escape_surreal_handles_special_chars() {
        assert_eq!(escape_surreal("it's"), "it\\'s");
        assert_eq!(escape_surreal("a\\b"), "a\\\\b");
        assert_eq!(escape_surreal("a\0b"), "ab");
        assert_eq!(escape_surreal("a\nb"), "a\\nb");
        assert_eq!(escape_surreal("a\rb"), "a\\rb");
        assert_eq!(escape_surreal("a\tb"), "a\\tb");
    }

    #[test]
    fn safe_limit_clamps() {
        assert_eq!(safe_limit(20), 20);
        assert_eq!(safe_limit(100), 100);
        assert_eq!(safe_limit(101), 100);
        assert_eq!(safe_limit(u32::MAX), 100);
    }
}
