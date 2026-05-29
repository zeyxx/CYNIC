//! Token-analysis post-processor: applies conditional logic AFTER InferenceDog scoring.
//! Moves asymmetric confidence caps, class-aware rules, signal inversions OUT of domain prompt
//! and INTO code where they can be tested, debugged, and optimized independently.

use crate::domain::dog::{AxiomScores, QScore, Stimulus};

/// Token classification based on on-chain metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenClass {
    /// Infrastructure/stablecoin: USDC, USDT, wSOL. Active authorities expected.
    Infrastructure,
    /// Governance/utility: JUP, RAY, JTO. May retain active mint for emissions.
    Governance,
    /// Growth/meme: BONK, WIF. Standard scoring — revoked authorities = trust.
    Growth,
    /// New/unclassified: pump.fun, <30d. Extreme skepticism prior.
    Unclassified,
}

/// Token metadata extracted from stimulus for post-processing.
#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub class: TokenClass,
    pub age_days: u32,
    pub is_pump_fun: bool,
    pub market_cap_usd: Option<f64>,
    pub supply_burned_pct: f64,
    pub longevity_k_score: f64,
    pub accumulator_ratio: f64,
    pub diamond_hands: f64,
    pub organic_growth: f64,
}

impl TokenMetadata {
    /// Infer token class from on-chain metrics in stimulus.
    /// Returns (class, age_days) tuple for post-processing.
    pub fn infer_from_stimulus(stimulus: &Stimulus) -> Self {
        // Parse stimulus context for metrics. Stimulus format:
        // [METRICS]\nmarket_cap: ...\nage_days: ...\n[BEHAVIORAL]\n...
        let context_lower = stimulus.context.as_deref().unwrap_or("").to_lowercase();

        let age_days = parse_metric(&context_lower, "age_days", "age", "token_age") as u32;
        let market_cap_usd = parse_metric_f64(&context_lower, "market_cap", "marketcap");
        let supply_burned_pct = parse_metric(&context_lower, "supply_burned", "burned", "burn_pct");
        let k_score = parse_metric(&context_lower, "longevity", "k_score", "k-score");
        let accumulator_ratio =
            parse_metric(&context_lower, "accumulator", "accumulators", "acc_ratio");
        let diamond_hands = parse_metric(&context_lower, "diamond_hands", "dh", "diamond_hands");
        let organic_growth = parse_metric(&context_lower, "organic_growth", "organic", "og");
        let is_pump_fun = context_lower.contains("pump.fun") || context_lower.contains("pumpfun");

        let class = classify_token(market_cap_usd, age_days, is_pump_fun);

        Self {
            class,
            age_days,
            is_pump_fun,
            market_cap_usd,
            supply_burned_pct,
            longevity_k_score: k_score,
            accumulator_ratio,
            diamond_hands,
            organic_growth,
        }
    }
}

/// Classify token based on on-chain metrics.
fn classify_token(market_cap_usd: Option<f64>, age_days: u32, is_pump_fun: bool) -> TokenClass {
    if is_pump_fun && age_days < 30 {
        return TokenClass::Unclassified;
    }

    // Infrastructure: market_cap >$500M, age >365d, high holder count (proxy: >100k)
    if let Some(cap) = market_cap_usd {
        if cap > 500_000_000.0 && age_days > 365 {
            return TokenClass::Infrastructure;
        }
    }

    // Governance: might have active mint but with documented purpose
    if age_days > 180 {
        // Heuristic: if aged but not mega-cap, likely governance or growth
        if market_cap_usd.map_or(false, |cap| cap < 500_000_000.0 && cap > 10_000_000.0) {
            return TokenClass::Governance;
        }
    }

    // Growth: standard token, any age, any cap
    if age_days > 30 {
        return TokenClass::Growth;
    }

    // Default: new, unclassified
    TokenClass::Unclassified
}

/// Apply asymmetric confidence caps based on token age.
fn apply_age_caps(scores: &mut AxiomScores, age_days: u32) {
    let cap = match age_days {
        0..=29 => 0.45,   // <30d: untested, cannot score high
        30..=364 => 0.55, // 30-365d: building track record
        _ => 0.618,       // >365d: established
    };

    if scores.fidelity > cap {
        scores.fidelity = cap;
    }
    if scores.phi > cap {
        scores.phi = cap;
    }
    if scores.verify > cap {
        scores.verify = cap;
    }
    if scores.culture > cap {
        scores.culture = cap;
    }
    if scores.burn > cap {
        scores.burn = cap;
    }
    if scores.sovereignty > cap {
        scores.sovereignty = cap;
    }
}

/// Invert signal values per empirical correlation data.
/// diamond_hands and accumulator_ratio correlate NEGATIVELY with outcomes.
fn invert_inverted_signals(metadata: &TokenMetadata, scores: &mut AxiomScores) {
    // diamond_hands: HIGH values = bag-holding FOMO, NOT conviction. INVERT.
    // If diamond_hands is high (>0.6), it's a WARNING not a positive signal.
    // Use as penalty to FIDELITY and SOVEREIGNTY.
    if metadata.diamond_hands > 0.6 {
        let penalty = (metadata.diamond_hands - 0.6) * 0.1;
        scores.fidelity = (scores.fidelity - penalty).max(0.05);
        scores.sovereignty = (scores.sovereignty - penalty).max(0.05);
    }

    // accumulator_ratio: HIGH values (>0.7) = aggressive buying = insider activity BEFORE dump.
    // Correlates with WORSE outcomes. Use as penalty to PHI and VERIFY.
    if metadata.accumulator_ratio > 0.7 {
        let penalty = (metadata.accumulator_ratio - 0.7) * 0.15;
        scores.phi = (scores.phi - penalty).max(0.05);
        scores.verify = (scores.verify - penalty).max(0.05);
    }
}

/// Apply class-aware interpretation adjustments.
fn apply_class_awareness(metadata: &TokenMetadata, scores: &mut AxiomScores) {
    match metadata.class {
        TokenClass::Infrastructure => {
            // Infrastructure: active authorities are EXPECTED and CORRECT.
            // Boost FIDELITY for properly centralized design (opposite of growth tokens).
            // If market cap >$500M and age >365d, boost FIDELITY slightly (trust via scale).
            if metadata
                .market_cap_usd
                .map_or(false, |cap| cap > 500_000_000.0)
            {
                scores.fidelity = (scores.fidelity + 0.05).min(0.618);
                scores.sovereignty = (scores.sovereignty + 0.05).min(0.618); // Regulated entity = sovereign through accountability
            }
        }
        TokenClass::Governance => {
            // Governance: active mint for scheduled emissions is OK. Don't penalize.
            // Boost CULTURE for established DAO/utility tokens.
            if metadata.age_days > 180 {
                scores.culture = (scores.culture + 0.03).min(0.618);
            }
        }
        TokenClass::Growth => {
            // Standard growth token: no adjustments needed. Scored normally.
            // Longevity is the strongest positive signal for this class.
            scores.fidelity = (scores.fidelity + (metadata.longevity_k_score * 0.1)).min(0.618);
        }
        TokenClass::Unclassified => {
            // pump.fun <30d: extreme skepticism. Apply aggressive penalty.
            if metadata.is_pump_fun {
                scores.fidelity = (scores.fidelity * 0.5).max(0.05);
                scores.phi = (scores.phi * 0.6).max(0.05);
                scores.culture = (scores.culture * 0.5).max(0.05);
            }
        }
    }
}

/// Apply empirical signal weighting (rho-based).
fn apply_signal_weighting(metadata: &TokenMetadata, scores: &mut AxiomScores) {
    // supply_burned_pct: rho=+0.672 (strongest positive)
    // Boost BURN axiom if significant supply burned.
    if metadata.supply_burned_pct > 0.5 {
        scores.burn = (scores.burn + 0.1).min(0.618);
    }

    // longevity_k_score: rho=+0.632 (strong positive)
    // Already applied in class_awareness for Growth tokens.
    // For Infrastructure, also use longevity as confirmation of track record.
    if metadata.class == TokenClass::Infrastructure && metadata.longevity_k_score > 0.6 {
        scores.verify = (scores.verify + 0.05).min(0.618);
    }

    // organic_growth: rho=~+0.3 (moderate positive)
    // Boost PHI if distribution is organic.
    if metadata.organic_growth > 0.6 {
        scores.phi = (scores.phi + 0.05).min(0.618);
    }

    // market_cap and liquidity: noise. Do NOT use for scoring (no adjustments).
}

/// Apply prior calibration based on token class.
fn apply_prior_calibration(metadata: &TokenMetadata, q_score: &mut QScore) {
    // pump.fun <30d: 98.6% fail prior. Can only HOWL if strong evidence.
    // If Unclassified, suppress overall score unless clear evidence to override.
    if metadata.class == TokenClass::Unclassified && metadata.is_pump_fun {
        let max_total = 0.382; // φ⁻² - maximum for unproven token
        q_score.total = q_score.total.min(max_total);
    }

    // Established infrastructure (cap >$500M, age >365d): neutral prior.
    // No suppression. Score reflects current health, not existence.
    if metadata.class == TokenClass::Infrastructure {
        // No prior adjustment. Trust the score.
    }

    // Everything else: mild skepticism already embedded in classifier.
    // No additional adjustment needed.
}

/// Post-process token verdict: apply all conditional logic AFTER InferenceDog scoring.
pub fn postprocess_token_verdict(
    stimulus: &Stimulus,
    mut scores: AxiomScores,
) -> (AxiomScores, QScore) {
    let metadata = TokenMetadata::infer_from_stimulus(stimulus);

    // Step 1: Apply asymmetric confidence caps (age-based)
    apply_age_caps(&mut scores, metadata.age_days);

    // Step 2: Invert empirically backwards signals
    invert_inverted_signals(&metadata, &mut scores);

    // Step 3: Apply class-aware interpretation
    apply_class_awareness(&metadata, &mut scores);

    // Step 4: Weight signals by empirical correlation
    apply_signal_weighting(&metadata, &mut scores);

    // Step 5: Compute Q-score and apply prior calibration
    let mut q_score = QScore {
        total: (scores.fidelity
            + scores.phi
            + scores.verify
            + scores.culture
            + scores.burn
            + scores.sovereignty)
            / 6.0,
        fidelity: scores.fidelity,
        phi: scores.phi,
        verify: scores.verify,
        culture: scores.culture,
        burn: scores.burn,
        sovereignty: scores.sovereignty,
    };

    apply_prior_calibration(&metadata, &mut q_score);

    (scores, q_score)
}

/// Parse a metric from stimulus context (tries multiple common field names).
fn parse_metric(context: &str, keys: &str, alt_key1: &str, alt_key2: &str) -> f64 {
    // Try primary key
    if let Some(val) = extract_number(context, keys) {
        return val;
    }
    // Try alternative keys
    if let Some(val) = extract_number(context, alt_key1) {
        return val;
    }
    if let Some(val) = extract_number(context, alt_key2) {
        return val;
    }
    0.0 // Default if not found
}

/// Overload for extracting f64 (for market_cap_usd).
fn parse_metric_f64(context: &str, key1: &str, key2: &str) -> Option<f64> {
    if let Some(val) = extract_number(context, key1) {
        return Some(val);
    }
    if let Some(val) = extract_number(context, key2) {
        return Some(val);
    }
    None
}

/// Extract number after a key in stimulus context.
/// Looks for "key: N" or "key=N" format.
fn extract_number(context: &str, key: &str) -> Option<f64> {
    let key_lower = key.to_lowercase();
    for line in context.lines() {
        let line_lower = line.to_lowercase();
        if line_lower.contains(&key_lower) {
            // Try to extract number: "key: 123", "key=456", "key=0.123"
            if let Some(after_colon) = line.split(':').nth(1) {
                if let Ok(num) = after_colon
                    .trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .parse()
                {
                    return Some(num);
                }
            }
            if let Some(after_eq) = line.split('=').nth(1) {
                if let Ok(num) = after_eq
                    .trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .parse()
                {
                    return Some(num);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_infrastructure() {
        let class = classify_token(Some(600_000_000.0), 400, false);
        assert_eq!(class, TokenClass::Infrastructure);
    }

    #[test]
    fn classify_unclassified_pump_fun() {
        let class = classify_token(Some(1_000_000.0), 10, true);
        assert_eq!(class, TokenClass::Unclassified);
    }

    #[test]
    fn age_caps_under_30_days() {
        let mut scores = AxiomScores {
            fidelity: 0.618,
            phi: 0.618,
            verify: 0.618,
            culture: 0.618,
            burn: 0.618,
            sovereignty: 0.618,
            ..Default::default()
        };
        apply_age_caps(&mut scores, 20);
        assert!(scores.fidelity <= 0.45);
        assert!(scores.phi <= 0.45);
    }

    #[test]
    fn age_caps_over_365_days() {
        let mut scores = AxiomScores {
            fidelity: 0.618,
            phi: 0.618,
            verify: 0.618,
            culture: 0.618,
            burn: 0.618,
            sovereignty: 0.618,
            ..Default::default()
        };
        apply_age_caps(&mut scores, 500);
        // No caps applied for >365d
        assert_eq!(scores.fidelity, 0.618);
    }

    #[test]
    fn invert_diamond_hands_penalty() {
        let metadata = TokenMetadata {
            class: TokenClass::Growth,
            age_days: 100,
            is_pump_fun: false,
            market_cap_usd: Some(50_000_000.0),
            supply_burned_pct: 0.0,
            longevity_k_score: 0.5,
            accumulator_ratio: 0.4,
            diamond_hands: 0.8, // High = FOMO = penalty
            organic_growth: 0.5,
        };

        let mut scores = AxiomScores {
            fidelity: 0.618,
            phi: 0.618,
            verify: 0.618,
            culture: 0.618,
            burn: 0.618,
            sovereignty: 0.618,
            ..Default::default()
        };

        invert_inverted_signals(&metadata, &mut scores);
        assert!(scores.fidelity < 0.618); // Should be penalized
        assert!(scores.sovereignty < 0.618);
    }

    #[test]
    fn prior_calibration_pump_fun_unclassified() {
        let metadata = TokenMetadata {
            class: TokenClass::Unclassified,
            age_days: 10,
            is_pump_fun: true,
            market_cap_usd: Some(500_000.0),
            supply_burned_pct: 0.0,
            longevity_k_score: 0.0,
            accumulator_ratio: 0.5,
            diamond_hands: 0.5,
            organic_growth: 0.0,
        };

        let mut q_score = QScore {
            total: 0.618,
            fidelity: 0.618,
            phi: 0.618,
            verify: 0.618,
            culture: 0.618,
            burn: 0.618,
            sovereignty: 0.618,
        };

        apply_prior_calibration(&metadata, &mut q_score);
        assert!(q_score.total <= 0.382); // Suppressed to φ⁻²
    }
}
