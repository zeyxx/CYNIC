use super::port::StorageCensusObservationPort;
use super::process::observe_deleted_open_files;
use super::types::{
    StorageCensusInputs, StorageCensusReport, StorageDeviceObservation, assemble_storage_census,
};
use super::{compare_snapshots, load_previous_snapshot_from_env, observe_snapshot};
use crate::adapters::local_disk::LocalDiskAdapter;
use crate::adapters::local_observation::LocalHostObservationAdapter;
use crate::container_runtime::{DockerObservationPort, DockerStorageReport, LocalDockerAdapter};
use crate::domain::disk::DiskObservationPort;
use crate::hardware_conformance::{HardwareConformanceReport, assess_hardware};
use crate::ports::HostHardwareObservationPort;
use crate::process_observation::capture_command;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct LocalStorageCensusAdapter {
    pub node_id: Option<String>,
    pub docker: Option<DockerStorageReport>,
    pub model_store: Option<Value>,
    pub previous_snapshot: Option<super::StorageCensusSnapshot>,
}

impl LocalStorageCensusAdapter {
    pub fn from_env() -> Self {
        Self {
            node_id: std::env::var("CYNIC_NODE_ID").ok(),
            docker: None,
            model_store: None,
            previous_snapshot: None,
        }
    }

    pub fn with_docker(mut self, docker: DockerStorageReport) -> Self {
        self.docker = Some(docker);
        self
    }

    pub fn with_model_store(mut self, model_store: Value) -> Self {
        self.model_store = Some(model_store);
        self
    }

    pub fn with_previous_snapshot(mut self, snapshot: super::StorageCensusSnapshot) -> Self {
        self.previous_snapshot = Some(snapshot);
        self
    }
}

impl StorageCensusObservationPort for LocalStorageCensusAdapter {
    fn observe_storage_census(&self) -> Result<StorageCensusReport, String> {
        let disk = LocalDiskAdapter::new()
            .observe_disk()
            .map_err(|error| format!("disk observation failed: {error}"))?;
        let hardware: HardwareConformanceReport =
            assess_hardware(LocalHostObservationAdapter.observe_hardware());
        let (devices, device_unknown) = observe_block_devices();
        let (process_inventory, process_unknown) = match observe_deleted_open_files() {
            Ok(inventory) => (
                Some(inventory),
                Some("process observer covers deleted-open files only".into()),
            ),
            Err(error) => (None, Some(error)),
        };
        let (growth_snapshot, growth, growth_unknown) = match observe_snapshot() {
            Ok(current) => match self
                .previous_snapshot
                .clone()
                .map(Some)
                .map(Ok)
                .unwrap_or_else(load_previous_snapshot_from_env)
            {
                Ok(Some(previous)) => {
                    let growth = compare_snapshots(&previous, &current);
                    (Some(current), growth, None)
                }
                Ok(None) => (
                    Some(current),
                    Vec::new(),
                    Some("growth history requires CYNIC_STORAGE_CENSUS_PREVIOUS_SNAPSHOT".into()),
                ),
                Err(error) => (Some(current), Vec::new(), Some(error)),
            },
            Err(error) => (None, Vec::new(), Some(error)),
        };
        let mut residual_unknowns = Vec::new();
        if self.model_store.is_none() {
            residual_unknowns
                .push("model-store projection is not attached to this host adapter".into());
        }
        if let Some(unknown) = process_unknown {
            residual_unknowns.push(unknown);
        }
        if let Some(unknown) = growth_unknown {
            residual_unknowns.push(unknown);
        }
        if let Some(unknown) = device_unknown {
            residual_unknowns.push(unknown);
        }
        let docker = match &self.docker {
            Some(report) => Some(report.clone()),
            None => match LocalDockerAdapter.observe() {
                Ok(report) => Some(report),
                Err(error) => {
                    residual_unknowns.push(format!("Docker observation failed: {error}"));
                    None
                }
            },
        };
        let inputs = StorageCensusInputs {
            devices,
            disk: Some(disk),
            docker,
            hardware: Some(hardware),
            process_inventory,
            model_store: self.model_store.clone(),
            growth,
            growth_snapshot,
            residual_unknowns,
            ..StorageCensusInputs::default()
        };
        Ok(assemble_storage_census(
            self.node_id.clone().unwrap_or_else(|| "unknown".into()),
            chrono::Utc::now().timestamp_millis().max(0) as u128,
            inputs,
        ))
    }
}

fn observe_block_devices() -> (Vec<StorageDeviceObservation>, Option<String>) {
    let output = match capture_command(
        "lsblk",
        &[
            "--json",
            "--bytes",
            "--output",
            "PATH,TYPE,SIZE,FSTYPE,MOUNTPOINTS,RM",
        ],
    ) {
        Ok(output) if output.success => output,
        Ok(output) => {
            return (
                Vec::new(),
                Some(format!(
                    "lsblk observation failed (exit_code={:?}): {}",
                    output.exit_code,
                    if output.stderr.is_empty() {
                        "no stderr"
                    } else {
                        output.stderr.as_str()
                    }
                )),
            );
        }
        Err(error) => {
            return (
                Vec::new(),
                Some(format!("lsblk observation unavailable: {error}")),
            );
        }
    };
    let document: Value = match serde_json::from_str(&output.stdout) {
        Ok(document) => document,
        Err(error) => {
            return (
                Vec::new(),
                Some(format!("lsblk JSON parse failed: {error}")),
            );
        }
    };
    let mut devices = Vec::new();
    if let Some(entries) = document.get("blockdevices").and_then(Value::as_array) {
        for entry in entries {
            collect_block_device(entry, &mut devices);
        }
        (devices, None)
    } else {
        (devices, Some("lsblk returned no blockdevices array".into()))
    }
}

fn collect_block_device(entry: &Value, devices: &mut Vec<StorageDeviceObservation>) {
    let mount = entry
        .get("mountpoints")
        .and_then(Value::as_array)
        .and_then(|mounts| mounts.iter().find_map(Value::as_str))
        .map(str::to_owned);
    devices.push(StorageDeviceObservation {
        path: entry
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
        device_type: entry
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
        size_bytes: entry.get("size").and_then(Value::as_u64),
        filesystem: entry
            .get("fstype")
            .and_then(Value::as_str)
            .map(str::to_owned),
        mounted: mount.is_some(),
        mount,
        removable: entry.get("rm").and_then(Value::as_bool),
        residual_unknowns: Vec::new(),
    });
    if let Some(children) = entry.get("children").and_then(Value::as_array) {
        for child in children {
            collect_block_device(child, devices);
        }
    }
}
