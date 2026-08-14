//! Rug pre-filter for token hygiene before dog dispatch.
//!
//! This is intentionally opinionated, deterministic logic for a local gate.
//! The kernel adapts its local `TokenData` into the trait and keeps this module
//! as a compatibility facade.

#[derive(Debug, Clone, PartialEq)]
pub enum PreFilterResult {
    Pass,
    Rug(String),
    Inconclusive,
}

pub trait RugPrefilterTokenView {
    fn holder_count(&self) -> u64;
    fn liquidity_usd(&self) -> Option<f64>;
    fn mint_authority_active(&self) -> bool;
    fn age_hours(&self) -> u64;
    fn top1_pct(&self) -> f64;
    fn lp_status(&self) -> &str;
    fn supply_burned_pct(&self) -> Option<f64>;
    fn trajectory_class(&self) -> Option<&str>;
    fn volume_24h_usd(&self) -> Option<f64>;
}

/// Run rug pre-filter against enriched token data.
///
/// Hard-fail signals (any one => Rug):
/// - holder_count == 0
/// - liquidity_usd < 100
/// - mint authority active and token age > 168h with small holder base
/// - top1_pct > 95 with < 10 holders
///
/// Soft signals:
/// - supply_burned_pct == 0 and lp_status != burned and age > 48h
/// - trajectory_class == DEAD
/// - volume_24h_usd == 0 and age > 72h
///
/// Fail-open policy:
/// 1-2 soft signals => Inconclusive (not veto), 3 soft signals => Rug.
pub fn rug_prefilter(token: &impl RugPrefilterTokenView) -> PreFilterResult {
    if token.holder_count() == 0 {
        return PreFilterResult::Rug("no holders — dead token".into());
    }

    if token.liquidity_usd().is_some_and(|l| l < 100.0) {
        return PreFilterResult::Rug(format!(
            "liquidity ${:.0} < $100",
            token.liquidity_usd().unwrap_or(0.0)
        ));
    }

    if token.mint_authority_active() && token.age_hours() > 168 && token.holder_count() < 100_000 {
        return PreFilterResult::Rug(format!(
            "mint authority active on {}-hour-old token ({} holders)",
            token.age_hours(),
            token.holder_count()
        ));
    }

    if token.top1_pct() > 95.0 && token.holder_count() < 10 {
        return PreFilterResult::Rug(format!(
            "top holder owns {:.0}% with {} holders",
            token.top1_pct(),
            token.holder_count()
        ));
    }

    let mut score: u32 = 0;

    if token.supply_burned_pct().is_none_or(|p| p == 0.0)
        && token.lp_status() != "burned"
        && token.age_hours() > 48
    {
        score += 1;
    }

    if token.trajectory_class() == Some("DEAD") {
        score += 1;
    }

    if token.volume_24h_usd() == Some(0.0) && token.age_hours() > 72 {
        score += 1;
    }

    match score {
        0 => PreFilterResult::Pass,
        1 | 2 => PreFilterResult::Inconclusive,
        _ => PreFilterResult::Rug(
            "soft-signal score=3: no-commitment + dead-trajectory + no-trading".into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct FakeTokenData {
        holder_count: u64,
        top1_pct: f64,
        age_hours: u64,
        mint_authority_active: bool,
        lp_status: String,
        liquidity_usd: Option<f64>,
        volume_24h_usd: Option<f64>,
        supply_burned_pct: Option<f64>,
        trajectory_class: Option<String>,
    }

    impl RugPrefilterTokenView for FakeTokenData {
        fn holder_count(&self) -> u64 {
            self.holder_count
        }

        fn liquidity_usd(&self) -> Option<f64> {
            self.liquidity_usd
        }

        fn mint_authority_active(&self) -> bool {
            self.mint_authority_active
        }

        fn age_hours(&self) -> u64 {
            self.age_hours
        }

        fn top1_pct(&self) -> f64 {
            self.top1_pct
        }

        fn lp_status(&self) -> &str {
            self.lp_status.as_str()
        }

        fn supply_burned_pct(&self) -> Option<f64> {
            self.supply_burned_pct
        }

        fn trajectory_class(&self) -> Option<&str> {
            self.trajectory_class.as_deref()
        }

        fn volume_24h_usd(&self) -> Option<f64> {
            self.volume_24h_usd
        }
    }

    fn healthy() -> FakeTokenData {
        FakeTokenData {
            holder_count: 500,
            top1_pct: 20.0,
            age_hours: 100,
            mint_authority_active: false,
            lp_status: "burned".into(),
            liquidity_usd: Some(50_000.0),
            volume_24h_usd: Some(10_000.0),
            supply_burned_pct: Some(50.0),
            trajectory_class: None,
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
        t.holder_count = 500;
        assert!(matches!(rug_prefilter(&t), PreFilterResult::Rug(_)));
    }

    #[test]
    fn young_token_active_mint_not_hard_fail() {
        let mut t = healthy();
        t.mint_authority_active = true;
        t.age_hours = 24;
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
        assert_eq!(rug_prefilter(&t), PreFilterResult::Inconclusive);
    }
}
