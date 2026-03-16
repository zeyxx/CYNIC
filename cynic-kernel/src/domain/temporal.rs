//! MCTS Temporal — 7 temporal perspectives for multi-dimensional judgment.
//! Each perspective evaluates a stimulus through a different time lens.
//! PHI (1.618) serves as the exploration constant (UCB1).
//!
//! Novel: no existing system evaluates through temporal perspectives
//! with phi-bounded confidence and geometric mean aggregation.

use serde::{Deserialize, Serialize};
use crate::domain::dog::{PHI, AxiomScores, phi_bound};

/// The 7 temporal perspectives — irreducible set.
/// Each sees the stimulus through a different time lens.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TemporalPerspective {
    /// What happened before? Historical context and precedent.
    Past,
    /// What is happening now? Current state and immediate reality.
    Present,
    /// What will happen? Projected consequences and trajectories.
    Future,
    /// Has this happened before? Recurring patterns and rhythms.
    Cycle,
    /// Where is this heading? Direction and momentum over time.
    Trend,
    /// What new thing is appearing? Novel patterns not seen before.
    Emergence,
    /// What deeper truth does this reveal? Meta-pattern across all perspectives.
    Transcendence,
}

impl TemporalPerspective {
    /// All 7 perspectives in canonical order.
    pub const ALL: [TemporalPerspective; 7] = [
        Self::Past,
        Self::Present,
        Self::Future,
        Self::Cycle,
        Self::Trend,
        Self::Emergence,
        Self::Transcendence,
    ];

    /// Human-readable description for prompt construction.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Past => "Evaluate through PAST: What historical context and precedent inform this? What happened before in similar situations?",
            Self::Present => "Evaluate through PRESENT: What is the current state? What is immediately true right now?",
            Self::Future => "Evaluate through FUTURE: What are the likely consequences? Where does this lead?",
            Self::Cycle => "Evaluate through CYCLE: Has this pattern occurred before? Is this a recurring phenomenon?",
            Self::Trend => "Evaluate through TREND: What is the trajectory? Is this accelerating, decelerating, or stable?",
            Self::Emergence => "Evaluate through EMERGENCE: What new thing is appearing here that hasn't been seen before?",
            Self::Transcendence => "Evaluate through TRANSCENDENCE: What deeper truth does this reveal beyond the immediate facts?",
        }
    }

    /// Short label for display and storage.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Past => "PAST",
            Self::Present => "PRESENT",
            Self::Future => "FUTURE",
            Self::Cycle => "CYCLE",
            Self::Trend => "TREND",
            Self::Emergence => "EMERGENCE",
            Self::Transcendence => "TRANSCENDENCE",
        }
    }
}

// ── TEMPORAL EVALUATION RESULT ──────────────────────────────

/// A single perspective's evaluation of a stimulus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalScore {
    pub perspective: TemporalPerspective,
    pub axiom_scores: AxiomScores,
    pub q_total: f64,
}

/// Aggregated result across all evaluated temporal perspectives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalVerdict {
    /// Per-perspective scores
    pub perspectives: Vec<TemporalScore>,
    /// Geometric mean of all perspective Q-Scores
    pub temporal_total: f64,
    /// Which perspective diverges most from consensus (discovery signal)
    pub outlier_perspective: Option<TemporalPerspective>,
    /// Max divergence from temporal consensus
    pub max_divergence: f64,
}

// ── UCB1 WITH PHI ───────────────────────────────────────────
/// Upper Confidence Bound with PHI as exploration constant.
/// Used to select which temporal perspective to explore next
/// when not evaluating all 7 (resource-constrained mode).
pub fn ucb1_phi(mean_reward: f64, total_visits: u32, node_visits: u32) -> f64 {
    if node_visits == 0 {
        return f64::INFINITY; // Unexplored — always select
    }
    let exploitation = mean_reward;
    let exploration = PHI * ((total_visits as f64).ln() / node_visits as f64).sqrt();
    exploitation + exploration
}

// ── TEMPORAL AGGREGATION (pure domain logic) ─────────────────

/// Aggregate temporal perspective scores into a TemporalVerdict.
/// Uses geometric mean — one weak perspective drags the total down.
pub fn aggregate_temporal(scores: &[TemporalScore]) -> TemporalVerdict {
    if scores.is_empty() {
        return TemporalVerdict {
            perspectives: Vec::new(),
            temporal_total: 0.0,
            outlier_perspective: None,
            max_divergence: 0.0,
        };
    }

    // Geometric mean of Q-Score totals across perspectives
    let n = scores.len() as f64;
    let product: f64 = scores.iter().map(|s| s.q_total.max(1e-10)).product();
    let geo_mean = product.powf(1.0 / n);
    let temporal_total = phi_bound(geo_mean);

    // Find outlier perspective (max divergence from consensus)
    let mean_q: f64 = scores.iter().map(|s| s.q_total).sum::<f64>() / n;
    let (outlier, max_div) = scores.iter()
        .map(|s| (s.perspective, (s.q_total - mean_q).abs()))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((TemporalPerspective::Present, 0.0));

    TemporalVerdict {
        perspectives: scores.to_vec(),
        temporal_total,
        outlier_perspective: if max_div > crate::domain::dog::PHI_INV2 { Some(outlier) } else { None },
        max_divergence: max_div,
    }
}

// ── TESTS ───────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dog::{AxiomReasoning, compute_qscore, PHI_INV};

    fn make_temporal_score(perspective: TemporalPerspective, value: f64) -> TemporalScore {
        let scores = AxiomScores {
            fidelity: value, phi: value, verify: value,
            culture: value, burn: value, sovereignty: value,
            reasoning: AxiomReasoning::default(), ..Default::default()
        };
        let q = compute_qscore(&scores);
        TemporalScore { perspective, axiom_scores: scores, q_total: q.total }
    }

    #[test]
    fn seven_perspectives_exist() {
        assert_eq!(TemporalPerspective::ALL.len(), 7);
    }

    #[test]
    fn each_perspective_has_description() {
        for p in &TemporalPerspective::ALL {
            assert!(!p.description().is_empty());
            assert!(!p.label().is_empty());
        }
    }

    #[test]
    fn ucb1_unexplored_returns_infinity() {
        let ucb = ucb1_phi(0.5, 10, 0);
        assert!(ucb.is_infinite());
    }

    #[test]
    fn ucb1_exploration_uses_phi() {
        let ucb = ucb1_phi(0.5, 100, 10);
        // Exploitation = 0.5, exploration = PHI * sqrt(ln(100)/10)
        let expected_exploration = PHI * (100_f64.ln() / 10.0).sqrt();
        assert!((ucb - (0.5 + expected_exploration)).abs() < 1e-10);
    }

    #[test]
    fn aggregate_empty_returns_zero() {
        let v = aggregate_temporal(&[]);
        assert_eq!(v.temporal_total, 0.0);
        assert!(v.outlier_perspective.is_none());
    }

    #[test]
    fn aggregate_uniform_scores() {
        let scores: Vec<_> = TemporalPerspective::ALL.iter()
            .map(|&p| make_temporal_score(p, 0.5))
            .collect();
        let v = aggregate_temporal(&scores);
        assert!(v.temporal_total > 0.0);
        assert!(v.temporal_total <= PHI_INV + 1e-10);
        // Uniform scores → no outlier
        assert!(v.outlier_perspective.is_none());
    }

    #[test]
    fn outlier_detected_when_perspectives_disagree() {
        let mut scores: Vec<_> = TemporalPerspective::ALL.iter()
            .map(|&p| make_temporal_score(p, 0.5))
            .collect();
        // Make EMERGENCE score very differently
        scores[5] = make_temporal_score(TemporalPerspective::Emergence, 0.05);
        let v = aggregate_temporal(&scores);
        // The low score should drag temporal_total down
        assert!(v.temporal_total < make_temporal_score(TemporalPerspective::Past, 0.5).q_total);
    }

    #[test]
    fn temporal_total_phi_bounded() {
        let scores: Vec<_> = TemporalPerspective::ALL.iter()
            .map(|&p| make_temporal_score(p, 0.95))
            .collect();
        let v = aggregate_temporal(&scores);
        assert!(v.temporal_total <= PHI_INV + 1e-10);
    }

    #[test]
    fn one_weak_perspective_drags_total() {
        let strong: Vec<_> = TemporalPerspective::ALL.iter()
            .map(|&p| make_temporal_score(p, 0.6))
            .collect();
        let mut weak = strong.clone();
        weak[0] = make_temporal_score(TemporalPerspective::Past, 0.05);

        let v_strong = aggregate_temporal(&strong);
        let v_weak = aggregate_temporal(&weak);
        assert!(v_weak.temporal_total < v_strong.temporal_total * 0.8);
    }
}
