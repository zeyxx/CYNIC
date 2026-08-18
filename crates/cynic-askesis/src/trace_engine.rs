//! ML Data Engine — Structured Agent Trace Collection & Synthetic Dataset Synthesis
//!
//! Transforms live agent interactions, tool calls, and verified outcomes into
//! canonical, provenance-backed training pairs for continuous local fine-tuning.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

pub const TRAINING_TRACE_SCHEMA: &str = "cynic.ml.training_trace.v1";

/// A structured training pair derived from verified execution traces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrainingTrace {
    pub schema: String,
    pub trace_id: String,
    pub domain: String,
    pub context_evidence_digests: Vec<String>,
    pub input_prompt: String,
    pub action_or_prediction: serde_json::Value,
    pub observed_outcome: serde_json::Value,
    pub verdict: String,
    pub q_score: Option<f64>,
    pub timestamp_unix_ms: u128,
}

impl TrainingTrace {
    pub fn new(
        trace_id: impl Into<String>,
        domain: impl Into<String>,
        input_prompt: impl Into<String>,
        action_or_prediction: serde_json::Value,
        observed_outcome: serde_json::Value,
        verdict: impl Into<String>,
        q_score: Option<f64>,
        context_evidence_digests: Vec<String>,
    ) -> Self {
        let timestamp_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        Self {
            schema: TRAINING_TRACE_SCHEMA.to_string(),
            trace_id: trace_id.into(),
            domain: domain.into(),
            context_evidence_digests,
            input_prompt: input_prompt.into(),
            action_or_prediction,
            observed_outcome,
            verdict: verdict.into(),
            q_score,
            timestamp_unix_ms,
        }
    }

    /// Converts this trace into an OpenAI/ChatML compatible fine-tuning JSON pair.
    pub fn to_fine_tuning_format(&self) -> serde_json::Value {
        serde_json::json!({
            "messages": [
                {
                    "role": "system",
                    "content": format!("You are a CYNIC sovereign AI agent operating in the domain: {}.", self.domain)
                },
                {
                    "role": "user",
                    "content": self.input_prompt
                },
                {
                    "role": "assistant",
                    "content": serde_json::to_string(&self.action_or_prediction).unwrap_or_default()
                }
            ],
            "metadata": {
                "trace_id": self.trace_id,
                "q_score": self.q_score,
                "verdict": self.verdict,
                "evidence_digests": self.context_evidence_digests
            }
        })
    }
}

/// Trace storage and dataset export manager.
pub struct TraceEngine;

impl TraceEngine {
    /// Appends a new training trace to the specified JSONL trace log.
    pub fn append_trace(path: &Path, trace: &TrainingTrace) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        let line = serde_json::to_string(trace)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(file, "{}", line)
    }

    /// Loads and filters high-quality training traces (e.g. Q-score >= min_q_score).
    pub fn load_filtered_traces(path: &Path, min_q_score: Option<f64>) -> std::io::Result<Vec<TrainingTrace>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut traces = Vec::new();

        for line_res in reader.lines() {
            let line = line_res?;
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            if let Ok(trace) = serde_json::from_str::<TrainingTrace>(&line) {
                if let Some(min_q) = min_q_score {
                    if trace.q_score.unwrap_or(0.0) < min_q {
                        continue;
                    }
                }
                traces.push(trace);
            }
        }
        Ok(traces)
    }

    /// Exports filtered traces into an OpenAI-compatible JSONL dataset for fine-tuning.
    pub fn export_fine_tuning_dataset(
        source_path: &Path,
        dest_path: &Path,
        min_q_score: Option<f64>,
    ) -> std::io::Result<usize> {
        let traces = Self::load_filtered_traces(source_path, min_q_score)?;
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = File::create(dest_path)?;
        for trace in &traces {
            let formatted = trace.to_fine_tuning_format();
            writeln!(file, "{}", serde_json::to_string(&formatted).unwrap_or_default())?;
        }
        Ok(traces.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn trace_engine_appends_and_filters_high_quality_traces() {
        let temp_trace_file = NamedTempFile::new().unwrap();
        let path = temp_trace_file.path();

        let trace1 = TrainingTrace::new(
            "trace-001",
            "coordination",
            "How do agents claim a file lease?",
            serde_json::json!({"action": "cynic_coord_claim", "fencing_token": 1}),
            serde_json::json!({"status": "claimed"}),
            "HOWL",
            Some(0.95),
            vec!["sha256:abcd".into()],
        );

        let trace2 = TrainingTrace::new(
            "trace-002",
            "inference",
            "Bad prediction with low q score",
            serde_json::json!({"action": "bad_call"}),
            serde_json::json!({"status": "failed"}),
            "BARK",
            Some(0.25),
            vec![],
        );

        TraceEngine::append_trace(path, &trace1).unwrap();
        TraceEngine::append_trace(path, &trace2).unwrap();

        let high_quality = TraceEngine::load_filtered_traces(path, Some(0.80)).unwrap();
        assert_eq!(high_quality.len(), 1);
        assert_eq!(high_quality[0].trace_id, "trace-001");

        let temp_export = NamedTempFile::new().unwrap();
        let exported_count = TraceEngine::export_fine_tuning_dataset(path, temp_export.path(), Some(0.80)).unwrap();
        assert_eq!(exported_count, 1);
    }
}
