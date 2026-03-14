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
        let context = stimulus.context.as_deref().unwrap_or("");
        let all_text = format!("{} {}", content, context);
        let len = content.len();
        let words: Vec<&str> = all_text.split_whitespace().collect();
        let word_count = words.len();

        // Structural signals
        let has_absolutes = words.iter().any(|w| matches!(w.to_lowercase().as_str(),
            "always" | "never" | "impossible" | "guaranteed" | "100%" | "certainly" | "undeniable"));
        let has_hedging = words.iter().any(|w| matches!(w.to_lowercase().as_str(),
            "probably" | "likely" | "approximately" | "perhaps" | "might" | "suggests" | "tends"));
        let has_evidence = words.iter().any(|w| matches!(w.to_lowercase().as_str(),
            "because" | "evidence" | "data" | "study" | "research" | "analysis" | "measured" | "tested" | "verified" | "proven" | "statistically"));
        let has_coercion = words.iter().any(|w| matches!(w.to_lowercase().as_str(),
            "must" | "mandatory" | "forced" | "required" | "compulsory" | "obey"));
        let has_agency = words.iter().any(|w| matches!(w.to_lowercase().as_str(),
            "choose" | "option" | "alternative" | "freedom" | "decide" | "prefer" | "consider"));
        let has_tradition = words.iter().any(|w| matches!(w.to_lowercase().as_str(),
            "tradition" | "convention" | "standard" | "established" | "classical" | "recognized" | "foundational" | "historical"));
        let has_disruption = words.iter().any(|w| matches!(w.to_lowercase().as_str(),
            "revolutionary" | "disrupt" | "radical" | "unprecedented" | "reject"));
        let sentence_count = content.matches('.').count() + content.matches('!').count() + content.matches('?').count();
        let has_numbers = content.chars().any(|c| c.is_ascii_digit());
        let has_notation = content.contains("...") || content.contains("->") || content.contains("=>")
            || content.chars().any(|c| "♔♕♖♗♘♙♚♛♜♝♞♟".contains(c));
        // Chess-specific structural detection (domain-aware but not domain-hardcoded)
        let has_algebraic = words.iter().any(|w| {
            let w = w.trim_matches(|c: char| !c.is_alphanumeric());
            w.len() >= 2 && w.len() <= 6
                && w.chars().next().map(|c| "abcdefghKQRBNO".contains(c)).unwrap_or(false)
                && w.chars().any(|c| c.is_ascii_digit())
        });
        let unique_ratio = {
            let mut unique: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();
            unique.sort(); unique.dedup();
            if word_count > 0 { unique.len() as f64 / word_count as f64 } else { 0.0 }
        };

        // FIDELITY: specificity, humility, evidence of grounding
        let mut fidelity: f64 = 0.35;
        if has_absolutes { fidelity -= 0.15; }
        if has_hedging { fidelity += 0.15; }
        if has_evidence { fidelity += 0.10; }
        if has_numbers || has_algebraic { fidelity += 0.05; }
        if len < 10 { fidelity = 0.15; }
        let fidelity: f64 = fidelity.clamp(0.05, 0.60);

        // PHI: structural harmony — sentence structure, vocabulary richness, proportion
        let mut phi: f64 = 0.35;
        if sentence_count >= 2 && word_count > 10 { phi += 0.10; }
        if unique_ratio > 0.7 { phi += 0.05; } else if unique_ratio < 0.4 { phi -= 0.10; }
        if len > 500 { phi -= 0.10; }
        if len < 15 { phi -= 0.10; }
        if has_notation || has_algebraic { phi += 0.05; }
        let phi: f64 = phi.clamp(0.05, 0.60);

        // VERIFY: falsifiability, evidence references, testability
        let mut verify: f64 = 0.30;
        if has_evidence { verify += 0.15; }
        if has_numbers { verify += 0.05; }
        if has_algebraic { verify += 0.10; } // Algebraic notation = verifiable on a board
        if content.contains('?') { verify += 0.05; }
        if has_absolutes && !has_evidence { verify -= 0.10; }
        let verify: f64 = verify.clamp(0.05, 0.60);

        // CULTURE: tradition, lineage, established patterns
        let mut culture: f64 = 0.35;
        if has_tradition { culture += 0.15; }
        if has_disruption { culture -= 0.10; }
        if has_algebraic { culture += 0.05; } // Chess notation = cultural artifact
        let culture: f64 = culture.clamp(0.05, 0.60);

        // BURN: efficiency — information density, conciseness
        let info_density = if word_count > 0 { (has_numbers as u8 + has_evidence as u8 + has_algebraic as u8) as f64 / 3.0 } else { 0.0 };
        let mut burn: f64 = 0.35;
        if len < 80 && sentence_count >= 1 { burn += 0.10; }
        if len > 300 { burn -= 0.15; }
        if info_density > 0.5 { burn += 0.10; }
        if unique_ratio < 0.4 { burn -= 0.10; } // Repetitive = wasteful
        let burn: f64 = burn.clamp(0.05, 0.60);

        // SOVEREIGNTY: agency preservation
        let mut sovereignty: f64 = 0.40;
        if has_coercion { sovereignty -= 0.20; }
        if has_agency { sovereignty += 0.10; }
        let sovereignty: f64 = sovereignty.clamp(0.05, 0.60);

        Ok(AxiomScores {
            fidelity,
            phi,
            verify,
            culture,
            burn,
            sovereignty,
            reasoning: AxiomReasoning {
                fidelity: format!("Structural: words={}, absolutes={}, hedging={}, evidence={}, specifics={}",
                    word_count, has_absolutes, has_hedging, has_evidence, has_numbers || has_algebraic),
                phi: format!("Structural: sentences={}, unique_ratio={:.2}, notation={}, len={}",
                    sentence_count, unique_ratio, has_notation || has_algebraic, len),
                verify: format!("Structural: evidence={}, numbers={}, algebraic={}, falsifiable={}",
                    has_evidence, has_numbers, has_algebraic, !has_absolutes || has_evidence),
                culture: format!("Structural: tradition_refs={}, disruption={}, notation={}",
                    has_tradition, has_disruption, has_algebraic),
                burn: format!("Structural: len={}, density={:.2}, unique_ratio={:.2}",
                    len, info_density, unique_ratio),
                sovereignty: format!("Structural: coercive={}, agency={}",
                    has_coercion, has_agency),
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
        assert!(scores.fidelity < 0.3, "absolutes should score low fidelity, got {}", scores.fidelity);
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
        assert!(scores.verify >= 0.4, "evidence should boost verify, got {}", scores.verify);
    }
}
