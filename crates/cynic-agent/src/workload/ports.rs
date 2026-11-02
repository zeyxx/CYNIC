use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use super::delivery::AgentWorkloadDelivery;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildWorkloadSubmissionReadBack {
    pub http_status: u16,
    pub body: String,
}

#[async_trait]
pub trait BuildWorkloadSubmissionPort: Send + Sync {
    async fn submit(
        &self,
        delivery: &AgentWorkloadDelivery,
    ) -> Result<BuildWorkloadSubmissionReadBack, String>;
}

#[async_trait]
pub trait BuildWorkloadObservationPort: Send + Sync {
    async fn observe_workload(
        &self,
        workload_id: &str,
    ) -> Result<BuildWorkloadObservationReadBack, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildWorkloadObservationReadBack {
    pub http_status: u16,
    pub body: serde_json::Value,
}
