//! In-memory judge job store for async/progressive evaluation.
//!
//! `POST /judge/async` spawns a background evaluation and returns a `request_id`.
//! `GET /judge/status/{id}` reads the progressive state: Dogs arrive one by one.
//!
//! Design: bounded (MAX_JOBS), TTL auto-cleanup (JOB_TTL_SECS), zero persistence.
//! Jobs are ephemeral — restarts lose them. The canonical verdict is in storage.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use super::judge::validate_judge_request;
use super::response::{dog_score_to_response, verdict_response_cached, verdict_to_response};
use super::types::{AppState, DogScoreResponse, ErrorResponse, JudgeRequest, JudgeResponse};
use crate::domain::dog::Stimulus;

use crate::domain::constants;

/// Max concurrent jobs — prevents memory exhaustion under load.
const MAX_JOBS: usize = constants::MAX_JUDGE_JOBS;
/// Jobs expire after 5 minutes — covers slowest Dog timeout + client polling window.
const JOB_TTL_SECS: u64 = constants::JUDGE_JOB_TTL.as_secs();

/// A progressive judge job tracked in memory.
#[derive(Debug)]
pub struct JudgeJob {
    pub status: JobStatus,
    pub dogs_total: usize,
    pub dogs_arrived: Vec<DogArrival>,
    pub verdict: Option<JudgeResponse>,
    pub error: Option<String>,
    pub created_at: Instant,
}

/// Progressive Dog arrival — one entry per Dog response (success or failure).
#[derive(Debug, Clone, Serialize)]
pub struct DogArrival {
    pub dog_id: String,
    pub arrived_at_ms: u64,
    pub success: bool,
    /// Present only on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<DogScoreResponse>,
    /// Present only on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    /// Spawned, waiting for Dogs.
    Pending,
    /// At least one Dog has arrived but not all.
    Evaluating,
    /// All Dogs responded, verdict aggregated.
    Complete,
    /// Pipeline error — no verdict produced.
    Failed,
}

impl JobStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Evaluating => "evaluating",
            Self::Complete => "complete",
            Self::Failed => "failed",
        }
    }
}

/// GET /judge/status response — progressive snapshot.
#[derive(Debug, Serialize)]
pub struct JudgeStatusResponse {
    pub request_id: String,
    pub status: String,
    pub dogs_total: usize,
    pub dogs_arrived: Vec<DogArrival>,
    /// Present only when status == "complete".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<JudgeResponse>,
    /// Present only when status == "failed".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// POST /judge/async response — background evaluation accepted.
#[derive(Debug, Serialize)]
pub struct JudgeAsyncAcceptedResponse {
    pub request_id: String,
    pub status: String,
    pub dogs_total: usize,
}

#[derive(Debug, Clone)]
pub struct JudgeJobSnapshot {
    pub status: JobStatus,
    pub dogs_total: usize,
    pub dogs_arrived: Vec<DogArrival>,
    pub verdict: Option<JudgeResponse>,
    pub error: Option<String>,
}

/// In-memory store for async judge jobs.
/// RwLock: reads are frequent (polling), writes are infrequent (Dog arrivals).
pub struct JudgeJobStore {
    jobs: RwLock<HashMap<String, JudgeJob>>,
}

impl std::fmt::Debug for JudgeJobStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.jobs.read().map_or(0, |j| j.len());
        f.debug_struct("JudgeJobStore")
            .field("active_jobs", &count)
            .finish()
    }
}

impl Default for JudgeJobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl JudgeJobStore {
    pub fn new() -> Self {
        Self {
            jobs: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new job. Returns false if at capacity (MAX_JOBS).
    pub fn create(&self, request_id: &str, dogs_total: usize) -> bool {
        let Ok(mut jobs) = self.jobs.write() else {
            return false; // WHY: K14 — poisoned lock = assume degraded
        };
        // Evict expired jobs before checking capacity
        let now = Instant::now();
        jobs.retain(|_, job| now.duration_since(job.created_at).as_secs() < JOB_TTL_SECS);
        if jobs.len() >= MAX_JOBS {
            return false;
        }
        jobs.insert(
            request_id.to_string(),
            JudgeJob {
                status: JobStatus::Pending,
                dogs_total,
                dogs_arrived: Vec::new(),
                verdict: None,
                error: None,
                created_at: now,
            },
        );
        true
    }

    /// Record a Dog arrival (success or failure). Updates job status.
    pub fn record_arrival(&self, request_id: &str, arrival: DogArrival) {
        let Ok(mut jobs) = self.jobs.write() else {
            return; // WHY: K14 — poisoned lock, best-effort
        };
        let Some(job) = jobs.get_mut(request_id) else {
            return;
        };
        job.dogs_arrived.push(arrival);
        if job.dogs_arrived.len() >= job.dogs_total {
            // All dogs responded — mark evaluating (aggregation happens next)
            job.status = JobStatus::Evaluating;
        } else if job.status == JobStatus::Pending {
            job.status = JobStatus::Evaluating;
        }
    }

    /// Mark job as complete with the final verdict response.
    pub fn complete(&self, request_id: &str, verdict: JudgeResponse) {
        let Ok(mut jobs) = self.jobs.write() else {
            return;
        };
        if let Some(job) = jobs.get_mut(request_id) {
            job.status = JobStatus::Complete;
            job.verdict = Some(verdict);
            job.error = None;
        }
    }

    /// Mark job as failed.
    pub fn fail(&self, request_id: &str, error: String) {
        let Ok(mut jobs) = self.jobs.write() else {
            return;
        };
        if let Some(job) = jobs.get_mut(request_id) {
            job.status = JobStatus::Failed;
            job.error = Some(error);
        }
    }

    /// Read job state for status polling.
    pub fn get(&self, request_id: &str) -> Option<JudgeJobSnapshot> {
        let jobs = self.jobs.read().ok()?; // WHY: K14 — poisoned = None = 404
        let job = jobs.get(request_id)?;
        Some(JudgeJobSnapshot {
            status: job.status.clone(),
            dogs_total: job.dogs_total,
            dogs_arrived: job.dogs_arrived.clone(),
            verdict: job.verdict.clone(),
            error: job.error.clone(),
        })
    }
}

pub async fn judge_async_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JudgeRequest>,
) -> Result<(StatusCode, Json<JudgeAsyncAcceptedResponse>), (StatusCode, Json<ErrorResponse>)> {
    let req = validate_judge_request(req)?;

    let request_id = uuid::Uuid::new_v4().to_string();
    let stimulus = Stimulus {
        content: req.content.clone(),
        context: req.context.clone(),
        domain: req.domain.clone(),
        request_id: None,
    };
    let judge = state.judge.load_full();
    let dogs_total = judge
        .runnable_dog_count(&stimulus, req.dogs.as_deref())
        .map_err(|e| {
            let status = match &e {
                crate::judge::JudgeError::InvalidInput(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (
                status,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    if !state.judge_jobs.create(&request_id, dogs_total) {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "judge async queue full".into(),
            }),
        ));
    }

    let semaphore = Arc::clone(&state.bg_semaphore);
    let Ok(permit) = semaphore.try_acquire_owned() else {
        state.judge_jobs.fail(
            &request_id,
            "judge async queue saturated: background task limit reached".into(),
        );
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "judge async queue saturated: background task limit reached".into(),
            }),
        ));
    };

    let job_store = Arc::clone(&state.judge_jobs);
    let request_id_for_task = request_id.clone();
    let storage = Arc::clone(&state.storage);
    let embedding = Arc::clone(&state.embedding);
    let usage = Arc::clone(&state.usage);
    let verdict_cache = Arc::clone(&state.verdict_cache);
    let metrics = Arc::clone(&state.metrics);
    let event_tx = state.event_tx.clone();
    let enricher = state.enricher.clone();

    state.bg_tasks.spawn(async move {
        let _permit = permit;
        let on_dog = {
            let job_store = Arc::clone(&job_store);
            let request_id = request_id_for_task.clone();
            Box::new(
                move |dog_id: &str,
                      success: bool,
                      elapsed_ms: u64,
                      score: Option<&crate::domain::dog::DogScore>,
                      error: Option<String>| {
                    job_store.record_arrival(
                        &request_id,
                        DogArrival {
                            dog_id: dog_id.to_string(),
                            arrived_at_ms: elapsed_ms,
                            success,
                            score: score.map(dog_score_to_response),
                            error: (!success).then(|| {
                                error.unwrap_or_else(|| "dog evaluation failed".into())
                            }),
                        },
                    );
                },
            )
        };

        let deps = crate::pipeline::PipelineDeps {
            judge: judge.as_ref(),
            storage: storage.as_ref(),
            embedding: embedding.as_ref(),
            usage: usage.as_ref(),
            verdict_cache: verdict_cache.as_ref(),
            metrics: metrics.as_ref(),
            event_tx: Some(&event_tx),
            request_id: Some(request_id_for_task.clone()),
            on_dog: Some(on_dog),
            expected_dog_count: judge.dog_ids().len(),
            enricher: enricher.as_deref(),
        };

        let result = crate::pipeline::run(
            req.content,
            req.context,
            req.domain,
            req.dogs.as_deref(),
            req.crystals,
            &deps,
        )
        .await;

        match result {
            Ok(crate::pipeline::PipelineResult::CacheHit {
                verdict,
                similarity,
            }) => {
                job_store.complete(
                    &request_id_for_task,
                    verdict_response_cached(verdict.as_ref(), similarity),
                );
            }
            Ok(crate::pipeline::PipelineResult::Evaluated {
                verdict,
                token_data,
                enriched_content,
            }) => {
                let mut resp = verdict_to_response(verdict.as_ref());
                resp.token_data = token_data.map(|b| *b);
                resp.stimulus_content = enriched_content;
                job_store.complete(&request_id_for_task, resp);
            }
            Err(e) => {
                tracing::error!(request_id = %request_id_for_task, error = %e, "judge async pipeline failed");
                job_store.fail(&request_id_for_task, e.to_string());
            }
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(JudgeAsyncAcceptedResponse {
            request_id,
            status: "pending".into(),
            dogs_total,
        }),
    ))
}

pub async fn judge_status_handler(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<String>,
) -> Result<Json<JudgeStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let Some(snapshot) = state.judge_jobs.get(&request_id) else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("judge job {request_id} not found"),
            }),
        ));
    };

    Ok(Json(JudgeStatusResponse {
        request_id,
        status: snapshot.status.as_str().into(),
        dogs_total: snapshot.dogs_total,
        dogs_arrived: snapshot.dogs_arrived,
        verdict: snapshot.verdict,
        error: snapshot.error,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_get_job() {
        let store = JudgeJobStore::new();
        assert!(store.create("req-1", 3));
        let snapshot = store.get("req-1").unwrap();
        assert_eq!(snapshot.status, JobStatus::Pending);
        assert_eq!(snapshot.dogs_total, 3);
        assert!(snapshot.dogs_arrived.is_empty());
        assert!(snapshot.verdict.is_none());
        assert!(snapshot.error.is_none());
    }

    #[test]
    fn record_arrival_transitions_to_evaluating() {
        let store = JudgeJobStore::new();
        store.create("req-2", 2);
        store.record_arrival(
            "req-2",
            DogArrival {
                dog_id: "deterministic-dog".into(),
                arrived_at_ms: 5,
                success: true,
                score: None,
                error: None,
            },
        );
        let snapshot = store.get("req-2").unwrap();
        assert_eq!(snapshot.status, JobStatus::Evaluating);
        assert_eq!(snapshot.dogs_arrived.len(), 1);
    }

    #[test]
    fn complete_marks_done() {
        let store = JudgeJobStore::new();
        store.create("req-3", 1);
        store.record_arrival(
            "req-3",
            DogArrival {
                dog_id: "det".into(),
                arrived_at_ms: 1,
                success: true,
                score: None,
                error: None,
            },
        );
        store.complete(
            "req-3",
            JudgeResponse {
                verdict_id: "v1".into(),
                domain: "general".into(),
                verdict: "Wag".into(),
                q_score: super::super::types::QScoreResponse {
                    total: 0.4,
                    fidelity: 0.4,
                    phi: 0.4,
                    verify: 0.4,
                    culture: 0.4,
                    burn: 0.4,
                    sovereignty: 0.4,
                },
                reasoning: super::super::types::ReasoningResponse {
                    fidelity: "ok".into(),
                    phi: "ok".into(),
                    verify: "ok".into(),
                    culture: "ok".into(),
                    burn: "ok".into(),
                    sovereignty: "ok".into(),
                },
                dogs_used: "det".into(),
                phi_max: crate::domain::dog::PHI_INV,
                timestamp: "2026-04-17T00:00:00Z".into(),
                dog_scores: vec![],
                voter_count: 1,
                anomaly_detected: false,
                max_disagreement: 0.0,
                anomaly_axiom: None,
                integrity_hash: None,
                prev_hash: None,
                cache_hit: None,
                token_data: None,
                stimulus_content: None,
                failed_dogs: Vec::new(),
                failed_dog_errors: Default::default(),
            },
        );
        let snapshot = store.get("req-3").unwrap();
        assert_eq!(snapshot.status, JobStatus::Complete);
        assert_eq!(
            snapshot.verdict.as_ref().map(|v| v.verdict_id.as_str()),
            Some("v1")
        );
    }

    #[test]
    fn unknown_job_returns_none() {
        let store = JudgeJobStore::new();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn fail_records_error() {
        let store = JudgeJobStore::new();
        store.create("req-4", 1);
        store.fail("req-4", "boom".into());
        let snapshot = store.get("req-4").unwrap();
        assert_eq!(snapshot.status, JobStatus::Failed);
        assert_eq!(snapshot.error.as_deref(), Some("boom"));
    }

    #[test]
    fn capacity_enforced() {
        let store = JudgeJobStore::new();
        for i in 0..MAX_JOBS {
            assert!(store.create(&format!("job-{i}"), 1));
        }
        assert!(!store.create("overflow", 1));
    }
}
