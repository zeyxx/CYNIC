//! InMemory Storage Adapter — deterministic, no external dependencies.
//!
//! Implements the same StoragePort contract as SurrealHttpStorage, including:
//! - Crystal state transitions at exact thresholds (21/233 obs, φ⁻¹/φ⁻² confidence)
//! - Quorum gate (voter_count < MIN_QUORUM → reject)
//! - Content evolves: updated when new observation has higher confidence (KC12 fix)
//! - Content sanitization via domain::sanitize
//! - Sorting by maturity then confidence DESC
//!
//! Used for: fast unit tests, CI without SurrealDB, proving StoragePort agnosticism.

use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::Mutex;

use crate::domain::ccm::{Crystal, CrystalState, SessionSummary};
use crate::domain::dog::{MIN_QUORUM, Verdict};
use crate::domain::dog_health::DogStats;
use crate::domain::sanitize::sanitize_crystal_content;
use crate::domain::storage::{
    ActivityStorage, AgentDispatch, AgentTask, CrystalStorage, DispatchStorage, Event, Observation,
    RawEvent, RawObservation, StateLogStorage, StorageError, StoragePort, SubmissionStorage,
    TaskStorage, UsageRow, VerdictStorage,
};
use crate::domain::usage::DogUsage;
use crate::domain::verdict_queue::QueuedVerdict;

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
    tasks: HashMap<String, AgentTask>,
    events: Vec<Event>,
    dog_stats: HashMap<String, DogStats>,
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

// State computation uses domain function (ccm::classify) — single source of truth.

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
impl VerdictStorage for InMemoryStorage {
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

    async fn recent_max_disagreements(&self, limit: usize) -> Result<Vec<f64>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s.verdicts.iter().map(|v| v.max_disagreement).collect();
        v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(limit);
        Ok(v)
    }

    async fn list_verdicts_by_domain(
        &self,
        domain: &str,
        limit: u32,
    ) -> Result<Vec<Verdict>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s
            .verdicts
            .iter()
            .filter(|v| v.domain == domain)
            .cloned()
            .collect();
        v.sort_by(|a, b| {
            a.q_score
                .sovereignty
                .partial_cmp(&b.q_score.sovereignty)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn last_integrity_hash(&self) -> Result<Option<String>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.verdicts
            .iter()
            .max_by_key(|v| &v.timestamp)
            .and_then(|v| v.integrity_hash.clone()))
    }

    async fn count_verdicts(&self) -> Result<u64, StorageError> {
        let s = self.state.lock().await;
        Ok(s.verdicts.len() as u64)
    }

    async fn count_verdicts_by_kind(
        &self,
    ) -> Result<std::collections::HashMap<String, u64>, StorageError> {
        let s = self.state.lock().await;
        let mut map = HashMap::new();
        for v in &s.verdicts {
            *map.entry(format!("{:?}", v.kind).to_lowercase())
                .or_insert(0) += 1;
        }
        Ok(map)
    }
}

#[async_trait]
impl CrystalStorage for InMemoryStorage {
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
        self.list_crystals_filtered(limit, None, None).await
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
                state.is_none_or(|st| format!("{:?}", c.state).to_lowercase() == st.to_lowercase())
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

    async fn update_crystal_relations(
        &self,
        id: &str,
        relations: std::collections::BTreeMap<String, String>,
    ) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        if let Some(c) = s.crystals.get_mut(id) {
            c.relations = relations;
        }
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

    async fn count_crystal_observations(&self) -> Result<u64, StorageError> {
        let s = self.state.lock().await;
        Ok(s.crystals.values().map(|c| c.observations as u64).sum())
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
            contributing_verdicts: vec![],
            certainty: 0.0,
            variance_m2: 0.0,
            mean_quorum: 0.0,
            howl_count: 0,
            wag_count: 0,
            growl_count: 0,
            bark_count: 0,
            source_diversity: 0,
            relations: std::collections::BTreeMap::new(),
            embedding: None,
            contributing_sources: std::collections::BTreeMap::new(),
            shattered_at: None,
            shatter_reason: None,
            shatter_source: None,
        });

        if score > crystal.confidence {
            crystal.content = sanitized;
        }

        let old_mean = crystal.confidence;
        let old_obs = crystal.observations as f64;
        let new_conf = if crystal.observations > 0 {
            (old_mean * old_obs + score) / (old_obs + 1.0)
        } else {
            score
        };
        let delta = score - old_mean;
        let delta2 = score - new_conf;
        crystal.variance_m2 = if crystal.observations > 0 {
            crystal.variance_m2 + delta * delta2
        } else {
            0.0
        };
        crystal.confidence = new_conf;
        crystal.observations += 1;

        crystal.mean_quorum = if crystal.observations > 1 {
            (crystal.mean_quorum * old_obs + voter_count as f64) / crystal.observations as f64
        } else {
            voter_count as f64
        };

        match verdict_kind {
            "howl" => crystal.howl_count += 1,
            "wag" => crystal.wag_count += 1,
            "growl" => crystal.growl_count += 1,
            "bark" => crystal.bark_count += 1,
            _ => crystal.howl_count += 1,
        }

        crystal.updated_at = timestamp.to_string();
        crystal.source_diversity = crystal.contributing_sources.len() as u32;
        crystal.certainty = crate::domain::ccm::compute_certainty(
            crystal.variance_m2,
            crystal.observations,
            crystal.source_diversity,
            1.0,
        );
        crystal.state = crate::domain::ccm::classify(crystal.certainty, crystal.observations);

        if !verdict_id.is_empty()
            && crystal.contributing_verdicts.len() < crate::domain::ccm::MAX_CONTRIBUTING_VERDICTS
            && !crystal
                .contributing_verdicts
                .contains(&verdict_id.to_string())
        {
            crystal.contributing_verdicts.push(verdict_id.to_string());
        }

        Ok(())
    }

    async fn observe_crystal_hypha(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: &str,
        source: &str,
        sentiment: Option<&str>,
    ) -> Result<(), StorageError> {
        let sanitized = sanitize_crystal_content(content);
        let mut s = self.state.lock().await;

        if s.crystals
            .get(id)
            .is_some_and(|c| c.state == CrystalState::Dissolved)
        {
            return Err(StorageError::QueryFailed(format!(
                "crystal {id} is dissolved — hypha observation rejected"
            )));
        }

        let crystal = s.crystals.entry(id.to_string()).or_insert_with(|| Crystal {
            id: id.to_string(),
            content: sanitized.clone(),
            domain: domain.to_string(),
            confidence: 0.0,
            observations: 0,
            state: CrystalState::Forming,
            created_at: timestamp.to_string(),
            updated_at: timestamp.to_string(),
            contributing_verdicts: vec![],
            certainty: 0.0,
            variance_m2: 0.0,
            mean_quorum: 0.0,
            howl_count: 0,
            wag_count: 0,
            growl_count: 0,
            bark_count: 0,
            source_diversity: 0,
            relations: std::collections::BTreeMap::new(),
            embedding: None,
            contributing_sources: std::collections::BTreeMap::new(),
            shattered_at: None,
            shatter_reason: None,
            shatter_source: None,
        });

        let old_mean = crystal.confidence;
        if score > old_mean && !content.is_empty() {
            crystal.content = sanitized;
        }

        let old_obs = crystal.observations as f64;
        let new_conf = if crystal.observations > 0 {
            (old_mean * old_obs + score) / (old_obs + 1.0)
        } else {
            score
        };
        let delta = score - old_mean;
        let delta2 = score - new_conf;
        crystal.variance_m2 = if crystal.observations > 0 {
            crystal.variance_m2 + delta * delta2
        } else {
            0.0
        };
        crystal.confidence = new_conf;
        crystal.observations += 1;

        match sentiment {
            Some("positive") => crystal.wag_count += 1,
            Some("negative") => crystal.growl_count += 1,
            _ => {}
        }

        *crystal
            .contributing_sources
            .entry(source.to_string())
            .or_insert(0) += 1;
        crystal.updated_at = timestamp.to_string();
        crystal.source_diversity = crystal.contributing_sources.len() as u32;
        crystal.certainty = crate::domain::ccm::compute_certainty(
            crystal.variance_m2,
            crystal.observations,
            crystal.source_diversity,
            1.0,
        );
        crystal.state = crate::domain::ccm::classify(crystal.certainty, crystal.observations);

        Ok(())
    }

    async fn shatter_crystal(
        &self,
        id: &str,
        reason: &str,
        source: &str,
        timestamp: &str,
    ) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        match s.crystals.get_mut(id) {
            Some(crystal) => {
                if crystal.state == CrystalState::Dissolved {
                    return Ok(());
                }
                crystal.state = CrystalState::Dissolved;
                crystal.shattered_at = Some(timestamp.to_string());
                crystal.shatter_reason = Some(reason.to_string());
                crystal.shatter_source = Some(source.to_string());
                crystal.updated_at = timestamp.to_string();
                Ok(())
            }
            None => Err(StorageError::QueryFailed(format!(
                "shatter_crystal: crystal {id} not found"
            ))),
        }
    }

    async fn store_crystal_embedding(
        &self,
        id: &str,
        embedding: &[f32],
    ) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        match s.crystals.get_mut(id) {
            Some(crystal) => {
                crystal.embedding = Some(embedding.to_vec());
                Ok(())
            }
            None => Err(StorageError::QueryFailed(format!(
                "store_crystal_embedding: crystal {id} not found"
            ))),
        }
    }

    async fn search_crystals_semantic(
        &self,
        _query_embedding: &[f32],
        _limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        Ok(vec![])
    }

    async fn find_similar_crystal(
        &self,
        _embedding: &[f32],
        _domain: &str,
        _threshold: f64,
    ) -> Result<Option<(String, f64)>, StorageError> {
        Ok(None)
    }

    async fn list_crystals_missing_embedding(
        &self,
        _limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.crystals
            .values()
            .filter(|c| c.embedding.is_none())
            .cloned()
            .collect())
    }

    async fn consolidate_duplicate_crystals(&self) -> Result<u64, StorageError> {
        Ok(0)
    }
}

#[async_trait]
impl ActivityStorage for InMemoryStorage {
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
            tags: obs.tags.clone(),
            value: obs.value.clone(),
            confidence: obs.confidence.clone(),
            consumer: obs.consumer.clone(),
            action: obs.action.clone(),
            depends_on: obs.depends_on.clone(),
            maturity: obs.maturity,
            hash: obs.hash.clone(),
            prev_hash: obs.prev_hash.clone(),
            observers: obs.observers.clone(),
            consensus_score: obs.consensus_score,
            source_tier: obs.source_tier.clone(),
        });
        Ok(())
    }

    async fn store_event(&self, event: &Event) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        let _id = format!("event:{}", s.events.len());
        s.events.push(event.clone());
        Ok(())
    }

    async fn fleet_stats(
        &self,
        _window_secs: u64,
        _limit: u32,
    ) -> Result<Vec<(String, u64, f64, u64, String)>, StorageError> {
        Ok(vec![])
    }

    async fn list_events(
        &self,
        _node: Option<&str>,
        _tool: Option<&str>,
        _limit: u32,
    ) -> Result<Vec<RawEvent>, StorageError> {
        Ok(vec![])
    }

    async fn list_degraded_nodes(
        &self,
        _window_secs: u64,
        _fatal_threshold: f64,
    ) -> Result<Vec<(String, String, u64, u64)>, StorageError> {
        Ok(vec![])
    }

    async fn list_observations_raw(
        &self,
        domain: Option<&str>,
        agent_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.observations
            .iter()
            .rev()
            .filter(|o| domain.is_none_or(|d| o.domain == d))
            .filter(|o| agent_id.is_none_or(|a| o.agent_id == a))
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn list_observations_by_target(
        &self,
        domain: &str,
        target: &str,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.observations
            .iter()
            .rev()
            .filter(|o| o.domain == domain && o.target == target)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn list_observations_by_tag(
        &self,
        _domain: &str,
        _tag: &str,
        _limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        Ok(vec![])
    }

    async fn last_observation_per_source(
        &self,
    ) -> Result<Vec<(String, String, u64)>, StorageError> {
        Ok(vec![])
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
        result.sort_by_key(|r| std::cmp::Reverse(r.2));
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

    async fn store_session_compliance(
        &self,
        _compliance: &crate::domain::compliance::SessionCompliance,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    async fn list_session_compliance(
        &self,
        _limit: u32,
    ) -> Result<Vec<crate::domain::compliance::SessionCompliance>, StorageError> {
        Ok(vec![])
    }

    async fn zone_activity(
        &self,
        _path_prefixes: &[String],
        _exclude_agent: &str,
        _project_root: &str,
    ) -> Result<Vec<crate::api::rest::dispatch::AgentActivity>, StorageError> {
        Ok(vec![])
    }

    async fn count_observations(&self) -> Result<u64, StorageError> {
        let s = self.state.lock().await;
        Ok(s.observations.len() as u64)
    }
}

#[async_trait]
impl TaskStorage for InMemoryStorage {
    async fn store_agent_task(&self, task: &AgentTask) -> Result<String, StorageError> {
        let mut s = self.state.lock().await;
        let id = task.id.clone();
        s.tasks.insert(id.clone(), task.clone());
        Ok(id)
    }

    async fn list_pending_agent_tasks(
        &self,
        kind: &str,
        limit: u32,
    ) -> Result<Vec<AgentTask>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s
            .tasks
            .values()
            .filter(|t| t.kind == kind && t.status == "pending")
            .cloned()
            .collect();
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn list_completed_agent_tasks(
        &self,
        kind: &str,
        limit: u32,
    ) -> Result<Vec<AgentTask>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s
            .tasks
            .values()
            .filter(|t| t.kind == kind && t.status == "completed")
            .cloned()
            .collect();
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn mark_agent_task_processing(&self, task_id: &str) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        if let Some(t) = s.tasks.get_mut(task_id) {
            t.status = "processing".into();
        }
        Ok(())
    }

    async fn update_agent_task_result(
        &self,
        task_id: &str,
        result: Option<String>,
        error: Option<String>,
    ) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        if let Some(t) = s.tasks.get_mut(task_id) {
            t.status = if error.is_some() {
                "failed".into()
            } else {
                "completed".into()
            };
            t.result = result;
            t.error = error;
        }
        Ok(())
    }
}

#[async_trait]
impl DispatchStorage for InMemoryStorage {
    async fn store_agent_dispatch(
        &self,
        _dispatch: &AgentDispatch,
    ) -> Result<String, StorageError> {
        Ok("".into())
    }
    async fn get_active_dispatch_for_scope(
        &self,
        _scope: &str,
    ) -> Result<Option<AgentDispatch>, StorageError> {
        Ok(None)
    }
    async fn get_dispatch(
        &self,
        _dispatch_id: &str,
    ) -> Result<Option<AgentDispatch>, StorageError> {
        Ok(None)
    }
    async fn get_active_dispatches_for_agent(
        &self,
        _agent_id: &str,
    ) -> Result<Vec<AgentDispatch>, StorageError> {
        Ok(vec![])
    }
    async fn update_dispatch_status(
        &self,
        _dispatch_id: &str,
        _new_status: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn update_dispatch_pr(
        &self,
        _dispatch_id: &str,
        _pr_number: u32,
    ) -> Result<(), StorageError> {
        Ok(())
    }
}

#[async_trait]
impl StateLogStorage for InMemoryStorage {
    async fn store_state_block(
        &self,
        _block: &crate::domain::state_log::StateBlock,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn last_state_block(
        &self,
    ) -> Result<Option<crate::domain::state_log::StateBlock>, StorageError> {
        Ok(None)
    }
    async fn list_state_blocks(
        &self,
        _since: &str,
        _limit: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
        Ok(vec![])
    }
    async fn latest_state_blocks(
        &self,
        _n: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
        Ok(vec![])
    }
}

#[async_trait]
impl SubmissionStorage for InMemoryStorage {
    async fn enqueue_verdict(
        &self,
        _verdict_id: &str,
        _content_hash: &str,
        _q_score: f64,
        _score_fidelity: f64,
        _score_phi: f64,
        _score_verify: f64,
        _score_culture: f64,
        _score_burn: f64,
        _score_sovereignty: f64,
        _dog_count: u32,
        _verdict_type: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn list_pending_verdicts(&self, _limit: u32) -> Result<Vec<QueuedVerdict>, StorageError> {
        Ok(vec![])
    }
    async fn list_submitted_verdicts(
        &self,
        _limit: u32,
    ) -> Result<Vec<QueuedVerdict>, StorageError> {
        Ok(vec![])
    }
    async fn get_queued_verdict(
        &self,
        _verdict_id: &str,
    ) -> Result<Option<QueuedVerdict>, StorageError> {
        Ok(None)
    }
    async fn update_verdict_submitted(
        &self,
        _verdict_id: &str,
        _tx_signature: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn update_verdict_confirmed(&self, _verdict_id: &str) -> Result<(), StorageError> {
        Ok(())
    }
    async fn update_verdict_failed(
        &self,
        _verdict_id: &str,
        _error_reason: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn queue_status_counts(&self) -> Result<(u32, u32, u32, u32), StorageError> {
        Ok((0, 0, 0, 0))
    }
}

#[async_trait]
impl StoragePort for InMemoryStorage {
    async fn ping(&self) -> Result<(), StorageError> {
        Ok(())
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

    async fn flush_dog_stats(&self, stats: &[(String, DogStats)]) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        for (dog_id, stat) in stats {
            s.dog_stats.insert(dog_id.clone(), stat.clone());
        }
        Ok(())
    }

    async fn load_dog_stats(
        &self,
    ) -> Result<Vec<(String, crate::domain::dog_health::DogStats)>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.dog_stats
            .iter()
            .map(|(id, stat)| (id.clone(), stat.clone()))
            .collect())
    }

    async fn load_dog_stat(
        &self,
        dog_id: &str,
    ) -> Result<Option<crate::domain::dog_health::DogStats>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.dog_stats.get(dog_id).cloned())
    }
}

#[cfg(test)]
mod crystal_hypha_tests {
    use super::*;

    #[tokio::test]
    async fn hypha_observe_creates_forming_crystal() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "insight",
            "hermes-skill",
            0.5,
            "2026-05-15T00:00:00Z",
            "hermes-agent",
            None,
        )
        .await
        .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.state, CrystalState::Forming);
        assert_eq!(c.observations, 1);
        assert_eq!(c.source_diversity, 1);
    }

    #[tokio::test]
    async fn hypha_observe_feeds_existing_crystal() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "insight",
            "hermes-skill",
            0.5,
            "2026-05-15T00:00:00Z",
            "hermes-agent",
            None,
        )
        .await
        .unwrap();
        s.observe_crystal_hypha(
            "test-1",
            "",
            "hermes-skill",
            0.6,
            "2026-05-15T01:00:00Z",
            "hermes-agent",
            None,
        )
        .await
        .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.observations, 2);
        assert!((c.confidence - 0.55).abs() < 0.01);
    }

    #[tokio::test]
    async fn hypha_observe_tracks_multiple_sources() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "insight",
            "skill",
            0.5,
            "2026-05-15T00:00:00Z",
            "hermes",
            None,
        )
        .await
        .unwrap();
        s.observe_crystal_hypha(
            "test-1",
            "",
            "skill",
            0.6,
            "2026-05-15T01:00:00Z",
            "claude",
            None,
        )
        .await
        .unwrap();
        s.observe_crystal_hypha(
            "test-1",
            "",
            "skill",
            0.55,
            "2026-05-15T02:00:00Z",
            "gemini",
            None,
        )
        .await
        .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.source_diversity, 3);
    }

    #[tokio::test]
    async fn hypha_observe_rejected_on_dissolved() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "insight",
            "skill",
            0.5,
            "2026-05-15T00:00:00Z",
            "hermes",
            None,
        )
        .await
        .unwrap();
        s.shatter_crystal("test-1", "rug", "admin", "2026-05-15T01:00:00Z")
            .await
            .unwrap();
        let result = s
            .observe_crystal_hypha(
                "test-1",
                "",
                "skill",
                0.6,
                "2026-05-15T02:00:00Z",
                "hermes",
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn shatter_transitions_to_dissolved() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "insight",
            "skill",
            0.5,
            "2026-05-15T00:00:00Z",
            "hermes",
            None,
        )
        .await
        .unwrap();
        s.shatter_crystal("test-1", "rug", "watch", "2026-05-15T01:00:00Z")
            .await
            .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.state, CrystalState::Dissolved);
    }

    #[tokio::test]
    async fn sentiment_maps_to_polarity() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "good",
            "skill",
            0.7,
            "2026-05-15T00:00:00Z",
            "hermes",
            Some("positive"),
        )
        .await
        .unwrap();
        s.observe_crystal_hypha(
            "test-1",
            "",
            "skill",
            0.3,
            "2026-05-15T01:00:00Z",
            "hermes",
            Some("negative"),
        )
        .await
        .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.wag_count, 1);
        assert_eq!(c.growl_count, 1);
    }

    #[tokio::test]
    async fn content_updates_on_higher_score() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "v1",
            "skill",
            0.5,
            "2026-05-15T00:00:00Z",
            "hermes",
            None,
        )
        .await
        .unwrap();
        s.observe_crystal_hypha(
            "test-1",
            "v2",
            "skill",
            0.8,
            "2026-05-15T01:00:00Z",
            "hermes",
            None,
        )
        .await
        .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.content, "v2");
    }

    #[tokio::test]
    async fn content_does_not_update_on_lower_score() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "v1",
            "skill",
            0.8,
            "2026-05-15T00:00:00Z",
            "hermes",
            None,
        )
        .await
        .unwrap();
        s.observe_crystal_hypha(
            "test-1",
            "",
            "skill",
            0.3,
            "2026-05-15T01:00:00Z",
            "hermes",
            None,
        )
        .await
        .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.content, "v1");
    }

    #[tokio::test]
    async fn shatter_nonexistent_crystal_errors() {
        let s = InMemoryStorage::new();
        let result = s.shatter_crystal("no", "no", "no", "now").await;
        assert!(result.is_err());
    }
}
