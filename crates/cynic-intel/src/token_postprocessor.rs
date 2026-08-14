#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenClass {
    Infrastructure,
    Governance,
    Growth,
    Unclassified,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct AxiomScoreVector {
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenQScore {
    pub total: f64,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
}

pub fn infer_token_metadata_from_context(context: Option<&str>) -> TokenMetadata {
    let context_lower = context.unwrap_or("").to_lowercase();

    let age_days = parse_metric(&context_lower, "age_days", "age", "token_age") as u32;
    let market_cap_usd = parse_metric_f64(&context_lower, "market_cap", "marketcap");
    let supply_burned_pct = parse_metric(&context_lower, "supply_burned", "burned", "burn_pct");
    let longevity_k_score = parse_metric(&context_lower, "longevity", "k_score", "k-score");
    let accumulator_ratio =
        parse_metric(&context_lower, "accumulator", "accumulators", "acc_ratio");
    let diamond_hands = parse_metric(&context_lower, "diamond_hands", "dh", "diamond_hands");
    let organic_growth = parse_metric(&context_lower, "organic_growth", "organic", "og");
    let is_pump_fun = context_lower.contains("pump.fun") || context_lower.contains("pumpfun");

    let class = classify_token(market_cap_usd, age_days, is_pump_fun);

    TokenMetadata {
        class,
        age_days,
        is_pump_fun,
        market_cap_usd,
        supply_burned_pct,
        longevity_k_score,
        accumulator_ratio,
        diamond_hands,
        organic_growth,
    }
}

pub fn classify_token(market_cap_usd: Option<f64>, age_days: u32, is_pump_fun: bool) -> TokenClass {
    if is_pump_fun && age_days < 30 {
        return TokenClass::Unclassified;
    }

    if market_cap_usd.is_some_and(|cap| cap > 500_000_000.0) && age_days > 365 {
        return TokenClass::Infrastructure;
    }

    if age_days > 180 && market_cap_usd.is_some_and(|cap| cap < 500_000_000.0 && cap > 10_000_000.0)
    {
        return TokenClass::Governance;
    }

    if age_days > 30 {
        return TokenClass::Growth;
    }

    TokenClass::Unclassified
}

pub fn postprocess_token_scores(
    metadata: &TokenMetadata,
    mut scores: AxiomScoreVector,
) -> (AxiomScoreVector, TokenQScore) {
    apply_age_caps(&mut scores, metadata.age_days);
    invert_inverted_signals(metadata, &mut scores);
    apply_class_awareness(metadata, &mut scores);
    apply_signal_weighting(metadata, &mut scores);

    let mut q_score = TokenQScore {
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
    apply_prior_calibration(metadata, &mut q_score);

    (scores, q_score)
}

fn apply_age_caps(scores: &mut AxiomScoreVector, age_days: u32) {
    let cap = match age_days {
        0..=29 => 0.45,
        30..=364 => 0.55,
        _ => 0.618,
    };

    scores.fidelity = scores.fidelity.min(cap);
    scores.phi = scores.phi.min(cap);
    scores.verify = scores.verify.min(cap);
    scores.culture = scores.culture.min(cap);
    scores.burn = scores.burn.min(cap);
    scores.sovereignty = scores.sovereignty.min(cap);
}

fn invert_inverted_signals(metadata: &TokenMetadata, scores: &mut AxiomScoreVector) {
    if metadata.diamond_hands > 0.6 {
        let penalty = (metadata.diamond_hands - 0.6) * 0.1;
        scores.fidelity = (scores.fidelity - penalty).max(0.05);
        scores.sovereignty = (scores.sovereignty - penalty).max(0.05);
    }

    if metadata.accumulator_ratio > 0.7 {
        let penalty = (metadata.accumulator_ratio - 0.7) * 0.15;
        scores.phi = (scores.phi - penalty).max(0.05);
        scores.verify = (scores.verify - penalty).max(0.05);
    }
}

fn apply_class_awareness(metadata: &TokenMetadata, scores: &mut AxiomScoreVector) {
    match metadata.class {
        TokenClass::Infrastructure => {
            if metadata
                .market_cap_usd
                .is_some_and(|cap| cap > 500_000_000.0)
            {
                scores.fidelity = (scores.fidelity + 0.05).min(0.618);
                scores.sovereignty = (scores.sovereignty + 0.05).min(0.618);
            }
        }
        TokenClass::Governance => {
            if metadata.age_days > 180 {
                scores.culture = (scores.culture + 0.03).min(0.618);
            }
        }
        TokenClass::Growth => {
            scores.fidelity = (scores.fidelity + (metadata.longevity_k_score * 0.1)).min(0.618);
        }
        TokenClass::Unclassified => {
            if metadata.is_pump_fun {
                scores.fidelity = (scores.fidelity * 0.5).max(0.05);
                scores.phi = (scores.phi * 0.6).max(0.05);
                scores.culture = (scores.culture * 0.5).max(0.05);
            }
        }
    }
}

fn apply_signal_weighting(metadata: &TokenMetadata, scores: &mut AxiomScoreVector) {
    if metadata.supply_burned_pct > 0.5 {
        scores.burn = (scores.burn + 0.1).min(0.618);
    }

    if metadata.class == TokenClass::Infrastructure && metadata.longevity_k_score > 0.6 {
        scores.verify = (scores.verify + 0.05).min(0.618);
    }

    if metadata.organic_growth > 0.6 {
        scores.phi = (scores.phi + 0.05).min(0.618);
    }
}

fn apply_prior_calibration(metadata: &TokenMetadata, q_score: &mut TokenQScore) {
    if metadata.class == TokenClass::Unclassified && metadata.is_pump_fun {
        q_score.total = q_score.total.min(0.382);
    }
}

fn parse_metric(context: &str, keys: &str, alt_key1: &str, alt_key2: &str) -> f64 {
    extract_number(context, keys)
        .or_else(|| extract_number(context, alt_key1))
        .or_else(|| extract_number(context, alt_key2))
        .unwrap_or(0.0)
}

fn parse_metric_f64(context: &str, key1: &str, key2: &str) -> Option<f64> {
    extract_number(context, key1).or_else(|| extract_number(context, key2))
}

fn extract_number(context: &str, key: &str) -> Option<f64> {
    let key_lower = key.to_lowercase();
    for line in context.lines() {
        let line_lower = line.to_lowercase();
        if line_lower.contains(&key_lower) {
            let colon_result = line
                .split(':')
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.parse().ok());
            if colon_result.is_some() {
                return colon_result;
            }
            let eq_result = line
                .split('=')
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.parse().ok());
            if eq_result.is_some() {
                return eq_result;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn max_scores() -> AxiomScoreVector {
        AxiomScoreVector {
            fidelity: 0.618,
            phi: 0.618,
            verify: 0.618,
            culture: 0.618,
            burn: 0.618,
            sovereignty: 0.618,
        }
    }

    #[test]
    fn classifies_infrastructure() {
        assert_eq!(
            classify_token(Some(600_000_000.0), 400, false),
            TokenClass::Infrastructure
        );
    }

    #[test]
    fn classifies_young_pump_fun_as_unclassified() {
        assert_eq!(
            classify_token(Some(1_000_000.0), 10, true),
            TokenClass::Unclassified
        );
    }

    #[test]
    fn caps_young_token_scores() {
        let metadata = TokenMetadata {
            class: TokenClass::Growth,
            age_days: 20,
            is_pump_fun: false,
            market_cap_usd: None,
            supply_burned_pct: 0.0,
            longevity_k_score: 0.0,
            accumulator_ratio: 0.0,
            diamond_hands: 0.0,
            organic_growth: 0.0,
        };

        let (scores, _) = postprocess_token_scores(&metadata, max_scores());

        assert!(scores.fidelity <= 0.45);
        assert!(scores.phi <= 0.45);
    }

    #[test]
    fn suppresses_unclassified_pump_fun_prior() {
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

        let (_, q_score) = postprocess_token_scores(&metadata, max_scores());

        assert!(q_score.total <= 0.382);
    }

    #[test]
    fn infers_metadata_from_context() {
        let metadata = infer_token_metadata_from_context(Some(
            "age_days: 12\nmarket_cap: 1000000\norigin=pump.fun\ndiamond_hands: 0.8",
        ));

        assert_eq!(metadata.age_days, 12);
        assert!(metadata.is_pump_fun);
        assert_eq!(metadata.class, TokenClass::Unclassified);
        assert_eq!(metadata.diamond_hands, 0.8);
    }
}
