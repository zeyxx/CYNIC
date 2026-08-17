use crate::container_runtime::DockerStorageReport;
use crate::domain::disk::DiskReport;
use crate::hardware_conformance::HardwareConformanceReport;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CensusKnowledge {
    Observed,
    Partial,
    Unknown,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageDeviceObservation {
    pub path: String,
    pub device_type: String,
    pub size_bytes: Option<u64>,
    pub filesystem: Option<String>,
    pub mount: Option<String>,
    pub mounted: bool,
    pub removable: Option<bool>,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageGrowthObservation {
    pub scope: String,
    pub previous_bytes: Option<u64>,
    pub current_bytes: Option<u64>,
    pub delta_bytes: Option<i64>,
    pub observed_at_unix_ms: u128,
    pub sample_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageFilesystemSnapshot {
    pub scope: String,
    pub source: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageCensusSnapshot {
    pub schema: String,
    pub observed_at_unix_ms: u128,
    pub filesystems: Vec<StorageFilesystemSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageCensusCoverage {
    pub hardware: CensusKnowledge,
    pub devices: CensusKnowledge,
    pub filesystems: CensusKnowledge,
    pub files: CensusKnowledge,
    pub processes: CensusKnowledge,
    pub containers: CensusKnowledge,
    pub artifacts: CensusKnowledge,
    pub growth: CensusKnowledge,
}

#[derive(Debug, Clone, Serialize)]
pub struct StorageCensusReport {
    pub schema: String,
    pub node_id: String,
    pub observed_at_unix_ms: u128,
    pub coverage: StorageCensusCoverage,
    pub devices: Vec<StorageDeviceObservation>,
    pub disk: Option<DiskReport>,
    pub docker: Option<DockerStorageReport>,
    pub hardware: Option<HardwareConformanceReport>,
    pub model_store: Option<serde_json::Value>,
    pub process_inventory: Option<serde_json::Value>,
    pub growth: Vec<StorageGrowthObservation>,
    pub growth_snapshot: Option<StorageCensusSnapshot>,
    pub residual_unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageCensusState {
    Healthy,
    Degraded,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageCensusAction {
    Continue,
    ThrottleStorageHeavyWorkloads,
    PauseNonEssentialIngestion,
    PreserveEvidence,
    RequestHumanIntervention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageCensusDecision {
    pub schema: String,
    pub node_id: String,
    pub state: StorageCensusState,
    pub actions: Vec<StorageCensusAction>,
    pub reclaimable_bytes: Option<u64>,
    pub residual_unknowns: Vec<String>,
    pub mutation_authorized: bool,
}

#[derive(Debug, Default)]
pub struct StorageCensusInputs {
    pub devices: Vec<StorageDeviceObservation>,
    pub disk: Option<DiskReport>,
    pub docker: Option<DockerStorageReport>,
    pub hardware: Option<HardwareConformanceReport>,
    pub model_store: Option<serde_json::Value>,
    pub process_inventory: Option<serde_json::Value>,
    pub growth: Vec<StorageGrowthObservation>,
    pub growth_snapshot: Option<StorageCensusSnapshot>,
    pub residual_unknowns: Vec<String>,
}

pub fn assemble_storage_census(
    node_id: impl Into<String>,
    observed_at_unix_ms: u128,
    inputs: StorageCensusInputs,
) -> StorageCensusReport {
    let coverage = StorageCensusCoverage {
        hardware: knowledge(inputs.hardware.is_some()),
        devices: knowledge(!inputs.devices.is_empty()),
        filesystems: knowledge(
            inputs
                .disk
                .as_ref()
                .is_some_and(|disk| !disk.filesystems.is_empty()),
        ),
        files: inputs
            .disk
            .as_ref()
            .map(|disk| {
                if disk.large_files.is_empty() {
                    CensusKnowledge::Partial
                } else {
                    CensusKnowledge::Observed
                }
            })
            .unwrap_or(CensusKnowledge::Unknown),
        processes: inputs
            .process_inventory
            .as_ref()
            .map(|_| CensusKnowledge::Partial)
            .unwrap_or(CensusKnowledge::Unknown),
        containers: inputs
            .docker
            .as_ref()
            .map(|_| CensusKnowledge::Partial)
            .unwrap_or(CensusKnowledge::Unknown),
        artifacts: knowledge(inputs.model_store.is_some()),
        growth: knowledge(!inputs.growth.is_empty()),
    };
    StorageCensusReport {
        schema: "cynic.host.storage_census.v1".into(),
        node_id: node_id.into(),
        observed_at_unix_ms,
        coverage,
        devices: inputs.devices,
        disk: inputs.disk,
        docker: inputs.docker,
        hardware: inputs.hardware,
        model_store: inputs.model_store,
        process_inventory: inputs.process_inventory,
        growth: inputs.growth,
        growth_snapshot: inputs.growth_snapshot,
        residual_unknowns: inputs.residual_unknowns,
    }
}

fn knowledge(observed: bool) -> CensusKnowledge {
    if observed {
        CensusKnowledge::Observed
    } else {
        CensusKnowledge::Unknown
    }
}
