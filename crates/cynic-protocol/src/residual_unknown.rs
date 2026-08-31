//! Residual unknown budget — Res1 / C18 epistemic honesty contract.
//!
//! Enforces the Res1 / C18 invariant: unknown state MUST remain an explicit
//! residual budget rather than being fabricated or coerced to zero.
//! When residual unknowns exceed the declared threshold, the system
//! transitions to an explicit degraded / insufficient evidence status.

use serde::{Deserialize, Serialize};

pub const RESIDUAL_UNKNOWN_BUDGET_SCHEMA: &str = "cynic.residual_unknown_budget.v1";
pub const RESIDUAL_UNKNOWN_EVALUATION_SCHEMA: &str = "cynic.residual_unknown_evaluation.v1";

/// Declared budget for residual unknowns in an observation or metrics domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidualUnknownBudget {
    pub schema: &'static str,
    /// Maximum allowed residual unknown percentage (0-100).
    /// Defaults to 40% (matching the minimum 60% known coverage threshold).
    pub max_residual_unknown_percent: u8,
    /// Optional hard ceiling on absolute unknown item count.
    pub max_unknown_count: Option<usize>,
}

impl Default for ResidualUnknownBudget {
    fn default() -> Self {
        Self {
            schema: RESIDUAL_UNKNOWN_BUDGET_SCHEMA,
            max_residual_unknown_percent: 40,
            max_unknown_count: None,
        }
    }
}

impl ResidualUnknownBudget {
    pub const fn new(max_residual_unknown_percent: u8, max_unknown_count: Option<usize>) -> Self {
        Self {
            schema: RESIDUAL_UNKNOWN_BUDGET_SCHEMA,
            max_residual_unknown_percent,
            max_unknown_count,
        }
    }

    /// Strict budget: at most 20% residual unknowns.
    pub const fn strict() -> Self {
        Self::new(20, None)
    }

    /// Industrial baseline: at most 40% residual unknowns (60% minimum known coverage).
    pub const fn standard() -> Self {
        Self::new(40, None)
    }
}

/// Point-in-time evaluation of residual unknowns against a budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidualUnknownEvaluation {
    pub schema: String,
    pub unknown_count: usize,
    pub total_count: usize,
    pub known_coverage_percent: u8,
    pub residual_unknown_percent: u8,
    pub max_residual_unknown_percent: u8,
    pub acceptable: bool,
    pub blockers: Vec<String>,
}

/// Pure evaluation of residual unknowns against a budget.
///
/// Returns an evaluation proving whether the observed unknown ratio respects the budget.
pub fn evaluate_residual_unknowns(
    unknown_count: usize,
    total_count: usize,
    budget: &ResidualUnknownBudget,
) -> ResidualUnknownEvaluation {
    let known_count = total_count.saturating_sub(unknown_count);
    let known_coverage_percent = if total_count == 0 {
        100
    } else {
        ((known_count * 100) / total_count) as u8
    };
    let residual_unknown_percent = 100u8.saturating_sub(known_coverage_percent);

    let mut blockers = Vec::new();

    if residual_unknown_percent > budget.max_residual_unknown_percent {
        blockers.push(format!(
            "residual_unknown_percent ({residual_unknown_percent}%) exceeds budget ({max}%)",
            max = budget.max_residual_unknown_percent
        ));
    }

    if let Some(max_count) = budget.max_unknown_count {
        if unknown_count > max_count {
            blockers.push(format!(
                "unknown_count ({unknown_count}) exceeds hard ceiling ({max_count})"
            ));
        }
    }

    let acceptable = blockers.is_empty();

    ResidualUnknownEvaluation {
        schema: RESIDUAL_UNKNOWN_EVALUATION_SCHEMA.to_string(),
        unknown_count,
        total_count,
        known_coverage_percent,
        residual_unknown_percent,
        max_residual_unknown_percent: budget.max_residual_unknown_percent,
        acceptable,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_known_coverage_is_acceptable() {
        let budget = ResidualUnknownBudget::standard();
        let eval = evaluate_residual_unknowns(0, 10, &budget);
        assert_eq!(eval.known_coverage_percent, 100);
        assert_eq!(eval.residual_unknown_percent, 0);
        assert!(eval.acceptable);
        assert!(eval.blockers.is_empty());
    }

    #[test]
    fn partial_coverage_within_budget_is_acceptable() {
        let budget = ResidualUnknownBudget::standard(); // 40% max unknown
        let eval = evaluate_residual_unknowns(3, 10, &budget); // 30% unknown
        assert_eq!(eval.known_coverage_percent, 70);
        assert_eq!(eval.residual_unknown_percent, 30);
        assert!(eval.acceptable);
    }

    #[test]
    fn excessive_unknowns_breaches_budget() {
        let budget = ResidualUnknownBudget::standard(); // 40% max unknown
        let eval = evaluate_residual_unknowns(5, 10, &budget); // 50% unknown
        assert_eq!(eval.known_coverage_percent, 50);
        assert_eq!(eval.residual_unknown_percent, 50);
        assert!(!eval.acceptable);
        assert_eq!(eval.blockers.len(), 1);
    }

    #[test]
    fn hard_count_ceiling_enforced() {
        let budget = ResidualUnknownBudget::new(50, Some(2));
        let eval = evaluate_residual_unknowns(3, 10, &budget);
        assert!(!eval.acceptable);
        assert!(eval.blockers[0].contains("unknown_count (3) exceeds hard ceiling (2)"));
    }

    #[test]
    fn zero_total_items_is_quiescent() {
        let budget = ResidualUnknownBudget::standard();
        let eval = evaluate_residual_unknowns(0, 0, &budget);
        assert_eq!(eval.known_coverage_percent, 100);
        assert_eq!(eval.residual_unknown_percent, 0);
        assert!(eval.acceptable);
    }
}
