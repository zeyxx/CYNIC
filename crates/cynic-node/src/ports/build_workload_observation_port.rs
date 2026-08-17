/// Read-only kernel transport for a workload delivery read-back.
#[async_trait::async_trait]
pub trait BuildWorkloadObservationPort: Send + Sync {
    async fn observe_workload(
        &self,
        workload_id: &str,
    ) -> Result<BuildWorkloadObservationReadBack, String>;
}

pub struct BuildWorkloadObservationReadBack {
    pub http_status: u16,
    pub body: serde_json::Value,
}
