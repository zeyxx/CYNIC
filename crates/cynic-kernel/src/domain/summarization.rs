//! SummarizationPort — domain contract for text summarization.
//! Used by session summarization pipeline. Adapter: SovereignSummarizer.

use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum SummarizationError {
    #[error("Summarization unreachable: {0}")]
    Unreachable(String),
    #[error("Summarization timed out")]
    Timeout,
    #[error("Summarization protocol error: {0}")]
    Protocol(String),
}

#[async_trait]
pub trait SummarizationPort: Send + Sync {
    /// Summarize text using an LLM. Returns the summary string.
    async fn summarize(&self, prompt: &str) -> Result<String, SummarizationError>;
}

/// Null implementation for graceful degradation and testing.
#[derive(Debug)]
pub struct NullSummarizer;

#[async_trait]
impl SummarizationPort for NullSummarizer {
    async fn summarize(&self, _prompt: &str) -> Result<String, SummarizationError> {
        Err(SummarizationError::Unreachable(
            "NullSummarizer — no LLM available".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn null_summarizer_returns_error() {
        let s = NullSummarizer;
        assert!(s.summarize("test").await.is_err());
    }
}
