//! DeterministicDog — structural form evaluator.
//!
//! Role: FORM judge, not SUBSTANCE judge. Evaluates what heuristics CAN
//! evaluate reliably: structure (PHI), efficiency (BURN), coercion signals
//! (SOVEREIGNTY). Returns NEUTRAL scores (φ⁻¹/2 ≈ 0.309) for axioms that
//! require semantic understanding (FIDELITY, VERIFY, CULTURE) — these are
//! left to LLM Dogs. This prevents the deterministic dog from dragging the
//! consensus average on axioms it cannot meaningfully assess.
//!
//! Design principle: better to abstain than to guess.

use crate::domain::dog::*;
use async_trait::async_trait;

/// Neutral score — center of [0, φ⁻¹]. Neither boosts nor drags LLM consensus.
const NEUTRAL: f64 = PHI_INV / 2.0; // ≈ 0.309

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

        // ── Signal detection ────────────────────────────────────
        // Strip punctuation so "obey." matches "obey", "required," matches "required"
        let lower_words: Vec<String> = words.iter()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
            .collect();

        let absolutes_count = lower_words.iter().filter(|w| matches!(w.as_str(),
            "always" | "never" | "impossible" | "guaranteed" | "100%" | "certainly" | "undeniable")).count();
        let hedging_count = lower_words.iter().filter(|w| matches!(w.as_str(),
            "probably" | "likely" | "approximately" | "perhaps" | "might" | "suggests" | "tends")).count();
        let coercion_count = lower_words.iter().filter(|w| matches!(w.as_str(),
            "must" | "mandatory" | "forced" | "required" | "compulsory" | "obey")).count();
        let agency_count = lower_words.iter().filter(|w| matches!(w.as_str(),
            "choose" | "option" | "alternative" | "freedom" | "decide" | "prefer" | "consider")).count();

        let sentence_count = content.matches('.').count()
            + content.matches('!').count()
            + content.matches('?').count();
        let has_numbers = content.chars().any(|c| c.is_ascii_digit());
        let has_notation = content.contains("...") || content.contains("->") || content.contains("=>")
            || content.chars().any(|c| "♔♕♖♗♘♙♚♛♜♝♞♟".contains(c));
        let algebraic_count = words.iter().filter(|w| {
            let w = w.trim_matches(|c: char| !c.is_alphanumeric());
            w.len() >= 2 && w.len() <= 6
                && w.chars().next().is_some_and(|c| "abcdefghKQRBNO".contains(c))
                && w.chars().any(|c| c.is_ascii_digit())
        }).count();

        let unique_ratio = {
            let mut unique = lower_words.clone();
            unique.sort();
            unique.dedup();
            if word_count > 0 { unique.len() as f64 / word_count as f64 } else { 0.0 }
        };

        // Proportional signal density (count-based, not boolean)
        let signal_density = if word_count > 0 {
            (absolutes_count + hedging_count + algebraic_count) as f64 / word_count as f64
        } else {
            0.0
        };

        // ── FIDELITY: NEUTRAL — requires semantic understanding ─
        // Only flag extreme red flags (many absolutes), otherwise abstain.
        let fidelity = if absolutes_count >= 3 && hedging_count == 0 {
            0.20 // strong red flag even without semantics
        } else {
            NEUTRAL
        };
        let fidelity_reason = if absolutes_count >= 3 && hedging_count == 0 {
            format!("Red flag: {} absolute claims, no hedging.", absolutes_count)
        } else {
            "Abstaining — fidelity requires semantic understanding.".into()
        };

        // ── PHI: FORM judge — structure, proportion, harmony ────
        let mut phi: f64 = 0.30;
        // Sentence structure
        if sentence_count >= 2 && word_count > 10 { phi += 0.10; }
        if (1..=5).contains(&sentence_count) && word_count > 5 { phi += 0.05; }
        // Vocabulary richness (gradient, not threshold)
        phi += (unique_ratio - 0.5) * 0.20; // [-0.10, +0.10] range
        // Length proportion (golden zone: 50-300 chars)
        if (50..=300).contains(&len) { phi += 0.05; }
        else if len > 500 { phi -= 0.10; }
        else if len < 15 { phi -= 0.15; }
        // Formal notation = precise structure
        if has_notation || algebraic_count > 0 { phi += 0.05; }
        let phi = phi.clamp(0.05, PHI_INV);
        let phi_reason = format!(
            "Structure: {} sentences, {:.0}% vocab diversity, {} chars.",
            sentence_count, unique_ratio * 100.0, len
        );

        // ── VERIFY: NEUTRAL — requires domain knowledge ────────
        // Only boost if algebraic notation (objectively verifiable on a board).
        let verify = if algebraic_count >= 2 {
            (NEUTRAL + 0.10).min(PHI_INV)
        } else {
            NEUTRAL
        };
        let verify_reason = if algebraic_count >= 2 {
            format!("Found {} algebraic notation tokens — verifiable on board.", algebraic_count)
        } else {
            "Abstaining — verification requires domain knowledge.".into()
        };

        // ── CULTURE: NEUTRAL — requires domain knowledge ───────
        let culture = NEUTRAL;
        let culture_reason = "Abstaining — cultural assessment requires domain knowledge.".into();

        // ── BURN: FORM judge — efficiency, density, conciseness ─
        let mut burn: f64 = 0.35;
        // Information density (proportional, not boolean)
        if signal_density > 0.15 { burn += 0.10; }
        else if signal_density > 0.05 { burn += 0.05; }
        // Conciseness (under 150 chars + at least one sentence = efficient)
        if len < 150 && sentence_count >= 1 { burn += 0.10; }
        else if len > 300 { burn -= 0.10; }
        if len > 500 { burn -= 0.05; } // additional penalty
        // Repetition penalty (gradient)
        if unique_ratio < 0.4 { burn -= 0.15; }
        else if unique_ratio < 0.5 { burn -= 0.05; }
        // Numbers/algebraic = information-dense
        if has_numbers && algebraic_count > 0 { burn += 0.05; }
        let burn = burn.clamp(0.05, PHI_INV);
        let burn_reason = if len < 100 && signal_density > 0.10 {
            "Concise and information-dense.".into()
        } else if len > 300 {
            format!("Verbose ({} chars) — potential excess.", len)
        } else if unique_ratio < 0.4 {
            format!("Repetitive ({:.0}% unique) — wasteful.", unique_ratio * 100.0)
        } else {
            format!("Moderate efficiency: {} chars, {:.1}% signal density.", len, signal_density * 100.0)
        };

        // ── SOVEREIGNTY: FORM judge — coercion vs agency ───────
        let mut sovereignty: f64 = 0.40;
        // Proportional coercion penalty (not just boolean)
        if coercion_count > 0 {
            sovereignty -= 0.10 * (coercion_count as f64).min(3.0);
        }
        if agency_count > 0 {
            sovereignty += 0.05 * (agency_count as f64).min(3.0);
        }
        let sovereignty = sovereignty.clamp(0.05, PHI_INV);
        let sovereignty_reason = if coercion_count > 0 {
            format!("{} coercive term(s) detected — limits agency.", coercion_count)
        } else if agency_count > 0 {
            format!("{} agency signal(s) — preserves choice.", agency_count)
        } else {
            "Neutral — no strong agency signals.".into()
        };

        // Track which axioms this Dog abstained on (returned NEUTRAL).
        // Abstention ≠ disagreement — excluded from spread calculation in judge.rs.
        let mut abstentions = Vec::new();
        if (fidelity - NEUTRAL).abs() < 0.001 { abstentions.push("fidelity".into()); }
        if (verify - NEUTRAL).abs() < 0.001 { abstentions.push("verify".into()); }
        if (culture - NEUTRAL).abs() < 0.001 { abstentions.push("culture".into()); }

        Ok(AxiomScores {
            fidelity,
            phi,
            verify,
            culture,
            burn,
            sovereignty,
            prompt_tokens: 0,
            completion_tokens: 0,
            reasoning: AxiomReasoning {
                fidelity: fidelity_reason,
                phi: phi_reason,
                verify: verify_reason,
                culture: culture_reason,
                burn: burn_reason,
                sovereignty: sovereignty_reason,
            },
            abstentions,
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
            content: "This will always work and is never wrong, guaranteed 100% certainly".into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(scores.fidelity < NEUTRAL, "many absolutes should flag fidelity, got {}", scores.fidelity);
    }

    #[tokio::test]
    async fn neutral_on_substance_axioms() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "The Sicilian Defense is a strong opening for black.".into(),
            context: None,
            domain: Some("chess".into()),
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        // Culture should be neutral — deterministic dog can't judge chess culture
        assert!((scores.culture - NEUTRAL).abs() < 0.01,
            "culture should be neutral, got {}", scores.culture);
    }

    #[tokio::test]
    async fn rewards_structural_quality() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "First, we analyze the position. Then, we consider candidate moves. Finally, we evaluate consequences.".into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(scores.phi > 0.40, "good structure should boost phi, got {}", scores.phi);
        assert!(scores.burn > 0.35, "concise structured text should score well on burn, got {}", scores.burn);
    }

    #[tokio::test]
    async fn penalizes_coercion() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "You must obey. This is mandatory and required. Compliance is compulsory.".into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(scores.sovereignty < 0.20, "coercion should tank sovereignty, got {}", scores.sovereignty);
    }

    #[tokio::test]
    async fn rewards_agency() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "You could choose this option, or consider an alternative approach.".into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(scores.sovereignty > 0.45, "agency should boost sovereignty, got {}", scores.sovereignty);
    }

    #[tokio::test]
    async fn algebraic_notation_boosts_verify() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6".into(),
            context: None,
            domain: Some("chess".into()),
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(scores.verify > NEUTRAL, "algebraic notation should boost verify, got {}", scores.verify);
    }

    #[tokio::test]
    async fn verbose_text_penalized_on_burn() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "This is a very long and verbose piece of text that goes on and on and on \
                without really saying much of anything at all. It keeps repeating the same ideas \
                over and over again in slightly different words but never actually making a clear \
                point. The text continues to ramble on and on and on and on with no end in sight \
                and really should have been edited down to something much more concise and to the \
                point. But instead it just keeps going and going and going.".into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(scores.burn < 0.25, "verbose repetitive text should score low on burn, got {}", scores.burn);
    }
}
