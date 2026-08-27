//! cot_middleware.rs — Chain-of-Thought (CoT) Token Stream Middleware & Speculative Alpha Monitor.
//!
//! Implements runtime monitoring requirements from `cynic-agent-isolation-plan.md`
//! and `cynic-hardening-hf-incident.md`:
//! - Real-time token-by-token parsing of `<thinking>...</thinking>` tags in streaming SSE.
//! - Extraction of internal reasoning traces vs final visible outputs.
//! - Speculative acceptance rate (\alpha) monitoring with anomaly detection.
//! - Circuit-breaker on excessive reasoning token bloat (thermodynamic cutoff).

use serde::{Deserialize, Serialize};

/// Event emitted as streaming tokens pass through the CoT middleware.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CotStreamEvent {
    /// Token belonging to the model's internal reasoning trace (<thinking>).
    ThinkingChunk(String),
    /// Token belonging to the final visible answer.
    ContentChunk(String),
    /// Transition from thinking to final content emission.
    ThinkingComplete {
        thinking_tokens: u32,
        duration_ms: Option<u64>,
    },
    /// End of stream with aggregate token counts.
    StreamComplete {
        total_tokens: u32,
        thinking_tokens: u32,
        content_tokens: u32,
    },
}

/// State of the CoT streaming parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CotParserState {
    Initial,
    InsideThinking,
    AfterThinking,
}

/// Token-by-token streaming parser for Chain-of-Thought tags.
#[derive(Debug, Clone)]
pub struct CotStreamParser {
    state: CotParserState,
    buffer: String,
    thinking_buffer: String,
    content_buffer: String,
    thinking_tokens_count: u32,
    content_tokens_count: u32,
    max_thinking_tokens: u32,
    aborted_by_circuit_breaker: bool,
}

impl CotStreamParser {
    pub const DEFAULT_MAX_THINKING_TOKENS: u32 = 8192;

    pub fn new() -> Self {
        Self {
            state: CotParserState::Initial,
            buffer: String::new(),
            thinking_buffer: String::new(),
            content_buffer: String::new(),
            thinking_tokens_count: 0,
            content_tokens_count: 0,
            max_thinking_tokens: Self::DEFAULT_MAX_THINKING_TOKENS,
            aborted_by_circuit_breaker: false,
        }
    }

    pub fn with_max_thinking_tokens(max_tokens: u32) -> Self {
        let mut parser = Self::new();
        parser.max_thinking_tokens = max_tokens;
        parser
    }

    pub fn process_token(&mut self, token: &str) -> Vec<CotStreamEvent> {
        let mut events = Vec::new();
        if self.aborted_by_circuit_breaker {
            return events;
        }

        self.buffer.push_str(token);

        match self.state {
            CotParserState::Initial => {
                if self.buffer.contains("<thinking>") {
                    self.state = CotParserState::InsideThinking;
                    if let Some(pos) = self.buffer.find("<thinking>") {
                        let prefix = &self.buffer[..pos];
                        if !prefix.is_empty() {
                            events.push(CotStreamEvent::ContentChunk(prefix.to_string()));
                            self.content_buffer.push_str(prefix);
                            self.content_tokens_count += 1;
                        }
                        let after = &self.buffer[pos + "<thinking>".len()..];
                        if !after.is_empty() {
                            events.push(CotStreamEvent::ThinkingChunk(after.to_string()));
                            self.thinking_buffer.push_str(after);
                            self.thinking_tokens_count += 1;
                        }
                    }
                    self.buffer.clear();
                } else if !self.buffer.starts_with('<') && self.buffer.len() > 10 {
                    // Not a thinking tag start -> regular content
                    events.push(CotStreamEvent::ContentChunk(token.to_string()));
                    self.content_buffer.push_str(token);
                    self.content_tokens_count += 1;
                    self.state = CotParserState::AfterThinking;
                    self.buffer.clear();
                }
            }
            CotParserState::InsideThinking => {
                self.thinking_tokens_count += 1;

                // Circuit breaker check
                if self.thinking_tokens_count > self.max_thinking_tokens {
                    self.aborted_by_circuit_breaker = true;
                    events.push(CotStreamEvent::ThinkingComplete {
                        thinking_tokens: self.thinking_tokens_count,
                        duration_ms: None,
                    });
                    return events;
                }

                if self.buffer.contains("</thinking>") {
                    self.state = CotParserState::AfterThinking;
                    if let Some(pos) = self.buffer.find("</thinking>") {
                        let thinking_part = &self.buffer[..pos];
                        if !thinking_part.is_empty() {
                            events.push(CotStreamEvent::ThinkingChunk(thinking_part.to_string()));
                            self.thinking_buffer.push_str(thinking_part);
                        }
                        events.push(CotStreamEvent::ThinkingComplete {
                            thinking_tokens: self.thinking_tokens_count,
                            duration_ms: None,
                        });
                        let after = &self.buffer[pos + "</thinking>".len()..];
                        if !after.is_empty() {
                            events.push(CotStreamEvent::ContentChunk(after.to_string()));
                            self.content_buffer.push_str(after);
                            self.content_tokens_count += 1;
                        }
                    }
                    self.buffer.clear();
                } else {
                    events.push(CotStreamEvent::ThinkingChunk(token.to_string()));
                    self.thinking_buffer.push_str(token);
                }
            }
            CotParserState::AfterThinking => {
                self.content_tokens_count += 1;
                events.push(CotStreamEvent::ContentChunk(token.to_string()));
                self.content_buffer.push_str(token);
            }
        }

        events
    }

    pub fn finish(&mut self) -> Vec<CotStreamEvent> {
        let mut events = Vec::new();
        if !self.buffer.is_empty() {
            if self.state == CotParserState::InsideThinking {
                events.push(CotStreamEvent::ThinkingChunk(self.buffer.clone()));
                events.push(CotStreamEvent::ThinkingComplete {
                    thinking_tokens: self.thinking_tokens_count,
                    duration_ms: None,
                });
            } else {
                events.push(CotStreamEvent::ContentChunk(self.buffer.clone()));
            }
            self.buffer.clear();
        }

        events.push(CotStreamEvent::StreamComplete {
            total_tokens: self.thinking_tokens_count + self.content_tokens_count,
            thinking_tokens: self.thinking_tokens_count,
            content_tokens: self.content_tokens_count,
        });

        events
    }

    pub fn thinking_content(&self) -> &str {
        &self.thinking_buffer
    }

    pub fn final_content(&self) -> &str {
        &self.content_buffer
    }

    pub fn is_circuit_breaker_tripped(&self) -> bool {
        self.aborted_by_circuit_breaker
    }
}

impl Default for CotStreamParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Speculative decoding acceptance rate (\alpha) monitor.
///
/// Tracks \alpha = accepted_tokens / draft_tokens.
/// A sharp drop in \alpha on a specific prompt is an anomaly indicator
/// that signals potential adversarial divergence between draft and target models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeculativeAlphaMonitor {
    pub total_draft_tokens: u64,
    pub total_accepted_tokens: u64,
    pub baseline_alpha: f64,
    pub critical_threshold: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlphaAnomalyStatus {
    Normal,
    LowEfficiencyWarning,
    AdversarialDivergenceAlert,
}

impl SpeculativeAlphaMonitor {
    pub const DEFAULT_BASELINE_ALPHA: f64 = 0.70;
    pub const DEFAULT_CRITICAL_THRESHOLD: f64 = 0.25;

    pub fn new() -> Self {
        Self {
            total_draft_tokens: 0,
            total_accepted_tokens: 0,
            baseline_alpha: Self::DEFAULT_BASELINE_ALPHA,
            critical_threshold: Self::DEFAULT_CRITICAL_THRESHOLD,
        }
    }

    pub fn record_speculative_step(&mut self, draft_count: u32, accepted_count: u32) {
        self.total_draft_tokens += u64::from(draft_count);
        self.total_accepted_tokens += u64::from(accepted_count.min(draft_count));
    }

    pub fn current_alpha(&self) -> f64 {
        if self.total_draft_tokens == 0 {
            self.baseline_alpha
        } else {
            self.total_accepted_tokens as f64 / self.total_draft_tokens as f64
        }
    }

    pub fn check_anomaly(&self, step_draft: u32, step_accepted: u32) -> AlphaAnomalyStatus {
        if step_draft == 0 {
            return AlphaAnomalyStatus::Normal;
        }
        let step_alpha = step_accepted as f64 / step_draft as f64;
        if step_alpha < self.critical_threshold && step_draft >= 4 {
            AlphaAnomalyStatus::AdversarialDivergenceAlert
        } else if step_alpha < self.baseline_alpha * 0.6 {
            AlphaAnomalyStatus::LowEfficiencyWarning
        } else {
            AlphaAnomalyStatus::Normal
        }
    }
}

impl Default for SpeculativeAlphaMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cot_stream_parser_thinking_extraction() {
        let mut parser = CotStreamParser::new();
        let tokens = vec![
            "<thinking>",
            "Let's analyze ",
            "the bug in the ",
            "scheduler.",
            "</thinking>",
            "The bug is in line 42.",
        ];

        let mut all_events = Vec::new();
        for t in tokens {
            all_events.extend(parser.process_token(t));
        }
        all_events.extend(parser.finish());

        assert_eq!(
            parser.thinking_content(),
            "Let's analyze the bug in the scheduler."
        );
        assert_eq!(parser.final_content(), "The bug is in line 42.");
        assert!(!parser.is_circuit_breaker_tripped());
    }

    #[test]
    fn test_cot_circuit_breaker_limits_runaway_reasoning() {
        let mut parser = CotStreamParser::with_max_thinking_tokens(3);
        parser.process_token("<thinking>");
        parser.process_token("one ");
        parser.process_token("two ");
        parser.process_token("three ");
        let events = parser.process_token("four "); // 4th token exceeds max 3

        assert!(parser.is_circuit_breaker_tripped());
        assert!(events.iter().any(|e| matches!(e, CotStreamEvent::ThinkingComplete { .. })));
    }

    #[test]
    fn test_speculative_alpha_monitor_anomaly_detection() {
        let mut monitor = SpeculativeAlphaMonitor::new();
        monitor.record_speculative_step(10, 8); // 80% acceptance
        assert_eq!(monitor.current_alpha(), 0.80);

        // Normal step
        assert_eq!(
            monitor.check_anomaly(5, 4),
            AlphaAnomalyStatus::Normal
        );

        // Sharp drop -> adversarial divergence alert (1/8 = 12.5% < 25%)
        assert_eq!(
            monitor.check_anomaly(8, 1),
            AlphaAnomalyStatus::AdversarialDivergenceAlert
        );
    }
}
