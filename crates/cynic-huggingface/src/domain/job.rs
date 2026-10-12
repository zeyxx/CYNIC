use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HfJobSpec {
    pub command: Vec<String>,
    pub hardware_target: Option<String>,
    pub repo: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HfJobHandle {
    pub idempotency_key: String,
    pub remote_id: String,
    pub observed_status: HfJobStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HfJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}
