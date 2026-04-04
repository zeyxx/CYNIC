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

// ── Scoring constants (empirically calibrated) ──────────────
// These values produce scores in [0.05, φ⁻¹] that distribute verdicts
// across the BARK→HOWL range for typical text. No linguistic theory
// justifies specific values — engineering choices validated by tests.

/// PHI base score — center of scoring range.
const PHI_BASE: f64 = 0.30;
/// BURN base score — slightly above center (benefit of the doubt on efficiency).
const BURN_BASE: f64 = 0.35;
/// SOVEREIGNTY base score — default assumes moderate agency.
const SOVEREIGNTY_BASE: f64 = 0.40;

/// Small signal adjustment (single structural indicator).
const ADJUST_SMALL: f64 = 0.05;
/// Medium signal adjustment (strong structural indicator).
const ADJUST_MEDIUM: f64 = 0.10;
/// Large penalty (severe structural deficiency).
const ADJUST_LARGE: f64 = 0.15;
/// Vocabulary diversity proportion scaling.
const DIVERSITY_SCALE: f64 = 0.20;

/// Golden zone for text length (chars).
const LEN_GOLDEN_MIN: usize = 50;
const LEN_GOLDEN_MAX: usize = 300;
/// Verbose threshold (chars).
const LEN_VERBOSE: usize = 500;
/// Minimal threshold — too short to judge structure (chars).
const LEN_MINIMAL: usize = 15;
/// Concise threshold for BURN bonus (chars).
const LEN_CONCISE: usize = 150;

/// Vocabulary diversity threshold — below this = repetitive.
const DIVERSITY_LOW: f64 = 0.4;
/// Vocabulary diversity midpoint.
const DIVERSITY_MID: f64 = 0.5;

/// Minimum sentence count for structure bonus.
const SENTENCE_MIN_FOR_STRUCTURE: usize = 2;
/// Minimum word count for structure bonus.
const WORD_MIN_FOR_STRUCTURE: usize = 10;
/// Minimum word count for concise-structure bonus.
const WORD_MIN_FOR_CONCISE: usize = 5;
/// Maximum sentence count for concise-structure bonus.
const SENTENCE_MAX_FOR_CONCISE: usize = 5;

/// Coercion density threshold — below this, isolated occurrences are noise.
/// ~1 coercive term per 33 words.
#[allow(dead_code)] // WHY: Used in Fix 6, applied in later task.
const COERCION_DENSITY_THRESHOLD: f64 = 0.03;
/// Agency density threshold — symmetric with coercion.
#[allow(dead_code)] // WHY: Used in Fix 6, applied in later task.
const AGENCY_DENSITY_THRESHOLD: f64 = 0.03;

/// FIDELITY red-flag score when many absolutes detected.
/// Distinct from DIVERSITY_SCALE (same value, different meaning).
#[allow(dead_code)] // WHY: Used in Fix 2, applied in later task.
const FIDELITY_RED_FLAG: f64 = 0.20;

#[derive(Debug)]
pub struct DeterministicDog;

// ── Formal notation detection (extensible registry) ────────
/// Domain-specific formal notation detector.
/// Returns count of recognized formal tokens in the word list.
type NotationDetector = fn(words: &[&str]) -> usize;

/// Registry of formal notation patterns by domain.
/// New domain = add one entry + one detection function. No scoring logic changes.
/// Spotted by Pocks: "counts one domain, crashes on two."
static FORMAL_NOTATIONS: &[(&str, NotationDetector)] = &[("chess", detect_algebraic_notation)];

fn detect_formal_notation(domain: Option<&str>, words: &[&str]) -> usize {
    let domain = domain.unwrap_or("");
    FORMAL_NOTATIONS
        .iter()
        .find(|(d, _)| d.eq_ignore_ascii_case(domain))
        .map(|(_, detect)| detect(words))
        .unwrap_or(0)
}

/// Abbreviation set for sentence boundary detection.
/// Ref: Mikheev (2002) simplified — static list, ~97% accuracy.
const ABBREVIATIONS: &[&str] = &[
    // Titles
    "dr", "mr", "mrs", "ms", "prof", "sr", "jr", "rev", "gen", "sgt", "lt", "col",
    // Latin/academic
    "etc", "eg", "ie", "al", "vs", "viz", "cf", "ca", "approx", "est", // Months
    "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    // Organizations
    "inc", "corp", "ltd", "co", "dept", "assn", "govt", // Measurements
    "fig", "vol", "no", "pg", "pp", // Places
    "st", "ave", "blvd", "rd",
];

/// Count sentence boundaries using a simplified Mikheev cascade.
/// Handles abbreviations, decimals, ellipsis, and numbered lists.
fn count_sentences(text: &str) -> usize {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.is_empty() {
        return 1;
    }

    let mut count = 0;

    for (i, token) in tokens.iter().enumerate() {
        let last_char = match token.chars().last() {
            Some(c) if c == '.' || c == '!' || c == '?' => c,
            _ => continue,
        };

        // `!` and `?` are always boundaries
        if last_char != '.' {
            count += 1;
            continue;
        }

        // Ellipsis: "..." or ".."
        if token.ends_with("...") || token.ends_with("..") {
            continue;
        }

        // Decimal: token stripped of trailing period is a number (e.g. "3.14." → "3.14")
        // A token like "3.14." ends a sentence — the trailing period is the boundary.
        // We only skip if the token is a pure decimal WITHOUT a sentence-terminal period,
        // i.e. when the last dot is the decimal point itself (token like "3.14" alone).
        // In whitespace-tokenized text "3.14" mid-sentence never ends in '.' so this
        // branch fires only for sentence-terminating tokens — which ARE boundaries.
        let stripped_period = token.trim_end_matches('.');

        // Numbered list: "1." "2." etc. followed by non-uppercase token
        if !stripped_period.is_empty() && stripped_period.chars().all(|c| c.is_ascii_digit()) {
            let next_starts_upper = tokens
                .get(i + 1)
                .and_then(|t| t.chars().next())
                .is_some_and(|c| c.is_uppercase());
            if !next_starts_upper {
                continue;
            }
            // Ambiguous (digit-period before uppercase) — count as boundary
        }

        // Abbreviation: strip trailing dots, strip internal dots, lowercase, check set
        let normalized: String = stripped_period.replace('.', "").to_lowercase();
        if !normalized.is_empty() && ABBREVIATIONS.contains(&normalized.as_str()) {
            continue;
        }

        // Default: sentence boundary
        count += 1;
    }

    count.max(1)
}

fn detect_algebraic_notation(words: &[&str]) -> usize {
    words
        .iter()
        .filter(|w| {
            let w = w.trim_matches(|c: char| !c.is_alphanumeric());
            w.len() >= 2
                && w.len() <= 6
                && w.chars()
                    .next()
                    .is_some_and(|c| "abcdefghKQRBNO".contains(c))
                && w.chars().any(|c| c.is_ascii_digit())
        })
        .count()
}

#[async_trait]
impl Dog for DeterministicDog {
    fn id(&self) -> &str {
        "deterministic-dog"
    }

    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError> {
        let content = &stimulus.content;
        // Context intentionally unused — DeterministicDog judges FORM of content only.
        // LLM Dogs use context for SUBSTANCE evaluation. See Fix 1 / F11.
        let len = content.chars().count();
        let words: Vec<&str> = content.split_whitespace().collect();
        let word_count = words.len();

        // ── Signal detection ────────────────────────────────────
        // Strip punctuation so "obey." matches "obey", "required," matches "required"
        let lower_words: Vec<String> = words
            .iter()
            .map(|w| {
                w.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase()
            })
            .collect();

        // F10 fix: match percentage patterns (e.g. "100%") on raw words BEFORE
        // trim_matches strips the %. lower_words has already lost the '%'.
        let pct_absolutes = words
            .iter()
            .filter(|w| {
                let w = w.to_lowercase();
                w.ends_with('%')
                    && w.trim_end_matches('%')
                        .parse::<f64>()
                        .is_ok_and(|n| n >= 100.0 || n <= 0.0)
            })
            .count();
        let absolutes_count = lower_words
            .iter()
            .filter(|w| {
                matches!(
                    w.as_str(),
                    "always" | "never" | "impossible" | "guaranteed" | "certainly" | "undeniable"
                )
            })
            .count()
            + pct_absolutes;
        let hedging_count = lower_words
            .iter()
            .filter(|w| {
                matches!(
                    w.as_str(),
                    "probably"
                        | "likely"
                        | "approximately"
                        | "perhaps"
                        | "might"
                        | "suggests"
                        | "tends"
                )
            })
            .count();
        let coercion_count = lower_words
            .iter()
            .filter(|w| {
                matches!(
                    w.as_str(),
                    "must" | "mandatory" | "forced" | "required" | "compulsory" | "obey"
                )
            })
            .count();
        let agency_count = lower_words
            .iter()
            .filter(|w| {
                matches!(
                    w.as_str(),
                    "choose"
                        | "option"
                        | "alternative"
                        | "freedom"
                        | "decide"
                        | "prefer"
                        | "consider"
                )
            })
            .count();

        let sentence_count = count_sentences(content);
        let has_numbers = content.chars().any(|c| c.is_ascii_digit());
        let has_notation = content.contains("...")
            || content.contains("->")
            || content.contains("=>")
            || content.chars().any(|c| "♔♕♖♗♘♙♚♛♜♝♞♟".contains(c));
        // F9 fix: algebraic notation detection only for registered domains.
        // Without domain guard, Rust identifiers (f64, u8, Rc4) trigger false VERIFY boost.
        // Extensible via FORMAL_NOTATIONS registry — new domain = one entry + one fn.
        let formal_notation_count = detect_formal_notation(stimulus.domain.as_deref(), &words);

        let unique_ratio = {
            let mut content_lower: Vec<String> = words
                .iter()
                .map(|w| {
                    w.trim_matches(|c: char| !c.is_alphanumeric())
                        .to_lowercase()
                })
                .collect();
            content_lower.sort();
            content_lower.dedup();
            if word_count > 0 {
                content_lower.len() as f64 / word_count as f64
            } else {
                0.0
            }
        };

        // Assertion density: absolute claims + formal notation = assertive content.
        // Hedging excluded — epistemic caution is neutral for efficiency, not signal.
        // Ref: Hyland (1998), hedges outnumber boosters 3:1 in academic prose.
        let assertion_density = if word_count > 0 {
            (absolutes_count + formal_notation_count) as f64 / word_count as f64
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
            format!("Red flag: {absolutes_count} absolute claims, no hedging.")
        } else {
            "Abstaining — fidelity requires semantic understanding.".into()
        };

        // ── PHI: FORM judge — structure, proportion, harmony ────
        let mut phi: f64 = PHI_BASE;
        // Sentence structure
        if sentence_count >= SENTENCE_MIN_FOR_STRUCTURE && word_count > WORD_MIN_FOR_STRUCTURE {
            phi += ADJUST_MEDIUM;
        }
        if (1..=SENTENCE_MAX_FOR_CONCISE).contains(&sentence_count)
            && word_count > WORD_MIN_FOR_CONCISE
        {
            phi += ADJUST_SMALL;
        }
        // Vocabulary richness (gradient, not threshold)
        phi += (unique_ratio - DIVERSITY_MID) * DIVERSITY_SCALE; // [-0.10, +0.10] range
        // Length proportion (golden zone: 50-300 chars)
        if (LEN_GOLDEN_MIN..=LEN_GOLDEN_MAX).contains(&len) {
            phi += ADJUST_SMALL;
        } else if len > LEN_VERBOSE {
            phi -= ADJUST_MEDIUM;
        } else if len < LEN_MINIMAL {
            phi -= ADJUST_LARGE;
        }
        // Formal notation = precise structure
        if has_notation || formal_notation_count > 0 {
            phi += ADJUST_SMALL;
        }
        let phi = phi.clamp(ADJUST_SMALL, PHI_INV);
        let phi_reason = format!(
            "Structure: {} sentences, {:.0}% vocab diversity, {} chars.",
            sentence_count,
            unique_ratio * 100.0,
            len
        );

        // ── VERIFY: NEUTRAL — requires domain knowledge ────────
        // Only boost if algebraic notation (objectively verifiable on a board).
        let verify = if formal_notation_count >= 2 {
            (NEUTRAL + 0.10).min(PHI_INV)
        } else {
            NEUTRAL
        };
        let verify_reason = if formal_notation_count >= 2 {
            format!(
                "Found {formal_notation_count} algebraic notation tokens — verifiable on board."
            )
        } else {
            "Abstaining — verification requires domain knowledge.".into()
        };

        // ── CULTURE: NEUTRAL — requires domain knowledge ───────
        let culture = NEUTRAL;
        let culture_reason = "Abstaining — cultural assessment requires domain knowledge.".into();

        // ── BURN: FORM judge — efficiency, density, conciseness ─
        let mut burn: f64 = BURN_BASE;
        // Information density (proportional, not boolean)
        if assertion_density > 0.15 {
            burn += ADJUST_MEDIUM;
        } else if assertion_density > 0.05 {
            burn += ADJUST_SMALL;
        }
        // Conciseness (under 150 chars + at least one sentence = efficient)
        if len < LEN_CONCISE && sentence_count >= 1 {
            burn += ADJUST_MEDIUM;
        } else if len > LEN_GOLDEN_MAX {
            burn -= ADJUST_MEDIUM;
        }
        if len > LEN_VERBOSE {
            burn -= ADJUST_SMALL;
        } // additional penalty
        // Repetition penalty (gradient)
        if unique_ratio < DIVERSITY_LOW {
            burn -= ADJUST_LARGE;
        } else if unique_ratio < DIVERSITY_MID {
            burn -= ADJUST_SMALL;
        }
        // Numbers/algebraic = information-dense
        if has_numbers && formal_notation_count > 0 {
            burn += ADJUST_SMALL;
        }
        let burn = burn.clamp(ADJUST_SMALL, PHI_INV);
        let burn_reason = if len < 100 && assertion_density > 0.10 {
            "Concise and information-dense.".into()
        } else if len > 300 {
            format!("Verbose ({len} chars) — potential excess.")
        } else if unique_ratio < 0.4 {
            format!(
                "Repetitive ({:.0}% unique) — wasteful.",
                unique_ratio * 100.0
            )
        } else {
            format!(
                "Moderate efficiency: {} chars, {:.1}% assertion density.",
                len,
                assertion_density * 100.0
            )
        };

        // ── SOVEREIGNTY: FORM judge — coercion vs agency ───────
        let mut sovereignty: f64 = SOVEREIGNTY_BASE;
        // Proportional coercion penalty (not just boolean)
        if coercion_count > 0 {
            sovereignty -= ADJUST_MEDIUM * (coercion_count as f64).min(3.0);
        }
        if agency_count > 0 {
            sovereignty += ADJUST_SMALL * (agency_count as f64).min(3.0);
        }
        let sovereignty = sovereignty.clamp(ADJUST_SMALL, PHI_INV);
        let sovereignty_reason = if coercion_count > 0 {
            format!("{coercion_count} coercive term(s) detected — limits agency.")
        } else if agency_count > 0 {
            format!("{agency_count} agency signal(s) — preserves choice.")
        } else {
            "Neutral — no strong agency signals.".into()
        };

        // Track which axioms this Dog abstained on (returned NEUTRAL).
        // Abstention ≠ disagreement — excluded from spread calculation in judge.rs.
        let mut abstentions = Vec::new();
        if (fidelity - NEUTRAL).abs() < 0.001 {
            abstentions.push("fidelity".into());
        }
        if (verify - NEUTRAL).abs() < 0.001 {
            abstentions.push("verify".into());
        }
        if (culture - NEUTRAL).abs() < 0.001 {
            abstentions.push("culture".into());
        }

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
        assert!(
            scores.fidelity < NEUTRAL,
            "many absolutes should flag fidelity, got {}",
            scores.fidelity
        );
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
        assert!(
            (scores.culture - NEUTRAL).abs() < 0.01,
            "culture should be neutral, got {}",
            scores.culture
        );
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
        assert!(
            scores.phi > 0.40,
            "good structure should boost phi, got {}",
            scores.phi
        );
        assert!(
            scores.burn > 0.35,
            "concise structured text should score well on burn, got {}",
            scores.burn
        );
    }

    #[tokio::test]
    async fn penalizes_coercion() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "You must obey. This is mandatory and required. Compliance is compulsory."
                .into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(
            scores.sovereignty < 0.20,
            "coercion should tank sovereignty, got {}",
            scores.sovereignty
        );
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
        assert!(
            scores.sovereignty > 0.45,
            "agency should boost sovereignty, got {}",
            scores.sovereignty
        );
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
        assert!(
            scores.verify > NEUTRAL,
            "algebraic notation should boost verify, got {}",
            scores.verify
        );
    }

    #[tokio::test]
    async fn formal_notation_unknown_domain_returns_zero() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6".into(),
            context: None,
            domain: Some("trading".into()),
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(
            (scores.verify - NEUTRAL).abs() < 0.01,
            "non-chess domain should not detect algebraic notation, verify={}",
            scores.verify
        );
    }

    #[tokio::test]
    async fn context_does_not_contaminate_scores() {
        let dog = DeterministicDog;
        let content = "The Sicilian Defense is a strong opening for black.";

        let without_context = Stimulus {
            content: content.into(),
            context: None,
            domain: Some("chess".into()),
        };
        let with_adversarial_context = Stimulus {
            content: content.into(),
            context: Some(
                "must always never guaranteed obey mandatory forced required compulsory".into(),
            ),
            domain: Some("chess".into()),
        };

        let scores_clean = dog.evaluate(&without_context).await.unwrap();
        let scores_dirty = dog.evaluate(&with_adversarial_context).await.unwrap();

        assert!(
            (scores_clean.sovereignty - scores_dirty.sovereignty).abs() < 0.001,
            "context must not affect sovereignty: clean={}, dirty={}",
            scores_clean.sovereignty,
            scores_dirty.sovereignty
        );
        assert!(
            (scores_clean.phi - scores_dirty.phi).abs() < 0.001,
            "context must not affect phi: clean={}, dirty={}",
            scores_clean.phi,
            scores_dirty.phi
        );
        assert!(
            (scores_clean.burn - scores_dirty.burn).abs() < 0.001,
            "context must not affect burn: clean={}, dirty={}",
            scores_clean.burn,
            scores_dirty.burn
        );
    }

    // ── Sentence Boundary Detection tests ──────────────────

    #[test]
    fn sbd_abbreviations() {
        assert_eq!(
            count_sentences("Dr. Smith analyzed 3.14 results. It worked."),
            2
        );
    }

    #[test]
    fn sbd_no_punctuation() {
        assert_eq!(count_sentences("Hello world"), 1);
    }

    #[test]
    fn sbd_multiple_terminators() {
        assert_eq!(count_sentences("Really? Yes! OK."), 3);
    }

    #[test]
    fn sbd_latin_abbreviations() {
        assert_eq!(count_sentences("I studied e.g. physics. Then math."), 2);
    }

    #[test]
    fn sbd_ellipsis() {
        assert_eq!(count_sentences("Wait... really?"), 1);
    }

    #[test]
    fn sbd_chess_notation() {
        assert_eq!(count_sentences("1. e4 e5 2. Nf3 Nc6 3. Bb5"), 2);
    }

    #[test]
    fn sbd_empty() {
        assert_eq!(count_sentences(""), 1);
    }

    #[test]
    fn sbd_decimal_numbers() {
        assert_eq!(count_sentences("The value is 3.14. The other is 2.718."), 2);
    }

    #[tokio::test]
    async fn hedging_does_not_inflate_burn() {
        let dog = DeterministicDog;
        let hedged = Stimulus {
            content:
                "This probably likely perhaps approximately tends to suggest something might work."
                    .into(),
            context: None,
            domain: None,
        };
        let neutral = Stimulus {
            content: "This thing does something that works in a normal way overall.".into(),
            context: None,
            domain: None,
        };
        let scores_hedged = dog.evaluate(&hedged).await.unwrap();
        let scores_neutral = dog.evaluate(&neutral).await.unwrap();

        assert!(
            scores_hedged.burn <= scores_neutral.burn + ADJUST_SMALL,
            "hedging should not inflate burn: hedged={}, neutral={}",
            scores_hedged.burn,
            scores_neutral.burn
        );
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
                point. But instead it just keeps going and going and going."
                .into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(
            scores.burn < 0.25,
            "verbose repetitive text should score low on burn, got {}",
            scores.burn
        );
    }
}
