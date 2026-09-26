use std::sync::Arc;

use async_trait::async_trait;
use cynic_node_protocol::{MachineEnrollmentAuthority, SignedNodeEnrollmentGrant};

#[derive(Debug, thiserror::Error)]
pub enum NodeEnrollmentStoreError {
    #[error("node enrollment conflict: {0}")]
    Conflict(String),
    #[error("node enrollment storage failed: {0}")]
    Storage(String),
}

#[async_trait]
pub trait NodeEnrollmentStorePort: Send + Sync + std::fmt::Debug {
    async fn consume_token_and_store_grant(
        &self,
        token_digest: &str,
        grant: SignedNodeEnrollmentGrant,
    ) -> Result<(), NodeEnrollmentStoreError>;

    async fn grant(
        &self,
        node_id: &str,
    ) -> Result<Option<SignedNodeEnrollmentGrant>, NodeEnrollmentStoreError>;

    async fn replace_grant(
        &self,
        expected_grant_id: &str,
        expected_generation: u64,
        grant: SignedNodeEnrollmentGrant,
    ) -> Result<(), NodeEnrollmentStoreError>;
}

pub trait NodeEnrollmentSignerPort: Send + Sync + std::fmt::Debug {
    fn key_id(&self) -> String;
    fn sign(&self, payload: &[u8]) -> Result<String, String>;
    fn verify(&self, payload: &[u8], signature_hex: &str) -> Result<(), String>;
}

#[derive(Debug)]
pub struct NodeEnrollmentRuntime {
    pub authority: MachineEnrollmentAuthority,
    pub store: Arc<dyn NodeEnrollmentStorePort>,
    pub signer: Arc<dyn NodeEnrollmentSignerPort>,
}
