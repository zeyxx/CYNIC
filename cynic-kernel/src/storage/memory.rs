//! InMemory Storage Adapter — deterministic, no external dependencies.
//!
//! Implements the same StoragePort contract as SurrealHttpStorage, including:
//! - Crystal state transitions at exact thresholds (21/233 obs, φ⁻¹/φ⁻² confidence)
//! - Quorum gate (voter_count < MIN_QUORUM → reject)
//! - Content set-once (first observation's content wins)
//! - Content sanitization via domain::sanitize
//! - Sorting by maturity then confidence DESC
//!
//! Used for: fast unit tests, CI without SurrealDB, proving StoragePort agnosticism.

use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::Mutex;

use crate::domain::ccm::{
    CANONICAL_CYCLES, Crystal, CrystalState, MIN_CRYSTALLIZATION_CYCLES, SessionSummary,
};
use crate::domain::dog::{MIN_QUORUM, PHI_INV, PHI_INV2, Verdict};
use crate::domain::sanitize::sanitize_crystal_content;
use crate::domain::storage::{
    Observation, ObservationFrequency, RawObservation, SessionTarget, StorageError, StoragePort,
    UsageRow,
};
use crate::domain::usage::DogUsage;

/// In-memory storage for deterministic testing. Thread-safe via tokio::sync::Mutex.
/// Uses tokio Mutex (not std) to prevent latent .await-while-locked traps.
#[derive(Debug)]
pub struct InMemoryStorage {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    verdicts: Vec<Verdict>,
    crystals: HashMap<String, Crystal>,
    observations: Vec<RawObservation>,
    summaries: Vec<SessionSummary>,
    usage: HashMap<String, UsageRow>,
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self {
            state: Mutex::new(State::default()),
        }
    }
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Compute crystal state from observations + confidence.
/// Uses domain constants — same source of truth as surreal.rs SQL.
fn compute_state(observations: u32, confidence: f64) -> CrystalState {
    if observations >= CANONICAL_CYCLES && confidence >= PHI_INV {
        CrystalState::Canonical
    } else if observations >= MIN_CRYSTALLIZATION_CYCLES && confidence >= PHI_INV {
        CrystalState::Crystallized
    } else if observations >= MIN_CRYSTALLIZATION_CYCLES && confidence < PHI_INV2 {
        CrystalState::Decaying
    } else {
        CrystalState::Forming
    }
}

/// Sort crystals by maturity (Canonical > Crystallized > Forming > Decaying > Dissolved)
/// then by confidence DESC within same state.
fn state_rank(s: &CrystalState) -> u8 {
    match s {
        CrystalState::Canonical => 0,
        CrystalState::Crystallized => 1,
        CrystalState::Forming => 2,
        CrystalState::Decaying => 3,
        CrystalState::Dissolved => 4,
    }
}

#[async_trait]
impl StoragePort for InMemoryStorage {
    async fn ping(&self) -> Result<(), StorageError> {
        Ok(())
    }

    async fn store_verdict(&self, verdict: &Verdict) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        s.verdicts.push(verdict.clone());
        Ok(())
    }

    async fn get_verdict(&self, id: &str) -> Result<Option<Verdict>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.verdicts.iter().find(|v| v.id == id).cloned())
    }

    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s.verdicts.iter().rev().cloned().collect();
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn store_crystal(&self, crystal: &Crystal) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        s.crystals.insert(crystal.id.clone(), crystal.clone());
        Ok(())
    }

    async fn get_crystal(&self, id: &str) -> Result<Option<Crystal>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.crystals.get(id).cloned())
    }

    async fn list_crystals(&self, limit: u32) -> Result<Vec<Crystal>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s.crystals.values().cloned().collect();
        v.sort_by(|a, b| {
            state_rank(&a.state).cmp(&state_rank(&b.state)).then(
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn list_crystals_filtered(
        &self,
        limit: u32,
        domain: Option<&str>,
        state: Option<&str>,
    ) -> Result<Vec<Crystal>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s
            .crystals
            .values()
            .filter(|c| domain.is_none_or(|d| c.domain == d))
            .filter(|c| {
                state.is_none_or(|st| {
                    let state_str = match c.state {
                        CrystalState::Forming => "forming",
                        CrystalState::Crystallized => "crystallized",
                        CrystalState::Canonical => "canonical",
                        CrystalState::Decaying => "decaying",
                        CrystalState::Dissolved => "dissolved",
                    };
                    state_str == st
                })
            })
            .cloned()
            .collect();
        v.sort_by(|a, b| {
            state_rank(&a.state).cmp(&state_rank(&b.state)).then(
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn delete_crystal(&self, id: &str) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        s.crystals.remove(id);
        Ok(())
    }

    async fn list_crystals_for_domain(
        &self,
        domain: &str,
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s
            .crystals
            .values()
            .filter(|c| {
                (c.domain == domain || c.domain == "general")
                    && (c.state == CrystalState::Crystallized || c.state == CrystalState::Canonical)
            })
            .cloned()
            .collect();
        v.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn observe_crystal(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: &str,
        voter_count: usize,
    ) -> Result<(), StorageError> {
        // T5+T8: Quorum gate — reject single-Dog observations
        if voter_count < MIN_QUORUM {
            return Err(StorageError::QueryFailed(format!(
                "quorum gate: voter_count {voter_count} < MIN_QUORUM {MIN_QUORUM}"
            )));
        }

        let sanitized = sanitize_crystal_content(content);

        let mut s = self.state.lock().await;

        let crystal = s.crystals.entry(id.to_string()).or_insert_with(|| Crystal {
            id: id.to_string(),
            content: sanitized.clone(),
            domain: domain.to_string(),
            confidence: 0.0,
            observations: 0,
            state: CrystalState::Forming,
            created_at: timestamp.to_string(),
            updated_at: timestamp.to_string(),
        });

        // Content set-once: first observation's content wins (F16 fix)
        // (content is already set from or_insert_with above, don't overwrite)

        // Running mean: new_conf = (old_conf * old_obs + score) / (old_obs + 1)
        let old_obs = crystal.observations as f64;
        crystal.confidence = (crystal.confidence * old_obs + score) / (old_obs + 1.0);
        crystal.observations += 1;
        crystal.updated_at = timestamp.to_string();

        // State transition (mirrors surreal.rs SQL thresholds)
        crystal.state = compute_state(crystal.observations, crystal.confidence);

        Ok(())
    }

    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        let obs_id = format!("observation:{}", s.observations.len());
        s.observations.push(RawObservation {
            id: obs_id,
            tool: obs.tool.clone(),
            target: obs.target.clone(),
            domain: obs.domain.clone(),
            status: obs.status.clone(),
            context: obs.context.clone(),
            created_at: obs.timestamp.clone(),
            project: obs.project.clone(),
            agent_id: obs.agent_id.clone(),
            session_id: obs.session_id.clone(),
        });
        Ok(())
    }

    async fn query_observations(
        &self,
        project: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ObservationFrequency>, StorageError> {
        let s = self.state.lock().await;
        let mut freq: HashMap<(String, String), u64> = HashMap::new();
        for obs in &s.observations {
            if obs.project == project && domain.is_none_or(|d| obs.domain == d) {
                *freq
                    .entry((obs.target.clone(), obs.tool.clone()))
                    .or_default() += 1;
            }
        }
        let mut result: Vec<_> = freq
            .into_iter()
            .map(|((target, tool), f)| ObservationFrequency {
                target,
                tool,
                freq: f,
            })
            .collect();
        result.sort_by(|a, b| b.freq.cmp(&a.freq));
        result.truncate(limit as usize);
        Ok(result)
    }

    async fn query_session_targets(
        &self,
        project: &str,
        limit: u32,
    ) -> Result<Vec<SessionTarget>, StorageError> {
        let s = self.state.lock().await;
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for obs in &s.observations {
            if obs.project == project {
                let key = (obs.session_id.clone(), obs.target.clone());
                if seen.insert(key.clone()) {
                    result.push(SessionTarget {
                        session_id: key.0,
                        target: key.1,
                    });
                }
            }
        }
        result.truncate(limit as usize);
        Ok(result)
    }

    async fn list_observations_raw(
        &self,
        domain: Option<&str>,
        agent_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        let s = self.state.lock().await;
        let v: Vec<_> = s
            .observations
            .iter()
            .rev()
            .filter(|o| domain.is_none_or(|d| o.domain == d))
            .filter(|o| agent_id.is_none_or(|a| o.agent_id == a))
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(v)
    }

    async fn store_session_summary(&self, summary: &SessionSummary) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        s.summaries.push(summary.clone());
        Ok(())
    }

    async fn list_session_summaries(
        &self,
        limit: u32,
    ) -> Result<Vec<SessionSummary>, StorageError> {
        let s = self.state.lock().await;
        let mut v = s.summaries.clone();
        v.reverse();
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn get_unsummarized_sessions(
        &self,
        min_observations: u32,
        limit: u32,
    ) -> Result<Vec<(String, String, u32)>, StorageError> {
        let s = self.state.lock().await;
        let summarized: std::collections::HashSet<_> =
            s.summaries.iter().map(|ss| ss.session_id.clone()).collect();
        let mut counts: HashMap<String, (String, u32)> = HashMap::new();
        for obs in &s.observations {
            let entry = counts
                .entry(obs.session_id.clone())
                .or_insert_with(|| (obs.agent_id.clone(), 0));
            entry.1 += 1;
        }
        let mut result: Vec<_> = counts
            .into_iter()
            .filter(|(sid, (_, count))| *count >= min_observations && !summarized.contains(sid))
            .map(|(sid, (aid, count))| (sid, aid, count))
            .collect();
        result.sort_by(|a, b| b.2.cmp(&a.2));
        result.truncate(limit as usize);
        Ok(result)
    }

    async fn get_session_observations(
        &self,
        session_id: &str,
    ) -> Result<Vec<RawObservation>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.observations
            .iter()
            .filter(|o| o.session_id == session_id)
            .cloned()
            .collect())
    }

    async fn flush_usage(&self, snapshot: &[(String, DogUsage)]) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        for (dog_id, usage) in snapshot {
            let entry = s.usage.entry(dog_id.clone()).or_insert_with(|| UsageRow {
                dog_id: dog_id.clone(),
                prompt_tokens: 0,
                completion_tokens: 0,
                requests: 0,
                failures: 0,
                total_latency_ms: 0,
            });
            entry.prompt_tokens += usage.prompt_tokens;
            entry.completion_tokens += usage.completion_tokens;
            entry.requests += usage.requests;
            entry.failures += usage.failures;
            entry.total_latency_ms += usage.total_latency_ms;
        }
        Ok(())
    }

    async fn load_usage_history(&self) -> Result<Vec<UsageRow>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.usage.values().cloned().collect())
    }

    async fn count_verdicts(&self) -> Result<u64, StorageError> {
        let s = self.state.lock().await;
        Ok(s.verdicts.len() as u64)
    }

    async fn count_crystal_observations(&self) -> Result<u64, StorageError> {
        let s = self.state.lock().await;
        Ok(s.crystals.values().map(|c| u64::from(c.observations)).sum())
    }
}
