//! Domain-Partitioned Task Queue & State Manager.
//!
//! Provides a thread-safe, persistent task queue partitioned by operational domain
//! (Inference, Infra, CodeAudit, Cortex, Governance, Nightshift)
//! with priority scheduling, lease attribution, and durable result recording.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cynic_protocol::{DomainTask, DomainTaskStatus, TaskDomain, TaskPriority};
use thiserror::Error;
use tokio::sync::RwLock;

pub const DEFAULT_DOMAIN_TASKS_PATH: &str = "/var/lib/cynic/evidence/domain_tasks.jsonl";

#[derive(Debug, Error)]
pub enum DomainTaskQueueError {
    #[error("I/O error accessing domain tasks file at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Serialization error in domain task queue: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Task '{task_id}' not found in domain queue")]
    TaskNotFound { task_id: String },

    #[error("Task '{task_id}' is already claimed by agent '{claimed_by}'")]
    AlreadyClaimed { task_id: String, claimed_by: String },

    #[error("Task '{task_id}' is not in claimable state (current: {status:?})")]
    InvalidState {
        task_id: String,
        status: DomainTaskStatus,
    },

    #[error("task admission refused: {error}")]
    AdmissionRequired {
        task_title: String,
        error: crate::infra::task_admission_port::TaskAdmissionError,
    },
}

pub struct DomainTaskQueue {
    storage_path: PathBuf,
    tasks: Arc<RwLock<BTreeMap<String, DomainTask>>>,
    /// Optional admission authority (plan §10 step 3). None = legacy
    /// behaviour (unbudgeted push always admitted). When set, every push
    /// passes through the EnergyBudget gate before touching storage.
    admission: Option<std::sync::Arc<dyn crate::infra::task_admission_port::TaskAdmissionPort>>,
}

impl std::fmt::Debug for DomainTaskQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DomainTaskQueue")
            .field("storage_path", &self.storage_path)
            .field("admission_bound", &self.admission.is_some())
            .finish_non_exhaustive()
    }
}
impl DomainTaskQueue {
    /// Open or initialize a domain task queue at the specified file path.
    pub fn open(storage_path: impl Into<PathBuf>) -> Result<Self, DomainTaskQueueError> {
        let path = storage_path.into();
        let mut loaded = BTreeMap::new();

        if path.exists() {
            let file = File::open(&path).map_err(|e| DomainTaskQueueError::Io {
                path: path.display().to_string(),
                source: e,
            })?;
            let reader = BufReader::new(file);

            for line_res in reader.lines() {
                let line = line_res.map_err(|e| DomainTaskQueueError::Io {
                    path: path.display().to_string(),
                    source: e,
                })?;
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }

                if let Ok(task) = serde_json::from_str::<DomainTask>(trimmed) {
                    loaded.insert(task.id.clone(), task);
                }
            }
        }

        Ok(Self {
            storage_path: path,
            tasks: Arc::new(RwLock::new(loaded)),
            admission: None,
        })
    }

    /// Bind an admission authority: every subsequent push passes the
    /// EnergyBudget gate. Idempotent per instance (last binding wins).
    pub fn with_admission(
        mut self,
        admission: std::sync::Arc<dyn crate::infra::task_admission_port::TaskAdmissionPort>,
    ) -> Self {
        self.admission = Some(admission);
        self
    }

    /// Open with the default system evidence path or user directory fallback.
    pub fn open_default() -> Result<Self, DomainTaskQueueError> {
        let default_path = Path::new(DEFAULT_DOMAIN_TASKS_PATH);
        if default_path.exists() || default_path.parent().map(|p| p.exists()).unwrap_or(false) {
            Self::open(default_path)
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            let user_fallback = PathBuf::from(home).join(".cynic/evidence/domain_tasks.jsonl");
            Self::open(user_fallback)
        }
    }

    /// Push a new task to the queue and persist to disk.
    ///
    /// When an admission authority is bound, the payload passes the
    /// EnergyBudget gate first: Unknown/Blocked refuse the push with
    /// `AdmissionRequired` carrying the honest reasons. Unbudgeted
    /// payloads admit freely (V1 semantics).
    pub async fn push_task(
        &self,
        domain: TaskDomain,
        priority: TaskPriority,
        title: impl Into<String>,
        payload: serde_json::Value,
        created_by: impl Into<String>,
    ) -> Result<DomainTask, DomainTaskQueueError> {
        if let Some(port) = &self.admission {
            if let Some(error) =
                crate::infra::task_admission_port::TaskAdmissionError::from_decision(
                    &port.admit(&payload),
                )
            {
                return Err(DomainTaskQueueError::AdmissionRequired {
                    task_title: title.into(),
                    error,
                });
            }
        }
        self.push_task_unchecked(domain, priority, title, payload, created_by)
            .await
    }

    /// Storage-only push, bypassing admission. Used internally after the
    /// gate and available to trusted internal callers that have already
    /// been admitted through another boundary.
    pub async fn push_task_unchecked(
        &self,
        domain: TaskDomain,
        priority: TaskPriority,
        title: impl Into<String>,
        payload: serde_json::Value,
        created_by: impl Into<String>,
    ) -> Result<DomainTask, DomainTaskQueueError> {
        static TASK_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let seq = TASK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let id = format!("task_{:?}_{}_{}", domain, now, seq).to_lowercase();
        let task = DomainTask::new(id, domain, priority, title, payload, created_by);
        let mut guard = self.tasks.write().await;
        self.append_record(&task)?;
        guard.insert(task.id.clone(), task.clone());
        Ok(task)
    }

    /// Claim the highest-priority pending task for a domain.
    pub async fn claim_task(
        &self,
        domain: TaskDomain,
        agent_id: impl Into<String>,
    ) -> Result<Option<DomainTask>, DomainTaskQueueError> {
        let agent = agent_id.into();
        let mut guard = self.tasks.write().await;

        // Find candidate with highest priority
        let best_id = guard
            .values()
            .filter(|t| t.domain == domain && t.status == DomainTaskStatus::Pending)
            .max_by_key(|t| t.priority)
            .map(|t| t.id.clone());

        if let Some(id) = best_id {
            if let Some(task) = guard.get_mut(&id) {
                task.status = DomainTaskStatus::Claimed;
                task.claimed_by_agent = Some(agent);
                let cloned = task.clone();
                self.rewrite_all_records(&guard)?;
                return Ok(Some(cloned));
            }
        }

        Ok(None)
    }

    /// Complete a task and record its result evidence.
    pub async fn complete_task(
        &self,
        task_id: &str,
        agent_id: &str,
        result_evidence: serde_json::Value,
    ) -> Result<DomainTask, DomainTaskQueueError> {
        let mut guard = self.tasks.write().await;
        let task = guard
            .get_mut(task_id)
            .ok_or_else(|| DomainTaskQueueError::TaskNotFound {
                task_id: task_id.to_string(),
            })?;

        task.status = DomainTaskStatus::Completed;
        task.claimed_by_agent = Some(agent_id.to_string());
        task.result_evidence = Some(result_evidence);
        let cloned = task.clone();
        self.rewrite_all_records(&guard)?;

        Ok(cloned)
    }

    /// Fail a task and record the error reason.
    pub async fn fail_task(
        &self,
        task_id: &str,
        error_details: impl Into<String>,
    ) -> Result<DomainTask, DomainTaskQueueError> {
        let mut guard = self.tasks.write().await;
        let task = guard
            .get_mut(task_id)
            .ok_or_else(|| DomainTaskQueueError::TaskNotFound {
                task_id: task_id.to_string(),
            })?;

        task.status = DomainTaskStatus::Failed;
        task.error_details = Some(error_details.into());
        let cloned = task.clone();
        self.rewrite_all_records(&guard)?;

        Ok(cloned)
    }

    /// List tasks matching optional domain and status filters.
    pub async fn list_tasks(
        &self,
        domain: Option<TaskDomain>,
        status: Option<DomainTaskStatus>,
    ) -> Vec<DomainTask> {
        let guard = self.tasks.read().await;
        guard
            .values()
            .filter(|t| domain.map(|d| t.domain == d).unwrap_or(true))
            .filter(|t| status.map(|s| t.status == s).unwrap_or(true))
            .cloned()
            .collect()
    }

    fn append_record(&self, task: &DomainTask) -> Result<(), DomainTaskQueueError> {
        if let Some(parent) = self.storage_path.parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.storage_path)
            .map_err(|e| DomainTaskQueueError::Io {
                path: self.storage_path.display().to_string(),
                source: e,
            })?;

        let line = serde_json::to_string(task)?;
        writeln!(file, "{line}").map_err(|e| DomainTaskQueueError::Io {
            path: self.storage_path.display().to_string(),
            source: e,
        })?;

        Ok(())
    }

    fn rewrite_all_records(
        &self,
        tasks: &BTreeMap<String, DomainTask>,
    ) -> Result<(), DomainTaskQueueError> {
        if let Some(parent) = self.storage_path.parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let temp_path = self.storage_path.with_extension("tmp");
        let mut file = File::create(&temp_path).map_err(|e| DomainTaskQueueError::Io {
            path: temp_path.display().to_string(),
            source: e,
        })?;

        for task in tasks.values() {
            let line = serde_json::to_string(task)?;
            writeln!(file, "{line}").map_err(|e| DomainTaskQueueError::Io {
                path: temp_path.display().to_string(),
                source: e,
            })?;
        }

        file.flush().map_err(|e| DomainTaskQueueError::Io {
            path: temp_path.display().to_string(),
            source: e,
        })?;

        std::fs::rename(&temp_path, &self.storage_path).map_err(|e| DomainTaskQueueError::Io {
            path: self.storage_path.display().to_string(),
            source: e,
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_domain_task_lifecycle() {
        let temp = NamedTempFile::new().unwrap();
        let queue = DomainTaskQueue::open(temp.path()).unwrap();

        // 1. Push 2 tasks
        let t1 = queue
            .push_task(
                TaskDomain::Inference,
                TaskPriority::Agent,
                "Sweep Qwen 2B 64k",
                serde_json::json!({"model": "qwen2b", "context": "64k"}),
                "omp_agent",
            )
            .await
            .unwrap();

        let _t2 = queue
            .push_task(
                TaskDomain::Inference,
                TaskPriority::Critical,
                "Emergency OOM Remediation",
                serde_json::json!({"action": "drain"}),
                "cortex",
            )
            .await
            .unwrap();

        // 2. Claim task (Critical priority must be claimed first!)
        let claimed = queue
            .claim_task(TaskDomain::Inference, "hermes_worker")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(claimed.priority, TaskPriority::Critical);
        assert_eq!(claimed.status, DomainTaskStatus::Claimed);

        // 3. Complete task
        let completed = queue
            .complete_task(
                &t1.id,
                "omp_agent",
                serde_json::json!({"ttft_ms": 120.0, "tok_s": 24.5}),
            )
            .await
            .unwrap();
        assert_eq!(completed.status, DomainTaskStatus::Completed);

        // 4. Persistence check: Re-open queue and verify items
        let reopened = DomainTaskQueue::open(temp.path()).unwrap();
        let all_tasks = reopened.list_tasks(Some(TaskDomain::Inference), None).await;
        assert_eq!(all_tasks.len(), 2);
    }

    // ── Admission gate tests (plan §10 step 3) ─────────────────────────

    /// Stub port: envelope on inference_tokens, fresh capacity flag settable.
    struct StubAdmission {
        envelope_tokens: u64,
        open: Vec<cynic_protocol::task_pipeline::EnergyBudget>,
        fresh: bool,
    }

    impl crate::infra::task_admission_port::TaskAdmissionPort for StubAdmission {
        fn open_reservations(&self) -> Vec<cynic_protocol::task_pipeline::EnergyBudget> {
            self.open.clone()
        }
        fn envelope(&self) -> cynic_protocol::task_pipeline::EnergyBudget {
            cynic_protocol::task_pipeline::EnergyBudget {
                wall_time_ms: u64::MAX,
                inference_tokens: Some(self.envelope_tokens),
                cpu_ms: None,
                ram_bytes_s: None,
            }
        }
        fn capacity_fresh(&self) -> bool {
            self.fresh
        }
    }

    fn budgeted_payload(tokens: u64) -> serde_json::Value {
        serde_json::json!({
            "energy_budget": { "wall_time_ms": 1_000, "inference_tokens": tokens }
        })
    }

    #[tokio::test]
    async fn unbudgeted_push_admits_without_port() {
        let temp = NamedTempFile::new().unwrap();
        let queue = DomainTaskQueue::open(temp.path()).unwrap();
        let task = queue
            .push_task(
                TaskDomain::Inference,
                TaskPriority::Agent,
                "legacy",
                serde_json::json!({}),
                "t",
            )
            .await
            .unwrap();
        assert_eq!(task.status, DomainTaskStatus::Pending);
    }

    #[tokio::test]
    async fn budgeted_push_within_envelope_is_admitted() {
        let temp = NamedTempFile::new().unwrap();
        let queue =
            DomainTaskQueue::open(temp.path())
                .unwrap()
                .with_admission(std::sync::Arc::new(StubAdmission {
                    envelope_tokens: 100,
                    open: vec![],
                    fresh: true,
                }));
        let task = queue
            .push_task(
                TaskDomain::Inference,
                TaskPriority::Agent,
                "sweep",
                budgeted_payload(10),
                "t",
            )
            .await
            .unwrap();
        assert_eq!(task.status, DomainTaskStatus::Pending);
    }

    #[tokio::test]
    async fn budgeted_push_over_envelope_is_refused_and_not_persisted() {
        let temp = NamedTempFile::new().unwrap();
        let queue =
            DomainTaskQueue::open(temp.path())
                .unwrap()
                .with_admission(std::sync::Arc::new(StubAdmission {
                    envelope_tokens: 50,
                    open: vec![],
                    fresh: true,
                }));
        let err = queue
            .push_task(
                TaskDomain::Inference,
                TaskPriority::Agent,
                "greedy",
                budgeted_payload(100),
                "t",
            )
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            DomainTaskQueueError::AdmissionRequired { ref error, .. }
                if matches!(error, crate::infra::task_admission_port::TaskAdmissionError::Blocked(_))
        ));

        // Refused tasks must not touch storage.
        let all = queue.list_tasks(Some(TaskDomain::Inference), None).await;
        assert!(all.is_empty(), "refused task must not be persisted");
    }

    #[tokio::test]
    async fn stale_capacity_yields_unknown_refusal() {
        let temp = NamedTempFile::new().unwrap();
        let queue =
            DomainTaskQueue::open(temp.path())
                .unwrap()
                .with_admission(std::sync::Arc::new(StubAdmission {
                    envelope_tokens: 100,
                    open: vec![],
                    fresh: false,
                }));
        let err = queue
            .push_task(
                TaskDomain::Inference,
                TaskPriority::Agent,
                "uncertain",
                budgeted_payload(1),
                "t",
            )
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            DomainTaskQueueError::AdmissionRequired { ref error, .. }
                if matches!(error, crate::infra::task_admission_port::TaskAdmissionError::Unknown(_))
        ));
    }
}
