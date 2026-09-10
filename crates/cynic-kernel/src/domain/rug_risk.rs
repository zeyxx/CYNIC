//! Compatibility facade for rug-risk token intelligence — PRODUCT-DOMAIN QUARANTINE.
//!
//! Heuristic ownership lives in `cynic-intel`; the kernel only adapts its
//! current `TokenData` record until pure data ownership moves to `cynic-data`.
//! Canonical owner: `cynic-intel::rug_risk` (law). Track via `cynic dev kernel-audit`
//! area `product_domain_residual`. Not low-layer runtime law.

use crate::domain::enrichment::TokenData;
use cynic_intel::rug_risk::RugRiskTokenView;

pub use cynic_intel::rug_risk::{RugRiskAssessment, RugSignal, format_rug_risk_section};

impl RugRiskTokenView for TokenData {
    fn holder_data_available(&self) -> bool {
        self.holder_data_available
    }

    fn age_hours(&self) -> u64 {
        self.age_hours
    }

    fn holder_count(&self) -> u64 {
        self.holder_count
    }

    fn trajectory_class(&self) -> Option<&str> {
        self.trajectory_class.as_deref()
    }

    fn trajectory_decay(&self) -> Option<f64> {
        self.trajectory_decay
    }

    fn mint_authority_active(&self) -> bool {
        self.mint_authority_active
    }

    fn freeze_authority_active(&self) -> bool {
        self.freeze_authority_active
    }

    fn lp_status(&self) -> &str {
        &self.lp_status
    }

    fn supply_burned_pct(&self) -> Option<f64> {
        self.supply_burned_pct
    }

    fn origin(&self) -> Option<&str> {
        self.origin.as_deref()
    }

    fn top1_pct(&self) -> f64 {
        self.top1_pct
    }

    fn herfindahl(&self) -> Option<f64> {
        self.herfindahl
    }

    fn buy_sell_ratio(&self) -> Option<f64> {
        self.buy_sell_ratio
    }

    fn divergence_class(&self) -> Option<&str> {
        self.divergence_class.as_deref()
    }

    fn kscore_diamond_hands(&self) -> Option<f64> {
        self.kscore.as_ref().map(|kscore| kscore.diamond_hands)
    }

    fn volume_24h_usd(&self) -> Option<f64> {
        self.volume_24h_usd
    }

    fn liquidity_usd(&self) -> Option<f64> {
        self.liquidity_usd
    }
}

pub fn assess_rug_risk(token: &TokenData) -> RugRiskAssessment {
    cynic_intel::rug_risk::assess_rug_risk(token)
}
