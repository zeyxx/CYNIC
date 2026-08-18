use super::types::DockerStorageReport;

pub trait DockerObservationPort: Send + Sync {
    fn observe(&self) -> Result<DockerStorageReport, String>;
}
