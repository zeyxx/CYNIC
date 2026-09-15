//! Build proof port — the single boundary between decision and execution.
//!
//! Implemented by runners (local process, SSH, remote node). The port owns no
//! policy; it only observes, compiles, and reports. CLI/MCP/agents inject the
//! adapter they need.

use async_trait::async_trait;
use cynic_build::build_proof::{BuildProofReport, BuildProofRequest};

#[async_trait]
pub trait BuildProofPort: Send + Sync {
    async fn prove(&self, request: &BuildProofRequest) -> Result<BuildProofReport, String>;

    fn node_id(&self) -> &str;
}
