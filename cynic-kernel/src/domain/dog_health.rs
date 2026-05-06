// domain/dog_health.rs — Per-Dog quality counters and evaluation outcome types.
//
// Shared vocabulary between judge/ (produces outcomes) and organ/ (tracks health).
// Lives in domain/ so neither layer depends on the other for these types.

use serde::{Deserialize, Serialize};

/// Failure type classification — different causes, different remediation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreFailureKind {
    /// >3 axioms at exactly 0.0. Model assigns zero to everything "bad". Capability limit.
    ZeroFlood,
    /// All axiom scores identical (variance ≈ 0). Model collapses to single value. Capability limit.
    Collapse,
    /// JSON not found, malformed, or no numeric scores after lenient parse.
    ParseError,
    /// Backend unreachable or timed out.
    Timeout,
    /// Backend returned HTTP error or was unreachable. Infrastructure issue, not model quality.
    ApiError,
}

/// Outcome of a single Dog evaluation, used to update organ health tracking.
#[derive(Debug, Clone, Copy)]
pub enum ScoreOutcome {
    Success {
        elapsed_ms: u64,
        completion_tokens: u32,
        thinking_tokens: u32,
    },
    Failure(ScoreFailureKind),
}

/// Per-Dog rolling counters. Updated after each Dog evaluation.
/// Does NOT lock — caller (Judge) holds the Arc<Mutex<DogStats>>.
/// Serializable for persistence across restarts (B5 fix — organ amnesia).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DogStats {
    /// Total calls attempted (success + all failure kinds).
    pub total_calls: u64,
    /// Calls that produced valid, differentiated scores (passed validate_scores).
    pub success_count: u64,
    /// Zero-flood failures: >3 axioms at 0.0.
    pub zero_flood_count: u64,
    /// Collapse failures: all axioms same value (variance < threshold).
    pub collapse_count: u64,
    /// Parse/JSON failures.
    pub parse_error_count: u64,
    /// Timeout failures.
    pub timeout_count: u64,
    /// API/infrastructure failures (backend unreachable, HTTP errors).
    pub api_error_count: u64,
    /// RFC3339 timestamp of last successful score (K14: None = never succeeded).
    #[serde(default)]
    pub last_success: Option<String>,
    /// Cumulative latency of successful calls (ms). Used to compute mean.
    pub total_latency_ms: u64,
    /// Cumulative completion tokens from successful calls.
    /// Drives dynamic budget: completion_mean = total_completion_tokens / success_count.
    #[serde(default)]
    pub total_completion_tokens: u64,
    /// Max completion tokens observed in any single call (empirical p100).
    #[serde(default)]
    pub max_completion_tokens: u32,
    /// Max content tokens observed (completion_tokens - thinking_tokens per call).
    /// Drives the content portion of completion_budget(). 0 for legacy persisted stats.
    #[serde(default)]
    pub max_content_tokens: u32,
    /// Max thinking tokens observed in any single call.
    /// Drives the thinking overhead portion of completion_budget(). 0 for non-thinking models.
    #[serde(default)]
    pub max_thinking_tokens: u32,
}

impl DogStats {
    pub fn new() -> Self {
        Self {
            total_calls: 0,
            success_count: 0,
            zero_flood_count: 0,
            collapse_count: 0,
            parse_error_count: 0,
            timeout_count: 0,
            api_error_count: 0,
            last_success: None,
            total_latency_ms: 0,
            total_completion_tokens: 0,
            max_completion_tokens: 0,
            max_content_tokens: 0,
            max_thinking_tokens: 0,
        }
    }

    pub fn record_success(&mut self) {
        self.total_calls += 1;
        self.success_count += 1;
        self.last_success = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn record_success_with_latency(&mut self, elapsed_ms: u64) {
        self.record_success();
        self.total_latency_ms += elapsed_ms;
    }

    /// Record completion token usage from a successful evaluation.
    /// Tracks content and thinking tokens separately for thinking-aware budget calibration.
    /// For non-thinking models, thinking_tokens = 0 and behavior is unchanged.
    pub fn record_completion_tokens(&mut self, completion_tokens: u32, thinking_tokens: u32) {
        self.total_completion_tokens += completion_tokens as u64;
        if completion_tokens > self.max_completion_tokens {
            self.max_completion_tokens = completion_tokens;
        }
        let content_tokens = completion_tokens.saturating_sub(thinking_tokens);
        if content_tokens > self.max_content_tokens {
            self.max_content_tokens = content_tokens;
        }
        if thinking_tokens > self.max_thinking_tokens {
            self.max_thinking_tokens = thinking_tokens;
        }
    }

    /// Mean latency of successful calls in milliseconds.
    /// Returns 0.0 when no successful calls recorded.
    pub fn mean_latency_ms(&self) -> f64 {
        if self.success_count == 0 {
            return 0.0;
        }
        self.total_latency_ms as f64 / self.success_count as f64
    }

    pub fn record_failure(&mut self, kind: ScoreFailureKind) {
        self.total_calls += 1;
        match kind {
            ScoreFailureKind::ZeroFlood => self.zero_flood_count += 1,
            ScoreFailureKind::Collapse => self.collapse_count += 1,
            ScoreFailureKind::ParseError => self.parse_error_count += 1,
            ScoreFailureKind::Timeout => self.timeout_count += 1,
            ScoreFailureKind::ApiError => self.api_error_count += 1,
        }
    }

    /// Fraction of calls that produced valid scores. K14: 0.0 when unknown (no calls yet).
    pub fn json_valid_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0; // K14: unknown = pessimistic
        }
        self.success_count as f64 / self.total_calls as f64
    }

    /// Fraction of calls that are capability-limit failures (zero flood + collapse).
    /// High value → model can't discriminate on this content type. Not fixable with config.
    pub fn capability_limit_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0;
        }
        (self.zero_flood_count + self.collapse_count) as f64 / self.total_calls as f64
    }

    /// Mean completion tokens per successful call.
    /// Returns 0 when no successful calls recorded.
    pub fn mean_completion_tokens(&self) -> u32 {
        if self.success_count == 0 {
            return 0;
        }
        (self.total_completion_tokens / self.success_count) as u32
    }

    /// Thinking-aware completion budget: content needs + thinking overhead.
    /// Returns None when baseline not yet established (< 20 calls).
    pub fn completion_budget(&self) -> Option<u32> {
        if !self.is_baseline_established() {
            return None;
        }
        if self.max_content_tokens > 0 || self.max_thinking_tokens > 0 {
            let content_budget = (self.max_content_tokens as f64 * 1.2).ceil() as u32;
            let thinking_budget = (self.max_thinking_tokens as f64 * 1.5).ceil() as u32;
            Some((content_budget + thinking_budget).min(4096))
        } else {
            let budget = (self.max_completion_tokens as f64 * 1.2).ceil() as u32;
            Some(budget.min(4096))
        }
    }

    /// Estimated tokens/second for this Dog from latency and completion data.
    /// Returns None when insufficient data.
    pub fn tok_per_sec(&self) -> Option<f64> {
        if self.success_count < 5 || self.total_latency_ms == 0 || self.total_completion_tokens == 0
        {
            return None;
        }
        let total_secs = self.total_latency_ms as f64 / 1000.0;
        Some(self.total_completion_tokens as f64 / total_secs)
    }

    /// Has seen enough calls to be statistically meaningful.
    pub fn is_baseline_established(&self) -> bool {
        self.total_calls >= 20
    }
}

impl Default for DogStats {
    fn default() -> Self {
        Self::new()
    }
}
