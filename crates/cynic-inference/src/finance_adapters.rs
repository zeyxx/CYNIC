//! Finance adapters for cynic-inference.
//!
//! Implements cynic-finance ContextMeter port using chat session metrics.

use cynic_finance::ports::{ContextMeter, MeterError};
use std::fs;
use std::path::Path;

/// Context meter — measures LLM context token consumption from chat session logs.
pub struct InferenceContextMeter {
    session_dir: String,
}

impl InferenceContextMeter {
    pub fn new(session_dir: impl Into<String>) -> Self {
        Self {
            session_dir: session_dir.into(),
        }
    }
}

impl ContextMeter for InferenceContextMeter {
    fn measure_tokens(&self, agent_id: &str) -> Result<u64, MeterError> {
        let session_path = Path::new(&self.session_dir).join(format!("{}.jsonl", agent_id));
        if !session_path.exists() {
            return Err(MeterError::Unknown(format!(
                "session file '{}' does not exist",
                session_path.display()
            )));
        }

        let content = fs::read_to_string(&session_path)
            .map_err(|e| MeterError::Unavailable(format!("read {}: {}", session_path.display(), e)))?;

        let mut total_tokens = 0u64;
        for line in content.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(tokens) = json.get("tokens").and_then(|t| t.as_u64()) {
                    total_tokens += tokens;
                }
            }
        }

        Ok(total_tokens)
    }
}
