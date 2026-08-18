//! Reconnectable wrappers for NodeControl and NodeEnrollment storage ports.

use async_trait::async_trait;
use std::sync::{Arc, RwLock};

use crate::domain::node_control::{NodeControlStoreError, NodeControlStorePort};
use crate::domain::node_enrollment::{NodeEnrollmentStoreError, NodeEnrollmentStorePort};
use cynic_protocol::{NodeControlPlan, NodeDesiredState, SignedNodeEnrollmentGrant};

#[derive(Debug)]
pub struct ReconnectableNodeControl {
    inner: Arc<RwLock<Option<Arc<dyn NodeControlStorePort>>>>,
}

impl ReconnectableNodeControl {
    pub fn new(inner: Option<Arc<dyn NodeControlStorePort>>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }
    pub fn shared_lock(&self) -> Arc<RwLock<Option<Arc<dyn NodeControlStorePort>>>> {
        Arc::clone(&self.inner)
    }
    fn current(&self) -> Result<Arc<dyn NodeControlStorePort>, NodeControlStoreError> {
        self.inner
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
            .ok_or_else(|| {
                NodeControlStoreError::Storage("storage_primary is unavailable".to_string())
            })
    }
}

#[async_trait]
impl NodeControlStorePort for ReconnectableNodeControl {
    async fn put_desired_state(
        &self,
        desired: NodeDesiredState,
    ) -> Result<(), NodeControlStoreError> {
        self.current()?.put_desired_state(desired).await
    }
    async fn desired_state(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeDesiredState>, NodeControlStoreError> {
        self.current()?.desired_state(node_id).await
    }
    async fn put_status(&self, status: NodeControlPlan) -> Result<(), NodeControlStoreError> {
        self.current()?.put_status(status).await
    }
    async fn latest_status(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeControlPlan>, NodeControlStoreError> {
        self.current()?.latest_status(node_id).await
    }
}

#[derive(Debug)]
pub struct ReconnectableNodeEnrollment {
    inner: Arc<RwLock<Option<Arc<dyn NodeEnrollmentStorePort>>>>,
}

impl ReconnectableNodeEnrollment {
    pub fn new(inner: Option<Arc<dyn NodeEnrollmentStorePort>>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }
    pub fn shared_lock(&self) -> Arc<RwLock<Option<Arc<dyn NodeEnrollmentStorePort>>>> {
        Arc::clone(&self.inner)
    }
    fn current(&self) -> Result<Arc<dyn NodeEnrollmentStorePort>, NodeEnrollmentStoreError> {
        self.inner
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
            .ok_or_else(|| {
                NodeEnrollmentStoreError::Storage("storage_primary is unavailable".to_string())
            })
    }
}

#[async_trait]
impl NodeEnrollmentStorePort for ReconnectableNodeEnrollment {
    async fn consume_token_and_store_grant(
        &self,
        token_digest: &str,
        grant: SignedNodeEnrollmentGrant,
    ) -> Result<(), NodeEnrollmentStoreError> {
        self.current()?
            .consume_token_and_store_grant(token_digest, grant)
            .await
    }
    async fn grant(
        &self,
        node_id: &str,
    ) -> Result<Option<SignedNodeEnrollmentGrant>, NodeEnrollmentStoreError> {
        self.current()?.grant(node_id).await
    }
    async fn replace_grant(
        &self,
        expected_grant_id: &str,
        expected_generation: u64,
        grant: SignedNodeEnrollmentGrant,
    ) -> Result<(), NodeEnrollmentStoreError> {
        self.current()?
            .replace_grant(expected_grant_id, expected_generation, grant)
            .await
    }
}
