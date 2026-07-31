use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::adapters::local_storage_lifecycle::LocalStorageLifecycleAdapter;
use crate::domain::storage_lifecycle::{
    RestartBudget, STORAGE_RECONCILE_SCHEMA, StorageDecision, StorageDesiredState,
    StorageObservation, reconcile_storage,
};
use crate::ports::storage_lifecycle_port::StorageLifecyclePort;

#[derive(Debug, Serialize)]
struct StorageReconcileReport {
    schema: &'static str,
    status: &'static str,
    desired: StorageDesiredState,
    observation: StorageObservation,
    budget_before: RestartBudget,
    decision: StorageDecision,
    applied: bool,
    apply_error: Option<String>,
    read_back: Option<StorageObservation>,
    evidence_path: Option<String>,
    unknowns: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedBudget {
    schema: String,
    window_started_unix_ms: u64,
    restart_count: u32,
    last_restart_unix_ms: Option<u64>,
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_storage_reconcile(
    state_dir: &str,
    warmup_seconds: u64,
    restart_window_seconds: u64,
    max_restarts_per_window: u32,
    apply: bool,
) -> ExitCode {
    let service = "surrealdb.service";
    let health_url = "http://127.0.0.1:8000/health";
    let desired = StorageDesiredState {
        service: service.to_string(),
        health_url: health_url.to_string(),
        desired_running: true,
        warmup_seconds,
        restart_window_seconds,
        max_restarts_per_window,
    };
    let now = now_unix_ms();
    let state_root = PathBuf::from(state_dir);
    let budget_path = state_root.join("restart-budget.json");
    let (budget_before, budget_unknown) = load_budget(
        &budget_path,
        now,
        restart_window_seconds,
        max_restarts_per_window,
    );
    let adapter = LocalStorageLifecycleAdapter::default();
    let observation = adapter.observe(service, health_url).await;
    let decision = reconcile_storage(&desired, &observation, &budget_before, now);

    let mut report = StorageReconcileReport {
        schema: STORAGE_RECONCILE_SCHEMA,
        status: status_for_decision(&decision),
        desired,
        observation,
        budget_before,
        decision: decision.clone(),
        applied: false,
        apply_error: None,
        read_back: None,
        evidence_path: None,
        unknowns: budget_unknown.into_iter().collect(),
    };

    if apply {
        if let StorageDecision::Restart { .. } = decision {
            let next_budget = next_budget(&report.budget_before, now, restart_window_seconds);
            if let Err(error) = persist_budget(&budget_path, &next_budget) {
                report.status = "unknown";
                report.apply_error = Some(error);
                report
                    .unknowns
                    .push("restart_not_attempted_budget_persistence_failed".to_string());
            } else {
                match adapter.restart(service) {
                    Ok(()) => {
                        report.applied = true;
                        tokio::time::sleep(std::time::Duration::from_secs(warmup_seconds)).await;
                        let read_back = adapter.observe(service, health_url).await;
                        report.status = if read_back.service_active && read_back.health_ok {
                            "converged"
                        } else {
                            "blocked"
                        };
                        report.read_back = Some(read_back);
                    }
                    Err(error) => {
                        report.status = "blocked";
                        report.apply_error = Some(error);
                    }
                }
            }
        }
    }

    if apply {
        match persist_evidence(&state_root, now, &report) {
            Ok(path) => report.evidence_path = Some(path.display().to_string()),
            Err(error) => {
                report.status = "unknown";
                report.unknowns.push(error);
            }
        }
    }

    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("storage reconcile report serialization failed: {error}");
            return ExitCode::FAILURE;
        }
    }

    if matches!(report.status, "healthy" | "converged" | "planned") {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn status_for_decision(decision: &StorageDecision) -> &'static str {
    match decision {
        StorageDecision::Healthy => "healthy",
        StorageDecision::ObserveOnly { .. } => "observed",
        StorageDecision::Restart { .. } => "planned",
        StorageDecision::Blocked { .. } => "blocked",
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

fn load_budget(
    path: &Path,
    now: u64,
    window_seconds: u64,
    max_restarts_per_window: u32,
) -> (RestartBudget, Option<String>) {
    let default = RestartBudget {
        window_started_unix_ms: now,
        restart_count: 0,
    };
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return (default, None),
        Err(error) => {
            return (
                RestartBudget {
                    window_started_unix_ms: now,
                    restart_count: max_restarts_per_window,
                },
                Some(format!("restart_budget_read_failed:{error}")),
            );
        }
    };
    let Ok(stored) = serde_json::from_slice::<PersistedBudget>(&bytes) else {
        return (
            RestartBudget {
                window_started_unix_ms: now,
                restart_count: max_restarts_per_window,
            },
            Some("restart_budget_invalid".to_string()),
        );
    };
    if stored.schema != "cynic.storage_restart_budget.v1"
        || now.saturating_sub(stored.window_started_unix_ms) >= window_seconds.saturating_mul(1_000)
    {
        return (default, None);
    }
    (
        RestartBudget {
            window_started_unix_ms: stored.window_started_unix_ms,
            restart_count: stored.restart_count,
        },
        None,
    )
}

fn next_budget(current: &RestartBudget, now: u64, window_seconds: u64) -> PersistedBudget {
    let expired =
        now.saturating_sub(current.window_started_unix_ms) >= window_seconds.saturating_mul(1_000);
    PersistedBudget {
        schema: "cynic.storage_restart_budget.v1".to_string(),
        window_started_unix_ms: if expired {
            now
        } else {
            current.window_started_unix_ms
        },
        restart_count: if expired {
            1
        } else {
            current.restart_count.saturating_add(1)
        },
        last_restart_unix_ms: Some(now),
    }
}

fn persist_budget(path: &Path, budget: &PersistedBudget) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "budget_parent_missing".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("budget_dir_create_failed:{error}"))?;
    let bytes = serde_json::to_vec_pretty(budget)
        .map_err(|error| format!("budget_serialize_failed:{error}"))?;
    let temporary = parent.join("restart-budget.json.tmp");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| format!("budget_open_failed:{error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("budget_write_failed:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("budget_sync_failed:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("budget_publish_failed:{error}"))?;
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("budget_parent_sync_failed:{error}"))
}

fn persist_evidence(
    state_root: &Path,
    now: u64,
    report: &StorageReconcileReport,
) -> Result<PathBuf, String> {
    let evidence_dir = state_root.join("evidence");
    fs::create_dir_all(&evidence_dir)
        .map_err(|error| format!("evidence_dir_create_failed:{error}"))?;
    let path = evidence_dir.join(format!("{now}.json"));
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("evidence_serialize_failed:{error}"))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|error| format!("evidence_open_failed:{error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("evidence_write_failed:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("evidence_sync_failed:{error}"))?;
    fs::File::open(&evidence_dir)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("evidence_parent_sync_failed:{error}"))?;
    Ok(path)
}
