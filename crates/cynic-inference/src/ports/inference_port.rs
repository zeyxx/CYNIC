use async_trait::async_trait;

use crate::domain::inference::{BackendError, BackendStatus, InferRequest, InferResponse};

#[async_trait]
pub trait BackendPort: Send + Sync {
    fn name(&self) -> &str;
    async fn health(&self) -> BackendStatus;
}

#[async_trait]
pub trait InferPort: Send + Sync {
    async fn infer(
        &self,
        request: &InferRequest,
        request_id: Option<&str>,
    ) -> Result<InferResponse, BackendError>;
}
