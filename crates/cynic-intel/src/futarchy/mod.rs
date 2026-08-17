//! Pure, in-memory conditional-market simulation primitives.
//!
//! This module estimates a market signal. It has no observer, settlement
//! adapter, or authority to declare an outcome; callers must retain outcome
//! evidence separately before acting on any estimate.

pub mod cpmm;
pub mod types;
pub mod vault;

pub use cpmm::{calculate_prices_bps, create_market, execute_bet, resolve_market};
pub use types::{
    ConditionalMarket, FUTARCHY_MARKET_SCHEMA_V1, MarketDecision, MarketResolution, PredictionBet,
    ProposalStatus,
};
pub use vault::{LIQUIDITY_VAULT_SCHEMA_V1, LiquidityVault};
