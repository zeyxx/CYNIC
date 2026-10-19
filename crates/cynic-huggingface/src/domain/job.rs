//! Pure contract for Hugging Face Jobs (`hf jobs ...`).
//!
//! Secrets are carried by NAME only (`secret_names`). The adapter resolves
//! each name from the process environment and passes `--secrets NAME` so
//! values never transit through argv, logs, or reports.

use serde::{Deserialize, Serialize};

/// How the job payload is expressed on the `hf` command line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HfJobKind {
    /// `hf jobs uv run [opts] SCRIPT [ARGS...]` — local file or URL.
    UvScript {
        script: String,
        script_args: Vec<String>,
    },
    /// `hf jobs run IMAGE COMMAND...` — raw Docker image.
    Docker { image: String, command: Vec<String> },
}

/// Validated submission spec. Pure data; the adapter renders argv.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HfJobSpec {
    pub name: Option<String>,
    pub kind: HfJobKind,
    /// Hardware flavor, e.g. `cpu-basic`, `a10g-small`. `None` = Hub default.
    pub flavor: Option<String>,
    /// Non-secret environment entries (`--env K=V`).
    pub env: Vec<(String, String)>,
    /// Secret NAMES resolved from the process environment (`--secrets NAME`).
    pub secret_names: Vec<String>,
    /// Volume mounts (`-v hf://...`).
    pub volumes: Vec<String>,
    /// Max duration (`30s`/`15m`/`2h`/`1d`). `None` = Hub default.
    pub timeout: Option<String>,
    pub labels: Vec<(String, String)>,
    pub namespace: Option<String>,
    /// `-d`: return immediately with the remote job id.
    pub detach: bool,
}

impl HfJobSpec {
    pub fn uv_script(
        script: impl Into<String>,
        script_args: Vec<String>,
    ) -> Result<Self, HfJobSpecError> {
        let script = script.into();
        if script.trim().is_empty() {
            return Err(HfJobSpecError::EmptyScript);
        }
        Ok(Self {
            name: None,
            kind: HfJobKind::UvScript {
                script,
                script_args,
            },
            flavor: None,
            env: Vec::new(),
            secret_names: Vec::new(),
            volumes: Vec::new(),
            timeout: None,
            labels: Vec::new(),
            namespace: None,
            detach: true,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HfJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    /// Stage string observed from the Hub that maps to nothing known.
    /// The raw stage is preserved on [`HfJobInfo::stage_raw`].
    Unknown,
}

/// Map a Hub-reported stage string to [`HfJobStatus`]. Pure and total:
/// unrecognized input is `Unknown`, never a fabricated healthy value.
pub fn map_stage_str(stage: &str) -> HfJobStatus {
    match stage.trim().to_ascii_lowercase().as_str() {
        "queued" | "pending" | "scheduled" | "created" => HfJobStatus::Queued,
        "running" | "started" => HfJobStatus::Running,
        "succeeded" | "success" | "completed" | "complete" | "finished" => HfJobStatus::Completed,
        "failed" | "failure" | "error" | "errored" => HfJobStatus::Failed,
        "cancelled" | "canceled" | "cancelled-by-user" => HfJobStatus::Cancelled,
        _ => HfJobStatus::Unknown,
    }
}

/// Observed job state. `raw` keeps the full Hub payload; `status` is the
/// best-effort mapping, `stage_raw` the verbatim stage string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HfJobInfo {
    pub remote_id: String,
    pub name: Option<String>,
    pub stage_raw: String,
    pub status: HfJobStatus,
    pub flavor: Option<String>,
    #[serde(default)]
    pub raw: serde_json::Value,
}

impl HfJobInfo {
    pub fn from_raw(raw: &serde_json::Value) -> Option<Self> {
        let obj = raw.as_object()?;
        let get = |keys: &[&str]| {
            keys.iter()
                .find_map(|k| obj.get(*k).and_then(|v| v.as_str()))
                .map(|s| s.to_string())
        };
        let remote_id = get(&["id", "job_id", "uuid"])?;
        let stage_raw = get(&["stage", "status", "state"]).unwrap_or_default();
        Some(Self {
            remote_id,
            name: get(&["name", "job_name"]),
            stage_raw: stage_raw.clone(),
            status: map_stage_str(&stage_raw),
            flavor: get(&["flavor", "hardware"]),
            raw: raw.clone(),
        })
    }
}

/// Scheduled-job submission (`hf jobs scheduled uv run SCHEDULE ...`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HfScheduledJobRequest {
    /// `@hourly`, `@daily`, or a CRON expression.
    pub schedule: String,
    pub spec: HfJobSpec,
}

impl HfScheduledJobRequest {
    pub fn validated(schedule: impl Into<String>, spec: HfJobSpec) -> Result<Self, HfJobSpecError> {
        let schedule = schedule.into();
        if schedule.trim().is_empty() {
            return Err(HfJobSpecError::EmptySchedule);
        }
        Ok(Self { schedule, spec })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HfJobHandle {
    pub idempotency_key: String,
    pub remote_id: String,
    pub observed_status: HfJobStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HfJobSpecError {
    #[error("uv script path required")]
    EmptyScript,
    #[error("schedule expression required")]
    EmptySchedule,
}
