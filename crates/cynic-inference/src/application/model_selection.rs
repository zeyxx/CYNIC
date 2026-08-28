//! model_selection.rs — Adaptive UCB Model Selector & Escalation Ladder.
//!
//! Implements Section 13 of the Local Benchmark Protocol:
//! - Utility calculation: Utility(m, t) = P(success | m, t) / E(TTCS | m, t)
//! - Multi-armed bandit UCB1 exploration for optimal Pareto model selection
//! - Systematic Escalation Ladder: FastWorker -> EnhancedRetrieval -> BalancedMoE -> QualityTier

use serde::{Deserialize, Serialize};

use crate::domain::backend_identity::BackendHealthSnapshot;
use crate::domain::routing::{RoutingDecision, RoutingIntent};
use crate::math::ucb_sampler::{Ucb1Arm, ucb1_score};

use super::routing_policy::select_backend;

/// Empirical performance statistics for a specific model configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelEmpiricalStats {
    pub model_alias: String,
    pub success_count: u64,
    pub total_attempts: u64,
    pub avg_ttcs_ms: f64,
    pub avg_tokens_per_task: u64,
    pub avg_wh_per_task: f64,
}

impl ModelEmpiricalStats {
    pub fn new(model_alias: &str) -> Self {
        Self {
            model_alias: model_alias.to_string(),
            success_count: 0,
            total_attempts: 0,
            avg_ttcs_ms: 10_000.0, // default prior: 10s
            avg_tokens_per_task: 2048,
            avg_wh_per_task: 0.1,
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            0.5 // neutral prior
        } else {
            self.success_count as f64 / self.total_attempts as f64
        }
    }

    /// Normalized economic utility in [0, 1].
    /// Higher is better: rewards high success rate and fast TTCS.
    pub fn normalized_utility(&self) -> f64 {
        let p_success = self.success_rate();
        let ttcs_secs = (self.avg_ttcs_ms / 1000.0).max(1.0);
        // Base utility: tasks per minute equivalent
        let raw_utility = (p_success * 60.0) / ttcs_secs;
        // Sigmoidal compression into [0, 1]
        raw_utility / (raw_utility + 1.0)
    }

    pub fn to_ucb_arm(&self) -> Ucb1Arm {
        if self.total_attempts == 0 {
            Ucb1Arm::unobserved()
        } else {
            let utility = self.normalized_utility();
            let mut arm = Ucb1Arm::unobserved();
            for _ in 0..self.total_attempts {
                let _ = arm.record(utility);
            }
            arm
        }
    }
}

/// Adaptive model selector balancing exploitation (high utility) and exploration (UCB1).
#[derive(Debug, Clone, Default)]
pub struct AdaptiveModelSelector {
    stats: Vec<ModelEmpiricalStats>,
}

impl AdaptiveModelSelector {
    pub fn new() -> Self {
        Self { stats: Vec::new() }
    }

    pub fn register_model_stats(&mut self, stats: ModelEmpiricalStats) {
        if let Some(existing) = self.stats.iter_mut().find(|s| s.model_alias == stats.model_alias) {
            *existing = stats;
        } else {
            self.stats.push(stats);
        }
    }

    /// Selects the optimal model alias using UCB1 over normalized economic utility.
    pub fn select_optimal_model(&self, candidate_aliases: &[&str]) -> Option<String> {
        if candidate_aliases.is_empty() {
            return None;
        }

        let total_samples: u64 = self.stats.iter().map(|s| s.total_attempts).sum();

        let mut best_alias = None;
        let mut best_score = -f64::INFINITY;

        for &alias in candidate_aliases {
            let model_stat = self.stats.iter().find(|s| s.model_alias == alias);
            let arm = model_stat.map(|s| s.to_ucb_arm()).unwrap_or_else(Ucb1Arm::unobserved);

            if arm.sample_count() == 0 {
                // Unobserved candidate gets immediate priority for exploration
                return Some(alias.to_string());
            }

            if let Some(score) = ucb1_score(arm, total_samples.max(1)) {
                if score > best_score {
                    best_score = score;
                    best_alias = Some(alias.to_string());
                }
            }
        }

        best_alias.or_else(|| candidate_aliases.first().map(|s| s.to_string()))
    }
}

/// Escalation tiers defined in the benchmark and agentic protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EscalationTier {
    /// Level 0: Fast efficient worker (e.g. Q2/IQ2, MoE with low active params).
    Tier0FastWorker,
    /// Level 1: Context enrichment (retrieve more relevant context before scaling compute).
    Tier1EnhancedRetrieval,
    /// Level 2: Balanced quality/speed tier (e.g. Q3/IQ3, larger MoE).
    Tier2BalancedMoE,
    /// Level 3: Quality tier (e.g. Q4 / dense reasoning model).
    Tier3QualityModel,
}

/// Escalation decision signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscalationSignal {
    ValidationSuccess,
    CompilationOrTestFailure,
    InsufficientContext,
    RepetitionOrToolLoop,
    LowConfidenceOrContradiction,
    PersistentFailure,
    TimeoutOrExcessiveCompute,
}

/// Action to execute following an escalation signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscalationAction {
    AcceptAndMemorize,
    RetryWithDiagnosticContext,
    RetrieveAdditionalContext,
    ChangeStrategyOrModel,
    ArbitrateWithVerifier,
    EscalateComputeTier(EscalationTier),
    DecomposeTask,
}

/// Pure rule-based escalation resolver.
pub fn resolve_escalation_action(
    current_tier: EscalationTier,
    signal: &EscalationSignal,
) -> EscalationAction {
    match signal {
        EscalationSignal::ValidationSuccess => EscalationAction::AcceptAndMemorize,
        EscalationSignal::CompilationOrTestFailure => {
            EscalationAction::RetryWithDiagnosticContext
        }
        EscalationSignal::InsufficientContext => EscalationAction::RetrieveAdditionalContext,
        EscalationSignal::RepetitionOrToolLoop => EscalationAction::ChangeStrategyOrModel,
        EscalationSignal::LowConfidenceOrContradiction => EscalationAction::ArbitrateWithVerifier,
        EscalationSignal::PersistentFailure => match current_tier {
            EscalationTier::Tier0FastWorker => {
                EscalationAction::EscalateComputeTier(EscalationTier::Tier1EnhancedRetrieval)
            }
            EscalationTier::Tier1EnhancedRetrieval => {
                EscalationAction::EscalateComputeTier(EscalationTier::Tier2BalancedMoE)
            }
            EscalationTier::Tier2BalancedMoE => {
                EscalationAction::EscalateComputeTier(EscalationTier::Tier3QualityModel)
            }
            EscalationTier::Tier3QualityModel => EscalationAction::DecomposeTask,
        },
        EscalationSignal::TimeoutOrExcessiveCompute => EscalationAction::DecomposeTask,
    }
}

/// Standard model selection delegate (with fallback support).
pub fn select_model(
    backends: &[BackendHealthSnapshot],
    intent: &RoutingIntent,
) -> Option<RoutingDecision> {
    select_backend(backends, intent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_empirical_stats_utility() {
        let mut stats = ModelEmpiricalStats::new("qwen-35b-iq2");
        stats.total_attempts = 10;
        stats.success_count = 9;
        stats.avg_ttcs_ms = 4000.0; // 4s

        assert_eq!(stats.success_rate(), 0.9);
        let utility = stats.normalized_utility();
        assert!(utility > 0.5);
    }

    #[test]
    fn test_adaptive_selector_prioritizes_unobserved_candidates() {
        let mut selector = AdaptiveModelSelector::new();
        let mut known = ModelEmpiricalStats::new("model-a");
        known.total_attempts = 20;
        known.success_count = 18;
        selector.register_model_stats(known);

        // model-b has zero observations -> must be chosen for exploration
        let candidates = ["model-a", "model-b"];
        let chosen = selector.select_optimal_model(&candidates);
        assert_eq!(chosen.as_deref(), Some("model-b"));
    }

    #[test]
    fn test_escalation_ladder_progression() {
        let act0 = resolve_escalation_action(
            EscalationTier::Tier0FastWorker,
            &EscalationSignal::PersistentFailure,
        );
        assert_eq!(
            act0,
            EscalationAction::EscalateComputeTier(EscalationTier::Tier1EnhancedRetrieval)
        );

        let act1 = resolve_escalation_action(
            EscalationTier::Tier1EnhancedRetrieval,
            &EscalationSignal::PersistentFailure,
        );
        assert_eq!(
            act1,
            EscalationAction::EscalateComputeTier(EscalationTier::Tier2BalancedMoE)
        );

        let act3 = resolve_escalation_action(
            EscalationTier::Tier3QualityModel,
            &EscalationSignal::PersistentFailure,
        );
        assert_eq!(act3, EscalationAction::DecomposeTask);

        let success = resolve_escalation_action(
            EscalationTier::Tier0FastWorker,
            &EscalationSignal::ValidationSuccess,
        );
        assert_eq!(success, EscalationAction::AcceptAndMemorize);
    }
}
