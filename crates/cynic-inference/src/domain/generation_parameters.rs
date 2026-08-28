//! generation_parameters.rs — Granular Generation & Sampling Runtime Controls.
//!
//! Models first-class runtime inference parameters:
//! - Granular Thinking Mode & Reasoning Effort (None, Low, Medium, High, Budget, Dynamic).
//! - Dual-phase Temperature (exploratory thinking_temperature vs deterministic output_temperature).
//! - Sampling Dynamics (temperature, top_p, min_p, top_k, repetition/presence penalties, seed).
//! - Constrained Decoding (Grammar GBNF, JSON Schema, stop sequences).

use serde::{Deserialize, Serialize};

/// Granular reasoning effort levels for thinking models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    /// Zero reasoning tokens — direct response only.
    None,
    /// Light reasoning (typically 256 to 512 tokens max).
    Low,
    /// Balanced standard reasoning (typically 1024 to 2048 tokens).
    Medium,
    /// Deep reasoning for complex mathematical or architectural tasks (4096 to 8192 tokens).
    High,
    /// Explicit numeric token budget cap.
    Budget(u32),
    /// Dynamic early-exit based on internal confidence / verification triggers.
    Dynamic,
}

impl ReasoningEffort {
    pub fn max_thinking_tokens(&self) -> Option<u32> {
        match self {
            Self::None => Some(0),
            Self::Low => Some(512),
            Self::Medium => Some(2048),
            Self::High => Some(8192),
            Self::Budget(b) => Some(*b),
            Self::Dynamic => None,
        }
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Advanced thinking control envelope with dual-phase temperature support.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingControl {
    /// Target reasoning effort / granularity.
    pub effort: ReasoningEffort,
    /// Whether to strip the internal thinking trace from the visible output.
    pub strip_from_visible_output: bool,
    /// Temperature used specifically during the thinking phase (typically 0.6 - 0.8 for exploration).
    pub thinking_temperature: Option<f32>,
    /// Custom opening and closing tags (defaults to "<thinking>" / "</thinking>").
    pub custom_tags: Option<(String, String)>,
}

impl Default for ThinkingControl {
    fn default() -> Self {
        Self {
            effort: ReasoningEffort::Medium,
            strip_from_visible_output: false,
            thinking_temperature: Some(0.6),
            custom_tags: None,
        }
    }
}

impl ThinkingControl {
    pub fn none() -> Self {
        Self {
            effort: ReasoningEffort::None,
            strip_from_visible_output: false,
            thinking_temperature: None,
            custom_tags: None,
        }
    }

    pub fn with_effort(effort: ReasoningEffort) -> Self {
        Self {
            effort,
            ..Default::default()
        }
    }

    pub fn with_budget(tokens: u32) -> Self {
        Self {
            effort: ReasoningEffort::Budget(tokens),
            ..Default::default()
        }
    }
}

/// Backward compatibility alias.
pub type ThinkingMode = ThinkingControl;

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
    pub thinking: ThinkingControl,
    pub sampling: SamplingParameters,
    pub max_output_tokens: u32,
}

impl GenerationParameters {
    /// Preset: Deterministic reasoning & mathematical verification (High effort, greedy output).
    pub fn deterministic_reasoning(max_tokens: u32) -> Self {
        Self {
            thinking: ThinkingControl {
                effort: ReasoningEffort::High,
                strip_from_visible_output: false,
                thinking_temperature: Some(0.4),
                custom_tags: None,
            },
            sampling: SamplingParameters {
                temperature: 0.0, // greedy deterministic
                seed: Some(42),
                ..Default::default()
            },
            max_output_tokens: max_tokens,
        }
    }

    /// Preset: Agentic coding & tool invocation (Medium effort, structured JSON).
    pub fn agentic_coding(max_tokens: u32, schema: Option<serde_json::Value>) -> Self {
        Self {
            thinking: ThinkingControl {
                effort: ReasoningEffort::Medium,
                strip_from_visible_output: false,
                thinking_temperature: Some(0.5),
                custom_tags: None,
            },
            sampling: SamplingParameters {
                temperature: 0.2,
                json_schema: schema,
                ..Default::default()
            },
            max_output_tokens: max_tokens,
        }
    }

    /// Preset: Fast low-latency classification (No thinking, low temperature).
    pub fn fast_classification(max_tokens: u32) -> Self {
        Self {
            thinking: ThinkingControl::none(),
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
    fn test_reasoning_effort_budgets() {
        assert_eq!(ReasoningEffort::None.max_thinking_tokens(), Some(0));
        assert_eq!(ReasoningEffort::Low.max_thinking_tokens(), Some(512));
        assert_eq!(ReasoningEffort::Medium.max_thinking_tokens(), Some(2048));
        assert_eq!(ReasoningEffort::High.max_thinking_tokens(), Some(8192));
        assert_eq!(ReasoningEffort::Budget(1500).max_thinking_tokens(), Some(1500));
        assert_eq!(ReasoningEffort::Dynamic.max_thinking_tokens(), None);
    }

    #[test]
    fn test_deterministic_reasoning_preset() {
        let params = GenerationParameters::deterministic_reasoning(2048);
        assert_eq!(params.sampling.temperature, 0.0);
        assert_eq!(params.max_output_tokens, 2048);
        assert_eq!(params.thinking.effort, ReasoningEffort::High);
    }

    #[test]
    fn test_fast_classification_preset_disables_thinking() {
        let params = GenerationParameters::fast_classification(64);
        assert_eq!(params.thinking.effort, ReasoningEffort::None);
        assert_eq!(params.max_output_tokens, 64);
    }
}
