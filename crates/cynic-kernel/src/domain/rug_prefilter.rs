//! Rug pre-filter — synchronous BURN gate before Dog dispatch.
//! NOT a safety gate — fail-open: Inconclusive → Dogs judge.
//! K15: consumes TokenData, changes system behavior (gates Dog dispatch).

use crate::domain::enrichment::TokenData;

#[derive(Debug, Clone, PartialEq)]
pub enum PreFilterResult {
    Pass,
    Rug(String),
    Inconclusive,
}

/// Run rug pre-filter against enriched token data.
///
/// Hard-fail signals (any one → Rug):
///   - holder_count == 0 OR liquidity_usd < $100
///   - mint_authority_active AND age_hours > 168 AND holder_count < 100_000
///     (stablecoins/protocol tokens keep mint authority intentionally)
///   - top1_pct > 95 AND holder_count < 10
///
/// Soft signals (score >= 3 → Rug, 1-2 → Inconclusive):
///   - supply_burned_pct == 0 AND lp_status != "burned" AND age_hours > 48
///   - trajectory_class == Some("DEAD")
///   - volume_24h_usd == 0 AND age_hours > 72
pub fn rug_prefilter(token: &TokenData) -> PreFilterResult {
    // Hard-fail: dead market
    if token.holder_count == 0 {
        return PreFilterResult::Rug("no holders — dead token".into());
    }
    if token.liquidity_usd.is_some_and(|l| l < 100.0) {
        return PreFilterResult::Rug(format!(
            "liquidity ${:.0} < $100",
            token.liquidity_usd.unwrap_or(0.0)
        ));
    }

    // Hard-fail: honeypot — but only for small tokens.
    // Stablecoins (USDC, USDT) and protocol tokens keep mint authority
    // intentionally. 100K+ holders = established, not a honeypot.
    if token.mint_authority_active && token.age_hours > 168 && token.holder_count < 100_000 {
        return PreFilterResult::Rug(format!(
            "mint authority active on {}-hour-old token ({} holders)",
            token.age_hours, token.holder_count
        ));
    }

    // Hard-fail: zero distribution
    if token.top1_pct > 95.0 && token.holder_count < 10 {
        return PreFilterResult::Rug(format!(
            "top holder owns {:.0}% with {} holders",
            token.top1_pct, token.holder_count
        ));
    }

    // Soft signals
    let mut score: u32 = 0;

    if token.supply_burned_pct.is_none_or(|p| p == 0.0)
        && token.lp_status != "burned"
        && token.age_hours > 48
    {
        score += 1;
    }

    if token.trajectory_class.as_deref() == Some("DEAD") {
        score += 1;
    }

    if (token.volume_24h_usd == Some(0.0)) && token.age_hours > 72 {
        score += 1;
    }

    if score >= 3 {
        return PreFilterResult::Rug(format!(
            "soft-signal score={score}: no-commitment + dead-trajectory + no-trading"
        ));
    }

    if score > 0 {
        return PreFilterResult::Inconclusive;
    }

    PreFilterResult::Pass
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy() -> TokenData {
        TokenData {
            mint: "TestMintAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into(),
            holder_count: 500,
            top1_pct: 20.0,
            top10_pct: 60.0,
            age_hours: 100,
            mint_authority_active: false,
            lp_status: "burned".into(),
            liquidity_usd: Some(50_000.0),
            volume_24h_usd: Some(10_000.0),
            supply_burned_pct: Some(50.0),
            ..TokenData::default()
        }
    }

    #[test]
    fn pass_on_healthy_token() {
        assert_eq!(rug_prefilter(&healthy()), PreFilterResult::Pass);
    }

    #[test]
    fn rug_on_zero_holders() {
        let mut t = healthy();
        t.holder_count = 0;
        assert!(matches!(rug_prefilter(&t), PreFilterResult::Rug(_)));
    }

    #[test]
    fn rug_on_low_liquidity() {
        let mut t = healthy();
        t.liquidity_usd = Some(50.0);
        assert!(matches!(rug_prefilter(&t), PreFilterResult::Rug(_)));
    }

    #[test]
    fn rug_on_active_mint_old_small_token() {
        let mut t = healthy();
        t.mint_authority_active = true;
        t.age_hours = 200;
        t.holder_count = 500; // small token — honeypot signal
        assert!(matches!(rug_prefilter(&t), PreFilterResult::Rug(_)));
    }

    #[test]
    fn young_token_active_mint_not_hard_fail() {
        let mut t = healthy();
        t.mint_authority_active = true;
        t.age_hours = 24;
        // 24h old — NOT week-old honeypot, should not produce the hard-fail Rug
        assert_ne!(
            rug_prefilter(&t),
            PreFilterResult::Rug("mint authority active on 24-hour-old token".into())
        );
    }

    #[test]
    fn rug_on_zero_distribution() {
        let mut t = healthy();
        t.top1_pct = 98.0;
        t.holder_count = 3;
        assert!(matches!(rug_prefilter(&t), PreFilterResult::Rug(_)));
    }

    #[test]
    fn all_soft_signals_produce_rug() {
        let mut t = healthy();
        t.lp_status = "unsecured".into();
        t.supply_burned_pct = Some(0.0);
        t.age_hours = 100;
        t.trajectory_class = Some("DEAD".into());
        t.volume_24h_usd = Some(0.0);
        assert!(matches!(rug_prefilter(&t), PreFilterResult::Rug(_)));
    }

    #[test]
    fn trajectory_none_does_not_trigger_soft() {
        let mut t = healthy();
        t.trajectory_class = None;
        t.lp_status = "unsecured".into();
        t.supply_burned_pct = Some(0.0);
        t.age_hours = 100;
        t.volume_24h_usd = Some(1000.0);
        // Only 1 soft signal (no-commitment) → Inconclusive
        assert_eq!(rug_prefilter(&t), PreFilterResult::Inconclusive);
    }

    #[test]
    fn stablecoin_active_mint_not_hard_fail() {
        // USDC/USDT keep mint authority intentionally — 1M holders should bypass
        let mut t = healthy();
        t.mint_authority_active = true;
        t.age_hours = 20_000; // years old
        t.holder_count = 1_000_000;
        t.liquidity_usd = Some(50_000_000.0);
        assert_ne!(
            std::mem::discriminant(&rug_prefilter(&t)),
            std::mem::discriminant(&PreFilterResult::Rug(String::new()))
        );
    }

    #[test]
    fn small_token_active_mint_still_hard_fail() {
        // Small token with active mint authority = honeypot signal
        let mut t = healthy();
        t.mint_authority_active = true;
        t.age_hours = 200;
        t.holder_count = 50;
        assert!(matches!(rug_prefilter(&t), PreFilterResult::Rug(_)));
    }

    #[test]
    fn jup_token_passes() {
        let t = TokenData {
            mint: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN".into(),
            holder_count: 250_000,
            top1_pct: 12.5,
            top10_pct: 45.2,
            age_hours: 1200,
            mint_authority_active: false,
            lp_status: "burned".into(),
            liquidity_usd: Some(5_000_000.0),
            volume_24h_usd: Some(500_000.0),
            supply_burned_pct: Some(0.0),
            ..TokenData::default()
        };
        assert_eq!(rug_prefilter(&t), PreFilterResult::Pass);
    }
}
