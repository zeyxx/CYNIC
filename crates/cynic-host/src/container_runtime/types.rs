use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockerRuntimeKind {
    Docker,
    Containerd,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockerDaemonObservation {
    pub reachable: bool,
    pub runtime: DockerRuntimeKind,
    pub server_version: Option<String>,
    pub root_dir: Option<String>,
    pub storage_driver: Option<String>,
    pub snapshotter: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockerImageObservation {
    pub id: String,
    pub repository: Option<String>,
    pub tag: Option<String>,
    pub logical_size_bytes: Option<u64>,
    pub unique_size_bytes: Option<u64>,
    pub container_refs: u32,
    pub reconstructible: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockerContainerObservation {
    pub id: String,
    pub name: String,
    pub image_id: Option<String>,
    pub state: String,
    pub project: Option<String>,
    pub writable_layer_bytes: Option<u64>,
    pub log_bytes: Option<u64>,
    pub volume_refs: Vec<String>,
    pub created_at_unix_ms: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockerVolumeObservation {
    pub name: String,
    pub driver: Option<String>,
    pub size_bytes: Option<u64>,
    pub active_links: u32,
    pub project: Option<String>,
    pub persistent: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockerStorageReport {
    pub schema: String,
    pub observed_at_unix_ms: u128,
    pub daemon: DockerDaemonObservation,
    pub images: Vec<DockerImageObservation>,
    pub containers: Vec<DockerContainerObservation>,
    pub volumes: Vec<DockerVolumeObservation>,
    pub image_bytes: Option<u64>,
    pub container_bytes: Option<u64>,
    pub volume_bytes: Option<u64>,
    pub reclaimable_bytes: Option<u64>,
    pub residual_unknowns: Vec<String>,
}
