//! StoragePort + CoordPort implementations for SurrealHttpStorage.

mod activity;
mod coord;
mod crystals;
mod maintenance;
mod ops;
mod verdicts;

use super::{SurrealHttpStorage, safe_limit};
use crate::domain::ccm::Crystal;
use crate::domain::dog::Verdict;
use crate::domain::storage::{
    Observation, ObservationFrequency, RawObservation, SessionTarget, StorageError, StoragePort,
    UsageRow,
};

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

    async fn store_verdict(&self, verdict: &Verdict) -> Result<(), StorageError> {
        verdicts::store_verdict(self, verdict).await
    }

    async fn get_verdict(&self, id: &str) -> Result<Option<Verdict>, StorageError> {
        verdicts::get_verdict(self, id).await
    }

    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError> {
        verdicts::list_verdicts(self, limit).await
    }

    async fn store_crystal(&self, crystal: &Crystal) -> Result<(), StorageError> {
        crystals::store_crystal(self, crystal).await
    }

    async fn get_crystal(&self, id: &str) -> Result<Option<Crystal>, StorageError> {
        crystals::get_crystal(self, id).await
    }

    async fn list_crystals(&self, limit: u32) -> Result<Vec<Crystal>, StorageError> {
        crystals::list_crystals(self, limit).await
    }

    async fn list_crystals_filtered(
        &self,
        limit: u32,
        domain: Option<&str>,
        state: Option<&str>,
    ) -> Result<Vec<Crystal>, StorageError> {
        crystals::list_crystals_filtered(self, limit, domain, state).await
    }

    async fn delete_crystal(&self, id: &str) -> Result<(), StorageError> {
        crystals::delete_crystal(self, id).await
    }

    async fn list_crystals_for_domain(
        &self,
        domain: &str,
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        crystals::list_crystals_for_domain(self, domain, limit).await
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

    async fn store_crystal_embedding(
        &self,
        id: &str,
        embedding: &[f32],
    ) -> Result<(), StorageError> {
        crystals::store_crystal_embedding(self, id, embedding).await
    }

    async fn search_crystals_semantic(
        &self,
        query_embedding: &[f32],
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        crystals::search_crystals_semantic(self, query_embedding, limit).await
    }

    async fn find_similar_crystal(
        &self,
        embedding: &[f32],
        domain: &str,
        threshold: f64,
    ) -> Result<Option<(String, f64)>, StorageError> {
        crystals::find_similar_crystal(self, embedding, domain, threshold).await
    }

    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError> {
        activity::store_observation(self, obs).await
    }

    async fn query_observations(
        &self,
        project: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ObservationFrequency>, StorageError> {
        activity::query_observations(self, project, domain, limit).await
    }

    async fn query_session_targets(
        &self,
        project: &str,
        limit: u32,
    ) -> Result<Vec<SessionTarget>, StorageError> {
        activity::query_session_targets(self, project, limit).await
    }

    async fn store_session_summary(
        &self,
        summary: &crate::domain::ccm::SessionSummary,
    ) -> Result<(), StorageError> {
        activity::store_session_summary(self, summary).await
    }

    async fn list_session_summaries(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::SessionSummary>, StorageError> {
        activity::list_session_summaries(self, limit).await
    }

    async fn get_unsummarized_sessions(
        &self,
        min_observations: u32,
        limit: u32,
    ) -> Result<Vec<(String, String, u32)>, StorageError> {
        activity::get_unsummarized_sessions(self, min_observations, limit).await
    }

    async fn get_session_observations(
        &self,
        session_id: &str,
    ) -> Result<Vec<RawObservation>, StorageError> {
        activity::get_session_observations(self, session_id).await
    }

    async fn store_session_compliance(
        &self,
        c: &crate::domain::compliance::SessionCompliance,
    ) -> Result<(), StorageError> {
        activity::store_session_compliance(self, c).await
    }

    async fn list_session_compliance(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::compliance::SessionCompliance>, StorageError> {
        activity::list_session_compliance(self, limit).await
    }

    async fn flush_usage(
        &self,
        snapshot: &[(String, crate::domain::usage::DogUsage)],
    ) -> Result<(), StorageError> {
        ops::flush_usage(self, snapshot).await
    }

    async fn cleanup_ttl(&self) -> Result<(), StorageError> {
        maintenance::cleanup_ttl(self).await
    }

    async fn last_integrity_hash(&self) -> Result<Option<String>, StorageError> {
        maintenance::last_integrity_hash(self).await
    }

    async fn load_usage_history(&self) -> Result<Vec<UsageRow>, StorageError> {
        ops::load_usage_history(self).await
    }

    async fn flush_dog_stats(
        &self,
        stats: &[(String, crate::organ::health::DogStats)],
    ) -> Result<(), StorageError> {
        ops::flush_dog_stats(self, stats).await
    }

    async fn load_dog_stats(
        &self,
    ) -> Result<Vec<(String, crate::organ::health::DogStats)>, StorageError> {
        ops::load_dog_stats(self).await
    }

    async fn list_crystals_missing_embedding(
        &self,
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        maintenance::list_crystals_missing_embedding(self, limit).await
    }

    async fn count_verdicts(&self) -> Result<u64, StorageError> {
        maintenance::count_verdicts(self).await
    }

    async fn count_crystal_observations(&self) -> Result<u64, StorageError> {
        maintenance::count_crystal_observations(self).await
    }

    async fn list_observations_raw(
        &self,
        domain: Option<&str>,
        agent_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        activity::list_observations_raw(self, domain, agent_id, limit).await
    }

    async fn store_infra_snapshot(
        &self,
        snap: &crate::domain::probe::EnvironmentSnapshot,
    ) -> Result<(), StorageError> {
        ops::store_infra_snapshot(self, snap).await
    }

    async fn list_infra_snapshots(
        &self,
        hours: u32,
    ) -> Result<Vec<crate::domain::probe::EnvironmentSnapshot>, StorageError> {
        ops::list_infra_snapshots(self, hours).await
    }

    async fn cleanup_infra_snapshots(&self, older_than_days: u32) -> Result<u64, StorageError> {
        ops::cleanup_infra_snapshots(self, older_than_days).await
    }

    async fn consolidate_duplicate_crystals(&self) -> Result<u64, StorageError> {
        maintenance::consolidate_duplicate_crystals(self).await
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
