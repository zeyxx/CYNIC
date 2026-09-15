//! Port + read-back type for submitting an admitted build workload to the kernel.
//!
//! The kernel is the runtime authority for workload admission. This port is the
//! thin boundary: it carries an already-admitted `AgentWorkloadDelivery` to the
//! kernel and returns the transport-level read-back. Validation of that read-back
//! lives in `cynic_protocol` (`validate_agent_workload_submission_read_back`),
//! not here.

use async_trait::async_trait;
use cynic_workload::AgentWorkloadDelivery;
use serde::{Deserialize, Serialize};

/// Transport-level read-back after a workload submission attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildWorkloadSubmissionReadBack {
    pub http_status: u16,
    pub body: String,
}

/// Boundary that delivers an admitted workload to the kernel.
#[async_trait]
pub trait BuildWorkloadSubmissionPort: Send + Sync {
    async fn submit(
        &self,
        delivery: &AgentWorkloadDelivery,
    ) -> Result<BuildWorkloadSubmissionReadBack, String>;
}
