use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use cynic_lifecycle::LocalArgvRunner;
use cynic_vcs::{
    SourceMaterializationAction, SourceMaterializationObservation, SourceMaterializationPlan,
    SourceMaterializationRequest, plan_source_materialization,
};
use serde::Serialize;

use crate::{
    adapters::{
        https_source_snapshot_acquisition::HttpsSourceSnapshotAcquisition,
        local_source_materialization::LocalSourceMaterialization,
        local_source_snapshot_acquisition::LocalSourceSnapshotAcquisition,
    },
    ports::source_materialization_port::{
        SourceMaterializationPort, SourceSnapshotAcquisitionPort,
    },
};

#[derive(Debug, Serialize)]
struct SourceMaterializationStep {
    action: SourceMaterializationAction,
    status: String,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct SourceMaterializationReport {
    schema: &'static str,
    status: String,
    request: SourceMaterializationRequest,
    before: SourceMaterializationObservation,
    initial_plan: SourceMaterializationPlan,
    applied: bool,
    steps: Vec<SourceMaterializationStep>,
    after: Option<SourceMaterializationObservation>,
    final_plan: Option<SourceMaterializationPlan>,
    evidence_path: Option<String>,
    unknowns: Vec<String>,
}

pub fn execute_source_materialization(
    request_path: &str,
    state_root: &str,
    snapshot_root: &str,
    snapshot_base_url: Option<&str>,
    expected_node_id: &str,
    apply: bool,
) -> ExitCode {
    let request = match read_request(request_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let runner = LocalArgvRunner;
    let adapter = LocalSourceMaterialization::new(
        &runner,
        state_root,
        snapshot_root,
        expected_node_id,
        "build_runner",
    );
    let remote_acquisition = snapshot_base_url
        .map(|base_url| HttpsSourceSnapshotAcquisition::new(&runner, snapshot_root, base_url));
    let local_acquisition = LocalSourceSnapshotAcquisition::new(snapshot_root);
    let acquisition: &dyn SourceSnapshotAcquisitionPort = remote_acquisition
        .as_ref()
        .map_or(&local_acquisition, |remote| remote);
    let before = match adapter.observe(&request) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "source_materialization_observe_failed:{}:{}",
                error.code, error.message
            );
            return ExitCode::FAILURE;
        }
    };
    let initial_plan = plan_source_materialization(&request, &before, now_unix_ms());
    let mut report = SourceMaterializationReport {
        schema: "cynic.node.source_materialization_report.v1",
        status: initial_plan.status.clone(),
        request,
        before,
        initial_plan,
        applied: false,
        steps: Vec::new(),
        after: None,
        final_plan: None,
        evidence_path: None,
        unknowns: Vec::new(),
    };

    if apply && matches!(report.initial_plan.status.as_str(), "planned" | "converged") {
        if report.initial_plan.status == "planned" {
            report.applied = true;
            for action in report.initial_plan.actions.clone() {
                let result = execute_action(acquisition, &adapter, &report.request, &action);
                let failed = result.is_err();
                report.steps.push(SourceMaterializationStep {
                    action,
                    status: if failed { "failed" } else { "completed" }.to_string(),
                    error: result
                        .err()
                        .map(|error| format!("{}:{}", error.code, error.message)),
                });
                if failed {
                    report.status = "blocked".to_string();
                    break;
                }
            }
        }
        match adapter.observe(&report.request) {
            Ok(after) => {
                let final_plan =
                    plan_source_materialization(&report.request, &after, now_unix_ms());
                if !report.steps.iter().any(|step| step.status == "failed") {
                    report.status = final_plan.status.clone();
                }
                report.after = Some(after);
                report.final_plan = Some(final_plan);
            }
            Err(error) => {
                report.status = "unknown".to_string();
                report.unknowns.push(format!(
                    "source_materialization_read_back_failed:{}:{}",
                    error.code, error.message
                ));
            }
        }
        match persist_evidence(Path::new(state_root), &report) {
            Ok(path) => report.evidence_path = Some(path.display().to_string()),
            Err(error) => {
                report.status = "unknown".to_string();
                report.unknowns.push(error);
            }
        }
    }

    let successful = matches!(report.status.as_str(), "planned" | "converged");
    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("source materialization report serialization failed: {error}");
            return ExitCode::FAILURE;
        }
    }
    if successful {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn execute_action<R: cynic_lifecycle::ArgvRunner>(
    acquisition: &dyn SourceSnapshotAcquisitionPort,
    adapter: &LocalSourceMaterialization<'_, R>,
    request: &SourceMaterializationRequest,
    action: &SourceMaterializationAction,
) -> Result<(), crate::ports::source_materialization_port::SourceMaterializationPortError> {
    match action {
        SourceMaterializationAction::AcquireSnapshot {
            snapshot_digest,
            snapshot_format,
        } => acquisition.acquire(snapshot_digest, snapshot_format),
        SourceMaterializationAction::MaterializeCheckout { .. } => adapter.materialize(request),
        SourceMaterializationAction::ReadBack {
            snapshot_digest,
            source_commit,
            ..
        } => {
            let observed = adapter.observe(request)?;
            if observed.observed_commit.as_deref() == Some(source_commit.as_str())
                && observed.observed_snapshot_digest.as_deref() == Some(snapshot_digest.as_str())
            {
                Ok(())
            } else {
                Err(
                    crate::ports::source_materialization_port::SourceMaterializationPortError::new(
                        "source_materialization_read_back_mismatch",
                        "checkout commit or provenance digest differs",
                    ),
                )
            }
        }
    }
}

fn read_request(path: &str) -> Result<SourceMaterializationRequest, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("source_materialization_request_read_failed:{error}"))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("source_materialization_request_decode_failed:{error}"))
}

fn persist_evidence(
    state_root: &Path,
    report: &SourceMaterializationReport,
) -> Result<PathBuf, String> {
    ensure_directory(state_root, false)?;
    let lifecycle_root = state_root.join("source-materialization");
    ensure_directory(&lifecycle_root, true)?;
    let evidence_root = lifecycle_root.join("evidence");
    ensure_directory(&evidence_root, true)?;
    let release_root = evidence_root.join(&report.request.release_id);
    ensure_directory(&release_root, true)?;

    let path = release_root.join(format!("{}.json", now_unix_ms()));
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("source_materialization_evidence_encode_failed:{error}"))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|error| format!("source_materialization_evidence_open_failed:{error}"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("source_materialization_evidence_write_failed:{error}"))?;
    File::open(&release_root)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("source_materialization_evidence_sync_failed:{error}"))?;
    Ok(path)
}

fn ensure_directory(path: &Path, create: bool) -> Result<(), String> {
    if create && !path.exists() {
        fs::create_dir(path).map_err(|error| {
            format!("source_materialization_evidence_dir_create_failed:{error}")
        })?;
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("source_materialization_evidence_dir_unobservable:{error}"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("source_materialization_evidence_dir_unsafe".to_string());
    }
    Ok(())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}
