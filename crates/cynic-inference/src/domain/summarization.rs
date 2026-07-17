//! Summarization domain values and errors.

#[derive(Debug, Clone)]
pub struct SummarizeRequest {
    pub prompt: String,
    pub max_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct SummarizeResponse {
    pub text: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum SummarizationError {
    #[error("Summarization unavailable: {0}")]
    Unavailable(String),
    #[error("Summarization failed: {0}")]
    Failed(String),
}
