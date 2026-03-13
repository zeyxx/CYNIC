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

        // CULTURE: does it honor existing patterns and continuity?
        let culture = if content.contains("tradition") || content.contains("convention")
            || content.contains("standard") || content.contains("established")
        {
            0.5 // References existing patterns
        } else if content.contains("revolutionary") || content.contains("disrupt") {
            0.25 // Breaks with continuity (not inherently bad, but low culture score)
        } else {
            0.4
        };

        // BURN: is this minimal and efficient? Destroy excess.
        let burn = if len < 50 && content.contains('.') {
            0.55 // Concise and complete
        } else if len > 300 {
            0.2 // Verbose — excess to burn
        } else if len > 100 {
            0.35
        } else {
            0.45
        };

        // SOVEREIGNTY: does this preserve individual agency?
        let sovereignty = if content.contains("must") || content.contains("mandatory")
            || content.contains("forced") || content.contains("required to")
        {
            0.2 // Coercive language — low sovereignty
        } else if content.contains("choose") || content.contains("option")
            || content.contains("alternative") || content.contains("freedom")
        {
            0.55 // Preserves agency
        } else {
            0.4
        };

        Ok(AxiomScores {
            fidelity,
            phi,
            verify,
            culture,
            burn,
            sovereignty,
            reasoning: AxiomReasoning {
                fidelity: format!("Heuristic: len={}, absolutes={}",
                    len, content.contains("always") || content.contains("never")),
                phi: format!("Heuristic: len={}, structured={}", len, content.contains('.')),
                verify: format!("Heuristic: evidence_words={}",
                    content.contains("because") || content.contains("evidence")),
                culture: format!("Heuristic: tradition_refs={}", content.contains("standard") || content.contains("convention")),
                burn: format!("Heuristic: len={}, concise={}", len, len < 50),
                sovereignty: format!("Heuristic: coercive={}", content.contains("must") || content.contains("forced")),
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
