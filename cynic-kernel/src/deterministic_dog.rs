//! DeterministicDog — rule-based axiom evaluator.
//! Proves mixed intelligence: not every Dog needs an LLM.
//! Uses heuristics to score stimuli. Fast, free, deterministic.

use crate::dog::*;
use async_trait::async_trait;

pub struct DeterministicDog;

#[async_trait]
impl Dog for DeterministicDog {
    fn id(&self) -> &str {
        "deterministic-dog"
    }

    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError> {
        let content = &stimulus.content;
        let len = content.len();

        // FIDELITY: penalize vague/short claims, reward specificity
        let fidelity = if len < 10 {
            0.2
        } else if content.contains("always") || content.contains("never") || content.contains("100%") {
            0.15 // Absolute claims are suspect
        } else if content.contains("probably") || content.contains("likely") || content.contains("approximately") {
            0.55 // Epistemic humility rewarded
        } else {
            0.35 // Neutral
        };

        // PHI: structural coherence — length, punctuation, structure
        let phi = if len > 500 {
            0.3 // Overly verbose
        } else if len > 50 && content.contains('.') {
            0.5 // Has structure
        } else if len < 20 {
            0.25 // Too terse for meaningful structure
        } else {
            0.4
        };

        // VERIFY: does it reference evidence or make falsifiable claims?
        let verify = if content.contains("because") || content.contains("evidence")
            || content.contains("data") || content.contains("according to")
        {
            0.5 // References evidence
        } else if content.contains("?") {
            0.45 // Questions are verifiable by nature
        } else if content.contains("I think") || content.contains("I believe") {
            0.3 // Opinions without evidence
        } else {
            0.35
        };

        Ok(AxiomScores {
            fidelity,
            phi,
            verify,
            reasoning: AxiomReasoning {
                fidelity: format!("Heuristic: len={}, absolutes={}",
                    len, content.contains("always") || content.contains("never")),
                phi: format!("Heuristic: len={}, structured={}", len, content.contains('.')),
                verify: format!("Heuristic: evidence_words={}",
                    content.contains("because") || content.contains("evidence")),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn penalizes_absolute_claims() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "This will always work 100% of the time".into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(scores.fidelity < 0.2);
    }

    #[tokio::test]
    async fn rewards_epistemic_humility() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "This will probably work in most cases, approximately 60% of the time".into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(scores.fidelity > 0.5);
    }

    #[tokio::test]
    async fn rewards_evidence_references() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "According to the data, this approach works because of X".into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(scores.verify >= 0.5);
    }
}
