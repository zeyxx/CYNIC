use crate::domain::inference::{BackendError, InferRequest, InferResponse};
use crate::ports::inference_port::InferPort;

pub async fn infer_with_backend(
    backend: &dyn InferPort,
    request: &InferRequest,
    request_id: Option<&str>,
) -> Result<InferResponse, BackendError> {
    backend.infer(request, request_id).await
}
