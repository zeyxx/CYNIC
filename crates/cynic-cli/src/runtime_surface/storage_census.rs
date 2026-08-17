//! Thin CLI projection for the host-owned storage census.

use std::path::Path;
use std::process::ExitCode;

use cynic_host::storage_census::{
    LocalStorageCensusAdapter, StorageCensusObservationPort, decide_storage_census,
};
use cynic_inference::ModelStoreService;

use crate::fs_json::write_pretty_json_file;

pub(crate) fn execute_storage_census(
    json: bool,
    previous_snapshot: Option<&str>,
    snapshot_output: Option<&str>,
) -> ExitCode {
    let adapter = LocalStorageCensusAdapter::from_env();
    let adapter = match ModelStoreService::from_env().scan() {
        Ok(report) => match serde_json::to_value(report) {
            Ok(report) => adapter.with_model_store(report),
            Err(_) => adapter,
        },
        Err(_) => adapter,
    };
    let adapter = match previous_snapshot {
        Some(path) => match cynic_host::storage_census::read_previous_snapshot(Path::new(path)) {
            Ok(snapshot) => adapter.with_previous_snapshot(snapshot),
            Err(error) => return emit_error(json, snapshot_output.is_some(), error),
        },
        None => adapter,
    };
    let result =
        StorageCensusObservationPort::observe_storage_census(&adapter).and_then(|report| {
            if let Some(path) = snapshot_output {
                let snapshot = report.growth_snapshot.as_ref().ok_or_else(|| {
                    "storage census did not produce a filesystem snapshot".to_string()
                })?;
                write_pretty_json_file(Path::new(path), snapshot, "storage census snapshot")?;
            }
            let decision = decide_storage_census(&report);
            Ok(serde_json::json!({
                "schema": "cynic.cli.storage_census.v1",
                "mutates": snapshot_output.is_some(),
                "snapshot_output": snapshot_output,
                "report": report,
                "decision": decision,
            }))
        });
    match result {
        Ok(result) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            } else {
                println!(
                    "storage census: state={:?} actions={:?}",
                    result["decision"]["state"], result["decision"]["actions"]
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) => emit_error(json, snapshot_output.is_some(), error),
    }
}

fn emit_error(json: bool, mutates: bool, error: String) -> ExitCode {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "schema": "cynic.cli.storage_census.v1",
                "mutates": mutates,
                "status": "error",
                "error": error,
            })
        );
    } else {
        eprintln!("Storage census failed: {error}");
    }
    ExitCode::FAILURE
}
