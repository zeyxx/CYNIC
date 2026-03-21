//! SummarizationPort — domain contract for text summarization.
//! Used by session summarization pipeline. Adapter: SovereignSummarizer.

use async_trait::async_trait;

#[derive(Debug)]
pub enum SummarizationError {
    Unreachable(String),
    Timeout,
    Protocol(String),
}

impl std::fmt::Display for SummarizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(msg) => write!(f, "Summarization unreachable: {}", msg),
            Self::Timeout => write!(f, "Summarization timed out"),
            Self::Protocol(msg) => write!(f, "Summarization protocol error: {}", msg),
        }
    }
}

impl std::error::Error for SummarizationError {}

#[async_trait]
pub trait SummarizationPort: Send + Sync {
    /// Summarize text using an LLM. Returns the summary string.
    async fn summarize(&self, prompt: &str) -> Result<String, SummarizationError>;
}

/// Null implementation for graceful degradation and testing.
pub struct NullSummarizer;

#[async_trait]
impl SummarizationPort for NullSummarizer {
    async fn summarize(&self, _prompt: &str) -> Result<String, SummarizationError> {
        Err(SummarizationError::Unreachable("NullSummarizer — no LLM available".into()))
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
