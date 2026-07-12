use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cynic_node::adapters::registry::LocalRegistryAdapter;
use cynic_node::domain::fleet_truth::build_report as build_fleet_truth_report;

use crate::fleet_observation::{ObservationInputs, load_observed_resources};

pub(crate) fn emit_fleet_report(
    registry_path: &Path,
    proxmox_json: Option<&str>,
    proxmox_list_text: Option<&str>,
    watchdog_json: Option<&str>,
) -> Result<(serde_json::Value, bool), String> {
    let registry_path = registry_path.to_path_buf();
    let adapter = LocalRegistryAdapter::new(&registry_path);
    let declared_nodes = adapter
        .declared_nodes()
        .map_err(|error| error.to_string())?;
    let registry_organs = adapter
        .registry_organs()
        .map_err(|error| error.to_string())?;
    let mut observed_resources = adapter
        .observed_provider_resources()
        .map_err(|error| error.to_string())?;
    let project_root = project_root_from_registry(&registry_path).ok_or_else(|| {
        format!(
            "cannot derive project root from registry path {}",
            registry_path.display()
        )
    })?;
    let organ_dirs = discover_organ_dirs(&project_root)?;

    observed_resources.append(&mut load_observed_resources(ObservationInputs {
        proxmox_json,
        proxmox_list_text,
        watchdog_json,
    })?);

    let report = build_fleet_truth_report(
        &declared_nodes,
        &observed_resources,
        &registry_organs,
        &organ_dirs,
    );
    let payload = serde_json::to_value(&report)
        .map_err(|error| format!("failed to serialize fleet report: {error}"))?;
    Ok((payload, report.summary.status != "ok"))
}

fn project_root_from_registry(registry_path: &Path) -> Option<PathBuf> {
    let registry_path = registry_path.canonicalize().ok()?;
    let infra_dir = registry_path.parent()?;
    if infra_dir.file_name().and_then(|name| name.to_str()) != Some("infra") {
        return None;
    }
    infra_dir.parent().map(Path::to_path_buf)
}

fn discover_organ_dirs(project_root: &Path) -> Result<BTreeSet<String>, String> {
    let organs = project_root.join("organs");
    let Ok(entries) = std::fs::read_dir(&organs) else {
        return Ok(BTreeSet::new());
    };
    let mut values = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read organ entry: {error}"))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !entry.path().is_dir()
            || name.starts_with('.')
            || matches!(name.as_str(), "controllers" | "state")
        {
            continue;
        }
        values.insert(name);
    }
    Ok(values)
}

pub(crate) fn emit_result(result: Result<(serde_json::Value, bool), String>) -> ExitCode {
    match result {
        Ok((payload, should_fail)) => match serde_json::to_string_pretty(&payload) {
            Ok(payload) => {
                println!("{payload}");
                if should_fail {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(error) => super::fail(error.to_string()),
        },
        Err(error) => super::fail(error),
    }
}
