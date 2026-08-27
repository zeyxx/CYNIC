//! generation_parameters.rs — Granular Generation & Sampling Runtime Controls.
//!
//! Models first-class runtime inference parameters:
//! - Thinking Mode / Chain-of-Thought controls (budget, enable/disable, custom delimiters).
//! - Sampling Dynamics (temperature, top_p, min_p, top_k, repetition/presence penalties, seed).
//! - Constrained Decoding (Grammar GBNF, JSON Schema, stop sequences).

use serde::{Deserialize, Serialize};

/// Mode and budget for Chain-of-Thought / reasoning generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingMode {
    /// Thinking enabled with optional max token budget and output stripping.
    Enabled {
        budget_tokens: Option<u32>,
        strip_from_visible_output: bool,
    },
    /// Thinking explicitly disabled (e.g. for fast direct tool calling or low latency).
    Disabled,
    /// Custom thinking delimiters for non-standard architectures.
    CustomDelimiters {
        open_tag: String,
        close_tag: String,
        budget_tokens: Option<u32>,
    },
}

impl Default for ThinkingMode {
    fn default() -> Self {
        Self::Enabled {
            budget_tokens: Some(4096),
            strip_from_visible_output: false,
        }
    }
}

/// Granular sampling parameters for token generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SamplingParameters {
    /// Softmax temperature (> 0.0). 0.0 implies greedy decoding.
    pub temperature: f32,
    /// Nucleus sampling threshold (top_p in (0.0, 1.0]).
    pub top_p: Option<f32>,
    /// Top-k candidate filtering.
    pub top_k: Option<u32>,
    /// Minimum probability threshold relative to the most likely token (min_p).
    pub min_p: Option<f32>,
    /// Repetition penalty factor (> 1.0 penalizes recently seen tokens).
    pub repetition_penalty: Option<f32>,
    /// Presence penalty (penalizes any token that has appeared at least once).
    pub presence_penalty: Option<f32>,
    /// Frequency penalty (penalizes tokens proportionally to their frequency).
    pub frequency_penalty: Option<f32>,
    /// Random seed for deterministic reproducibility in benchmarks.
    pub seed: Option<u64>,
    /// Custom stop sequences that terminate generation early.
    pub stop_sequences: Vec<String>,
    /// Constrained decoding grammar (e.g. GBNF).
    pub grammar: Option<String>,
    /// Strict JSON schema constraint.
    pub json_schema: Option<serde_json::Value>,
}

impl Default for SamplingParameters {
    fn default() -> Self {
        Self {
            temperature: 0.2, // low default for reasoning & coding fidelity
            top_p: Some(0.95),
            top_k: Some(40),
            min_p: Some(0.05),
            repetition_penalty: Some(1.05),
            presence_penalty: None,
            frequency_penalty: None,
            seed: Some(42),
            stop_sequences: Vec::new(),
            grammar: None,
            json_schema: None,
        }
    }
}

/// Complete generation envelope passed to inference runtime ports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GenerationParameters {
    pub thinking: ThinkingMode,
    pub sampling: SamplingParameters,
    pub max_output_tokens: u32,
}

impl GenerationParameters {
    /// Preset: Deterministic reasoning & mathematical verification.
    pub fn deterministic_reasoning(max_tokens: u32) -> Self {
        Self {
            thinking: ThinkingMode::Enabled {
                budget_tokens: Some(max_tokens),
                strip_from_visible_output: false,
            },
            sampling: SamplingParameters {
                temperature: 0.0, // greedy deterministic
                seed: Some(42),
                ..Default::default()
            },
            max_output_tokens: max_tokens,
        }
    }

    /// Preset: Agentic coding & tool invocation (structured JSON, thinking enabled).
    pub fn agentic_coding(max_tokens: u32, schema: Option<serde_json::Value>) -> Self {
        Self {
            thinking: ThinkingMode::Enabled {
                budget_tokens: Some(max_tokens / 2),
                strip_from_visible_output: false,
            },
            sampling: SamplingParameters {
                temperature: 0.2,
                json_schema: schema,
                ..Default::default()
            },
            max_output_tokens: max_tokens,
        }
    }

    /// Preset: Fast low-latency classification (no thinking, temperature 0.1).
    pub fn fast_classification(max_tokens: u32) -> Self {
        Self {
            thinking: ThinkingMode::Disabled,
            sampling: SamplingParameters {
                temperature: 0.1,
                top_p: Some(0.9),
                ..Default::default()
            },
            max_output_tokens: max_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_reasoning_preset() {
        let params = GenerationParameters::deterministic_reasoning(2048);
        assert_eq!(params.sampling.temperature, 0.0);
        assert_eq!(params.max_output_tokens, 2048);
        assert!(matches!(params.thinking, ThinkingMode::Enabled { .. }));
    }

    #[test]
    fn test_fast_classification_preset_disables_thinking() {
        let params = GenerationParameters::fast_classification(64);
        assert_eq!(params.thinking, ThinkingMode::Disabled);
        assert_eq!(params.max_output_tokens, 64);
    }
}
