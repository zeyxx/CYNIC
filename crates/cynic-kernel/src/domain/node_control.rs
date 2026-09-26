use std::sync::Arc;

use async_trait::async_trait;
use cynic_node_protocol::{NodeControlPlan, NodeDesiredState};

#[derive(Debug, thiserror::Error)]
pub enum NodeControlStoreError {
    #[error("node control record is invalid: {0}")]
    Invalid(String),
    #[error("node control store conflict: {0}")]
    Conflict(String),
    #[error("node control store failed: {0}")]
    Storage(String),
}

#[async_trait]
pub trait NodeControlStorePort: Send + Sync + std::fmt::Debug {
    async fn put_desired_state(
        &self,
        desired: NodeDesiredState,
    ) -> Result<(), NodeControlStoreError>;

    async fn desired_state(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeDesiredState>, NodeControlStoreError>;

    async fn put_status(&self, status: NodeControlPlan) -> Result<(), NodeControlStoreError>;

    async fn latest_status(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeControlPlan>, NodeControlStoreError>;
}

#[derive(Debug)]
pub struct NodeControlRuntime {
    pub store: Arc<dyn NodeControlStorePort>,
}
