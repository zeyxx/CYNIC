use serde::{Deserialize, Serialize};

/// Task taxonomy — derived from CYNIC commit analysis (2434 commits).
///
/// Frequencies from real data:
/// - Solo tasks: 68.9%
/// - Chain tasks: 26.2% (sequential handoff)
/// - Fanout tasks: 4.9% (parallel dispatch)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskCategory {
    /// Single agent, single scope (file or crate).
    Solo,
    /// Sequential handoff: parent → child → grandchild.
    Chain,
    /// Parallel dispatch: 1 parent → N workers → 1 merge.
    Fanout,
}

/// Difficulty tier — determines model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    /// Routine: free inference suffices.
    T0,
    /// Standard: sovereign inference (7B local).
    T1,
    /// Complex: powerful provider (20B-70B).
    T2,
    /// Critical: multi-prism review required.
    T3,
}

/// Agent model tier — maps to hardware budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelTier {
    /// Free inference (meituan/longcat-2.0, etc.).
    Free,
    /// Sovereign local (7B coordinator).
    Sovereign,
    /// Powerful provider (20B-70B).
    Powerful,
}

impl Tier {
    /// Recommended model tier for this task tier.
    pub fn model_tier(&self) -> ModelTier {
        match self {
            Tier::T0 => ModelTier::Free,
            Tier::T1 => ModelTier::Sovereign,
            Tier::T2 => ModelTier::Powerful,
            Tier::T3 => ModelTier::Powerful,
        }
    }

    /// Wall-clock budget in seconds.
    pub fn wall_seconds(&self) -> u32 {
        match self {
            Tier::T0 => 60,
            Tier::T1 => 300,
            Tier::T2 => 900,
            Tier::T3 => 1800,
        }
    }

    /// Action budget (number of agent turns).
    pub fn actions(&self) -> u32 {
        match self {
            Tier::T0 => 5,
            Tier::T1 => 10,
            Tier::T2 => 25,
            Tier::T3 => 50,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_t0_uses_free_model() {
        assert_eq!(Tier::T0.model_tier(), ModelTier::Free);
    }

    #[test]
    fn tier_t3_has_largest_budget() {
        assert!(Tier::T3.actions() > Tier::T0.actions());
        assert!(Tier::T3.wall_seconds() > Tier::T0.wall_seconds());
    }
}
