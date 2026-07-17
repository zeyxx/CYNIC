use async_trait::async_trait;

use crate::domain::summarization::{SummarizationError, SummarizeRequest, SummarizeResponse};

#[async_trait]
pub trait SummarizationPort: Send + Sync {
    async fn summarize(
        &self,
        request: &SummarizeRequest,
    ) -> Result<SummarizeResponse, SummarizationError>;
}
