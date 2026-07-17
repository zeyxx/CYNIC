use crate::domain::summarization::{SummarizationError, SummarizeRequest, SummarizeResponse};
use crate::ports::summarization_port::SummarizationPort;

pub async fn summarize_with_backend(
    backend: &dyn SummarizationPort,
    request: &SummarizeRequest,
) -> Result<SummarizeResponse, SummarizationError> {
    backend.summarize(request).await
}
