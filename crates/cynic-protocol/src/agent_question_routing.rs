//! Operator-question routing (contracts/agent-policy.json `facts_dictate_routing`).
//!
//! Pure decision: given what the observed data determines and what authority
//! the citizen holds, route to act, observe, or ask one single question.
//! No I/O, no clock reads. Unknowns travel alongside the verdict and are
//! stated with the question; they never alter the route.

use serde::{Deserialize, Serialize};

pub const QUESTION_ROUTING_SCHEMA: &str = "cynic.agent_question_routing.v1";

/// Why closing the gap exceeds citizen authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceedsAuthority {
    ScopeExpansion,
    BudgetDecision,
    MutationBeyondMandate,
    ConstitutionalDoubt,
}

/// What the observed data determines about the next step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataVerdict {
    /// Observed data determines the next action.
    Dictated,
    /// Data insufficient, but the gap is closable within citizen authority.
    ClosableGap,
    /// Data insufficient and the choice exceeds citizen authority.
    Escalation(ExceedsAuthority),
}

/// The only three permitted outcomes. Total over [`DataVerdict`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionRoute {
    /// Act within authority, then report the fact-to-action chain.
    Act,
    /// Observe more, narrow the unknowns, then act. Still no question.
    Observe,
    /// Ask the operator a single question with the unknowns stated.
    SingleQuestion,
}

/// Route the next step from the data verdict.
///
/// Total function: every verdict maps to exactly one route, so no caller
/// can fall through to an unexamined dilemma.
pub fn decide_operator_question_permitted(verdict: DataVerdict) -> QuestionRoute {
    match verdict {
        DataVerdict::Dictated => QuestionRoute::Act,
        DataVerdict::ClosableGap => QuestionRoute::Observe,
        DataVerdict::Escalation(_) => QuestionRoute::SingleQuestion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictated_data_routes_to_act() {
        assert_eq!(
            decide_operator_question_permitted(DataVerdict::Dictated),
            QuestionRoute::Act
        );
    }

    #[test]
    fn closable_gap_routes_to_observe() {
        assert_eq!(
            decide_operator_question_permitted(DataVerdict::ClosableGap),
            QuestionRoute::Observe
        );
    }

    #[test]
    fn every_escalation_reason_routes_to_single_question() {
        for reason in [
            ExceedsAuthority::ScopeExpansion,
            ExceedsAuthority::BudgetDecision,
            ExceedsAuthority::MutationBeyondMandate,
            ExceedsAuthority::ConstitutionalDoubt,
        ] {
            assert_eq!(
                decide_operator_question_permitted(DataVerdict::Escalation(reason)),
                QuestionRoute::SingleQuestion,
            );
        }
    }
}
