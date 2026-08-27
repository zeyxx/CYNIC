//! Failure gallery for inference breakdowns.
//!
//! From Ahmad's "Step-By-Step LLM Engineering Projects":
//!   "Post the weird repetition loop. Post the expert-routing collapse.
//!    Post the quantization regression. Post the safety failure you did
//!    not expect. That is how fundamentals compound."
//!
//! A failure gallery is a durable record of how the system breaks, so
//! that "every bug is evidence of an unmodelled contract" becomes
//! actionable.

use serde::{Deserialize, Serialize};

/// A recorded failure scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureGalleryEntry {
    /// Human-readable scenario name.
    pub scenario: String,
    /// What the system was expected to do.
    pub expected_behavior: String,
    /// What actually happened.
    pub actual_behavior: String,
    /// Root cause once identified.
    pub root_cause: String,
    /// Steps to reproduce.
    pub reproduction_steps: Vec<String>,
    /// Optional artifact reference (log dump, trace, sha256).
    pub artifact_ref: Option<String>,
    /// Whether this failure has been fixed.
    pub resolved: bool,
    /// The fix (if resolved).
    pub fix_description: Option<String>,
}

/// Durable collection and classifier for inference failures.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FailureGallery {
    pub entries: Vec<FailureGalleryEntry>,
}

impl FailureGallery {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn record(&mut self, entry: FailureGalleryEntry) {
        self.entries.push(entry);
    }

    pub fn record_symptom(
        &mut self,
        scenario: &str,
        symptom: &str,
        reproduction_steps: Vec<String>,
    ) {
        let category = classify_failure(symptom);
        let remediation = suggested_fix(category);
        self.entries.push(FailureGalleryEntry {
            scenario: scenario.to_string(),
            expected_behavior: "Expected successful aligned completion".to_string(),
            actual_behavior: symptom.to_string(),
            root_cause: format!("classified_as:{category}"),
            reproduction_steps,
            artifact_ref: None,
            resolved: false,
            fix_description: Some(remediation.to_string()),
        });
    }

    pub fn unresolved(&self) -> Vec<&FailureGalleryEntry> {
        self.entries.iter().filter(|e| !e.resolved).collect()
    }

    pub fn by_category(&self, category: FailureCategory) -> Vec<&FailureGalleryEntry> {
        self.entries
            .iter()
            .filter(|e| classify_failure(&e.actual_behavior) == category)
            .collect()
    }
}

/// Pure function: classify a failure by symptom.
///
/// From Ahmad's "Failure Modes and Fixes" — common local LLM failures.
pub fn classify_failure(symptom: &str) -> FailureCategory {
    let lower = symptom.to_ascii_lowercase();
    if lower.contains("out of memory")
        || lower.contains("oom")
        || lower.contains("cuda") && lower.contains("memory")
    {
        FailureCategory::OutOfMemory
    } else if lower.contains("gibberish")
        || lower.contains("role confusion")
        || lower.contains("template")
    {
        FailureCategory::ChatTemplate
    } else if lower.contains("first token")
        || lower.contains("slow prompt")
        || lower.contains("prefill")
    {
        FailureCategory::SlowPrefill
    } else if lower.contains("slow")
        || lower.contains("decode")
        || lower.contains("token") && lower.contains("second")
    {
        FailureCategory::SlowDecode
    } else if lower.contains("document") || lower.contains("retrieval") || lower.contains("rag") {
        FailureCategory::RetrievalFailure
    } else if lower.contains("json") || lower.contains("schema") || lower.contains("tool call") {
        FailureCategory::StructuredOutput
    } else if lower.contains("repeat") || lower.contains("loop") {
        FailureCategory::RepetitionLoop
    } else {
        FailureCategory::Other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    OutOfMemory,
    ChatTemplate,
    SlowPrefill,
    SlowDecode,
    RetrievalFailure,
    StructuredOutput,
    RepetitionLoop,
    Other,
}

impl std::fmt::Display for FailureCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "out_of_memory"),
            Self::ChatTemplate => write!(f, "chat_template"),
            Self::SlowPrefill => write!(f, "slow_prefill"),
            Self::SlowDecode => write!(f, "slow_decode"),
            Self::RetrievalFailure => write!(f, "retrieval_failure"),
            Self::StructuredOutput => write!(f, "structured_output"),
            Self::RepetitionLoop => write!(f, "repetition_loop"),
            Self::Other => write!(f, "other"),
        }
    }
}

/// Suggested remediation for each category.
pub fn suggested_fix(category: FailureCategory) -> &'static str {
    match category {
        FailureCategory::OutOfMemory => {
            "Use a smaller model, reduce context length, lower batch/concurrency, \
             choose a better quantization, or leave more VRAM headroom (10-30%)."
        }
        FailureCategory::ChatTemplate => {
            "Verify the chat template, tokenizer, BOS/EOS tokens, reasoning-mode \
             switch, and tool schema against the model card."
        }
        FailureCategory::SlowPrefill => {
            "Shorten the prompt, use prefix caching, improve retrieval, reduce \
             context length, or switch to a faster runtime."
        }
        FailureCategory::SlowDecode => {
            "Check memory bandwidth, quantization format, CPU spill, attention \
             backend, speculative decoding support, or model size vs hardware."
        }
        FailureCategory::RetrievalFailure => {
            "Inspect parsed text, chunk boundaries, metadata, top-k retrieval, \
             reranking, and citation grounding."
        }
        FailureCategory::StructuredOutput => {
            "Use lower temperature, constrained decoding, stricter schemas, better \
             examples, and a model tuned for tool use."
        }
        FailureCategory::RepetitionLoop => {
            "Reduce temperature or top-p, add repetition penalties, check stop \
             tokens, and verify the template is not causing the model to see its \
             own answer as a new prompt."
        }
        FailureCategory::Other => "Investigate logs, traces, and reproduction steps.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_out_of_memory() {
        assert_eq!(
            classify_failure("CUDA out of memory. Tried to allocate 2.00 GiB"),
            FailureCategory::OutOfMemory
        );
    }

    #[test]
    fn classify_chat_template() {
        assert_eq!(
            classify_failure("Gibberish output: role confusion"),
            FailureCategory::ChatTemplate
        );
    }

    #[test]
    fn classify_repetition_loop() {
        assert_eq!(
            classify_failure("Model is stuck in a repetition loop"),
            FailureCategory::RepetitionLoop
        );
    }

    #[test]
    fn classify_slow_decode() {
        assert_eq!(
            classify_failure("Streaming is very slow: 3 tokens per second"),
            FailureCategory::SlowDecode
        );
    }

    #[test]
    fn suggested_fix_for_oom() {
        let fix = suggested_fix(FailureCategory::OutOfMemory);
        assert!(fix.contains("headroom"));
    }

    #[test]
    fn all_categories_have_display() {
        let categories = [
            FailureCategory::OutOfMemory,
            FailureCategory::ChatTemplate,
            FailureCategory::SlowPrefill,
            FailureCategory::SlowDecode,
            FailureCategory::RetrievalFailure,
            FailureCategory::StructuredOutput,
            FailureCategory::RepetitionLoop,
            FailureCategory::Other,
        ];
        for cat in &categories {
            let displayed = format!("{}", cat);
            assert!(!displayed.is_empty());
        }
    }

    #[test]
    fn test_failure_gallery_recording_and_queries() {
        let mut gallery = FailureGallery::new();
        gallery.record_symptom(
            "qwen-35b-test",
            "CUDA out of memory",
            vec!["run with 128k context".to_string()],
        );
        gallery.record_symptom(
            "gemma-test",
            "Model is stuck in a repetition loop",
            vec!["repeat prompt".to_string()],
        );

        assert_eq!(gallery.entries.len(), 2);
        assert_eq!(gallery.unresolved().len(), 2);

        let oom_entries = gallery.by_category(FailureCategory::OutOfMemory);
        assert_eq!(oom_entries.len(), 1);
        assert_eq!(oom_entries[0].scenario, "qwen-35b-test");
    }
}
