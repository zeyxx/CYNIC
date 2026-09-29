//! Append-only per-turn inference trace (data-centric ledger input).
//!
//! Pure types only: adapters serialize these lines to
//! `evidence/inference_traces.jsonl`. Token/latency fields stay explicit
//! `None` until backends report usage — unknown is data, never zero-filled.

use serde::{Deserialize, Serialize};

/// Machine-readable turn outcome for later eval/routing (never PII beyond
/// lengths — prompts and contents are NOT stored here).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatTrace {
    pub schema: String,
    pub ts_unix_ms: u64,
    pub session_id: String,
    pub model_alias: Option<String>,
    pub endpoint: Option<String>,
    /// `ok` | `blocked` | `error`.
    pub status: TraceStatus,
    pub tool_turns: u32,
    pub prompt_chars: usize,
    pub content_chars: usize,
    /// Wall time of the whole turn (ms).
    pub elapsed_ms: u64,
    /// Unknown until backends report usage.
    pub prompt_tokens: Option<u64>,
    /// Unknown until backends report usage.
    pub gen_tokens: Option<u64>,
    /// Unknown until streaming reports first-token timing.
    pub ttft_ms: Option<u64>,
    /// Unknown until backends report throughput.
    pub decode_tps: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    Ok,
    Blocked,
    Error,
}

impl ChatTrace {
    pub const SCHEMA: &'static str = "cynic.trace.chat_turn.v1";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ts_unix_ms: u64,
        session_id: impl Into<String>,
        model_alias: Option<String>,
        endpoint: Option<String>,
        status: TraceStatus,
        tool_turns: u32,
        prompt_chars: usize,
        content_chars: usize,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            schema: Self::SCHEMA.to_string(),
            ts_unix_ms,
            session_id: session_id.into(),
            model_alias,
            endpoint,
            status,
            tool_turns,
            prompt_chars,
            content_chars,
            elapsed_ms,
            prompt_tokens: None,
            gen_tokens: None,
            ttft_ms: None,
            decode_tps: None,
        }
    }
}

/// Trailing-window aggregates over recorded traces (pure).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TraceSummary {
    pub turns: usize,
    pub ok_rate: f64,
    pub mean_elapsed_ms: f64,
}

/// Summarize up to the whole slice (callers window it). Empty input is an
/// explicit zero, never NaN.
pub fn summarize_traces(traces: &[ChatTrace]) -> TraceSummary {
    if traces.is_empty() {
        return TraceSummary::default();
    }
    let ok = traces
        .iter()
        .filter(|t| t.status == TraceStatus::Ok)
        .count();
    let mean_elapsed_ms =
        traces.iter().map(|t| t.elapsed_ms as f64).sum::<f64>() / traces.len() as f64;
    TraceSummary {
        turns: traces.len(),
        ok_rate: ok as f64 / traces.len() as f64,
        mean_elapsed_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(status: TraceStatus, elapsed_ms: u64) -> ChatTrace {
        ChatTrace::new(1, "s", None, None, status, 0, 10, 20, elapsed_ms)
    }

    #[test]
    fn summary_counts_ok_rate_and_mean() {
        let traces = vec![
            trace(TraceStatus::Ok, 100),
            trace(TraceStatus::Ok, 300),
            trace(TraceStatus::Error, 200),
        ];
        let summary = summarize_traces(&traces);
        assert_eq!(summary.turns, 3);
        assert!((summary.ok_rate - 2.0 / 3.0).abs() < 1e-9);
        assert!((summary.mean_elapsed_ms - 200.0).abs() < 1e-9);
    }

    #[test]
    fn summary_empty_is_explicit_zero() {
        assert_eq!(summarize_traces(&[]), TraceSummary::default());
    }

    #[test]
    fn usage_fields_default_to_unknown() {
        let trace = trace(TraceStatus::Ok, 50);
        assert_eq!(trace.prompt_tokens, None);
        assert_eq!(trace.decode_tps, None);
    }
}
