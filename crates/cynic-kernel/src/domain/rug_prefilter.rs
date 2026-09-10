//! Compatibility adapter for pre-filter gate — PRODUCT-DOMAIN QUARANTINE.
//!
//! Token pre-filter heuristics now live in `cynic-intel`.
//! Canonical owner: `cynic-intel::rug_prefilter` (law); this module keeps only
//! the kernel-local `TokenData` adaptation. Track via `cynic dev kernel-audit`
//! area `product_domain_residual`. Not low-layer runtime law.

use crate::domain::enrichment::TokenData;

pub use cynic_intel::rug_prefilter::PreFilterResult;

impl cynic_intel::rug_prefilter::RugPrefilterTokenView for TokenData {
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
        &self.lp_status
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

pub fn rug_prefilter(token: &TokenData) -> PreFilterResult {
    cynic_intel::rug_prefilter::rug_prefilter(token)
}
