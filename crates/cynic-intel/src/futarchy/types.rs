use serde::{Deserialize, Serialize};

pub const FUTARCHY_MARKET_SCHEMA_V1: &str = "cynic.futarchy_market.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketDecision {
    Pass,
    Fail,
    Undecided,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Open,
    Resolved,
    Executed,
    Canceled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredictionBet {
    pub bettor_wallet: String,
    pub is_yes: bool,
    pub amount_units: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketResolution {
    pub decision: MarketDecision,
    pub yes_price_bps: u32,
    pub no_price_bps: u32,
    pub payout_ratio_bps: u32,
    pub resolved_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalMarket {
    pub schema: &'static str,
    pub market_id: String,
    pub proposal_id: String,
    pub question: String,
    pub yes_pool_units: u64,
    pub no_pool_units: u64,
    pub total_liquidity_units: u64,
    pub min_conviction_threshold_bps: u32,
    pub status: ProposalStatus,
    pub resolution: Option<MarketResolution>,
}
