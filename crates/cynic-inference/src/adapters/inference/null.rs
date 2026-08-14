use async_trait::async_trait;

use crate::domain::inference::{BackendError, InferRequest, InferResponse};
use crate::ports::inference_port::InferPort;

#[derive(Debug)]
pub struct NullInfer;

#[async_trait]
impl InferPort for NullInfer {
    async fn infer(
        &self,
        _request: &InferRequest,
        _request_id: Option<&str>,
    ) -> Result<InferResponse, BackendError> {
        Err(BackendError::Unreachable(
            "No sovereign LLM available".into(),
        ))
    }
}
