//! Finance adapters for cynic-inference.
//!
//! Implements cynic-finance ContextMeter port using ChatTrace JSONL.

use cynic_finance::ports::{ContextMeter, MeterError};
use std::fs;
use std::path::Path;

/// Context meter — measures LLM context token consumption from chat trace logs.
///
/// Reads `evidence/inference_traces.jsonl` and sums `prompt_tokens + gen_tokens`
/// for all traces matching the given agent/session.
#[derive(Debug)]
pub struct InferenceContextMeter {
    trace_dir: String,
}

impl InferenceContextMeter {
    pub fn new(trace_dir: impl Into<String>) -> Self {
        Self {
            trace_dir: trace_dir.into(),
        }
    }

    /// Parses a ChatTrace JSONL line and returns (session_id, prompt_tokens, gen_tokens).
    fn parse_trace_line(line: &str) -> Option<(String, u64, u64)> {
        let json: serde_json::Value = serde_json::from_str(line).ok()?;
        let session_id = json.get("session_id")?.as_str()?.to_string();
        let prompt_tokens = json
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let gen_tokens = json.get("gen_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        Some((session_id, prompt_tokens, gen_tokens))
    }
}

impl ContextMeter for InferenceContextMeter {
    fn measure_tokens(&self, agent_id: &str) -> Result<u64, MeterError> {
        let trace_path = Path::new(&self.trace_dir).join("inference_traces.jsonl");
        if !trace_path.exists() {
            return Err(MeterError::Unknown(format!(
                "trace file '{}' does not exist",
                trace_path.display()
            )));
        }

        let content = fs::read_to_string(&trace_path)
            .map_err(|e| MeterError::Unavailable(format!("read {}: {}", trace_path.display(), e)))?;

        let mut total_tokens = 0u64;
        let mut found = false;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Some((session_id, prompt_tokens, gen_tokens)) = Self::parse_trace_line(line) {
                if session_id == agent_id {
                    total_tokens += prompt_tokens + gen_tokens;
                    found = true;
                }
            }
        }

        if !found {
            return Err(MeterError::Unknown(format!(
                "no traces found for agent '{}'",
                agent_id
            )));
        }

        Ok(total_tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_trace_line_valid() {
        let line = r#"{"schema":"cynic.trace.chat_turn.v1","ts_unix_ms":1000,"session_id":"s1","status":"ok","tool_turns":0,"prompt_chars":10,"content_chars":20,"elapsed_ms":50,"prompt_tokens":100,"gen_tokens":50}"#;
        let (session_id, prompt, gen_tokens) = InferenceContextMeter::parse_trace_line(line).unwrap();
        assert_eq!(session_id, "s1");
        assert_eq!(prompt, 100);
        assert_eq!(gen_tokens, 50);
    }

    #[test]
    fn parse_trace_line_missing_tokens() {
        let line = r#"{"schema":"cynic.trace.chat_turn.v1","ts_unix_ms":1000,"session_id":"s1","status":"ok","tool_turns":0,"prompt_chars":10,"content_chars":20,"elapsed_ms":50}"#;
        let (session_id, prompt, gen_tokens) = InferenceContextMeter::parse_trace_line(line).unwrap();
        assert_eq!(session_id, "s1");
        assert_eq!(prompt, 0);
        assert_eq!(gen_tokens, 0);
    }

    #[test]
    fn parse_trace_line_invalid() {
        let line = "not json";
        assert!(InferenceContextMeter::parse_trace_line(line).is_none());
    }
}
