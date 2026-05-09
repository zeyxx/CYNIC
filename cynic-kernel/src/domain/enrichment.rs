//! TokenEnricher — port trait for on-chain token data enrichment.
//! When domain=token-analysis and content looks like a Solana address,
//! the pipeline enriches the stimulus before Dogs evaluate.
//! Pure domain — zero external dependencies.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Enriched token data from on-chain sources.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenData {
    /// Mint address
    pub mint: String,
    /// Token name (e.g. "Jupiter")
    pub name: Option<String>,
    /// Token symbol (e.g. "JUP")
    pub symbol: Option<String>,
    /// Total supply (raw, before decimals)
    pub supply: Option<u64>,
    /// Decimal places
    pub decimals: Option<u8>,
    /// Price per token in USD
    pub price_usd: Option<f64>,
    /// Number of unique token holders (may be lower bound if > 20 exist).
    pub holder_count: u64,
    /// True if holder_count is exact (< 20 accounts returned from RPC).
    /// False = lower bound (20 accounts returned, likely more exist).
    pub holder_count_is_exact: bool,
    /// Percentage held by the largest wallet
    pub top1_pct: f64,
    /// Type of largest holder: "lp_pool", "burn", "locker", "wallet", "unknown"
    pub top1_type: String,
    /// Percentage held by the top 10 wallets
    pub top10_pct: f64,
    /// Herfindahl-Hirschman Index for holder concentration (0.0 to 1.0)
    pub herfindahl: Option<f64>,
    /// Token age in hours
    pub age_hours: u64,
    /// True if age_hours is exact (creation tx found). False = lower bound estimate.
    pub age_is_exact: bool,
    /// Whether the mint authority is active
    pub mint_authority_active: bool,
    /// Whether the freeze authority is active
    pub freeze_authority_active: bool,
    /// Liquidity pool status: "burned", "locked", "unsecured"
    pub lp_status: String,
    /// Percentage of supply burned
    pub supply_burned_pct: Option<f64>,
    /// Percentage of supply locked
    pub supply_locked_pct: Option<f64>,
    /// Source of the token (e.g. "pump.fun", "raydium")
    pub origin: Option<String>,
    /// Token standard (Fungible, NonFungible, etc.)
    pub token_standard: Option<String>,
    /// Off-chain description from token metadata
    pub description: Option<String>,
    /// Creation date if available
    pub created_at: Option<String>,
    /// True if holder/concentration data was available from RPC.
    /// False = RPC degraded, holders/top1/top10/herfindahl are zero (not real).
    pub holder_data_available: bool,
    /// K-Score behavioral composite (None if behavioral analysis unavailable).
    pub kscore: Option<KScore>,
    /// Per-wallet behavioral breakdown (top-N holders).
    pub wallet_behaviors: Vec<WalletBehavior>,
}

/// Classification of a token holder based on buy/sell behavior.
/// Derived from retention_ratio = current_balance / total_bought.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HolderClass {
    /// Bought more since initial purchase (retention >= accumulator_threshold, default 1.5)
    Accumulator,
    /// Holding all or most (retention >= holder_threshold, default 1.0)
    Holder,
    /// Sold some (retention >= reducer_threshold, default 0.5)
    Reducer,
    /// Sold most or all (retention < reducer_threshold)
    Extractor,
}

impl std::fmt::Display for HolderClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accumulator => write!(f, "accumulator"),
            Self::Holder => write!(f, "holder"),
            Self::Reducer => write!(f, "reducer"),
            Self::Extractor => write!(f, "extractor"),
        }
    }
}

/// Per-wallet behavioral analysis from SWAP transaction history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBehavior {
    pub class: HolderClass,
    /// current_balance / total_bought (> 1.0 = bought more via DCA, < 1.0 = sold some)
    pub retention_ratio: f64,
    /// Number of SWAP transactions found for this wallet+token
    pub swap_count: u32,
}

/// K-Score composite — behavioral health metric for a token.
/// Formula: K = DiamondHands^w_dh × OrganicGrowth^w_og × Longevity^w_lon
/// Weights configurable via backends.toml [kscore].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KScore {
    /// Final composite score (0.0 to 1.0)
    pub score: f64,
    /// DiamondHands pillar: sqrt(conviction × retention_signal)
    /// conviction = (accumulators + holders) / analyzed
    /// retention_signal = tanh(accumulators / max(extractors,1) / 2)
    ///   tanh saturates to 1.0 when acc >> ext (strong diamond hands)
    ///   tanh → 0 when ext >> acc (everyone selling)
    pub diamond_hands: f64,
    /// OrganicGrowth pillar: sqrt(holder_norm × inverse_concentration)
    pub organic_growth: f64,
    /// Longevity pillar: 1 - e^(-age_days/21)
    pub longevity: f64,
    /// Wallets analyzed
    pub wallets_analyzed: u32,
    /// Breakdown: how many of each class
    pub accumulators: u32,
    pub holders: u32,
    pub reducers: u32,
    pub extractors: u32,
}

impl TokenData {
    /// Format enriched data as a structured stimulus for Dogs.
    /// This follows the [METRICS] / [BASELINES] / [AXIOM EVIDENCE] / [QUESTION] format.
    pub fn to_stimulus(&self) -> String {
        crate::domain::stimulus::build_token_stimulus(self)
    }
}

/// Port trait for token enrichment.
/// Adapters implement this to fetch on-chain data from any source.
#[async_trait]
pub trait TokenEnricherPort: Send + Sync {
    /// Enrich a Solana mint address with on-chain data.
    /// Returns None if the address is not a valid token.
    async fn enrich(&self, mint_address: &str) -> Result<Option<TokenData>, EnrichmentError>;
}

/// Enrichment errors — separate from Dog/Judge errors.
#[derive(Debug, thiserror::Error)]
pub enum EnrichmentError {
    #[error("enrichment request failed: {0}")]
    RequestFailed(String),
    #[error("enrichment timed out")]
    Timeout,
    #[error("enrichment unavailable")]
    Unavailable,
}

/// Check if a string looks like a Solana address (32-44 chars, base58 alphabet).
pub fn looks_like_solana_address(s: &str) -> bool {
    let len = s.len();
    (32..=44).contains(&len)
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_solana_address() {
        assert!(looks_like_solana_address(
            "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"
        ));
        assert!(looks_like_solana_address(
            "So11111111111111111111111111111111111111112"
        ));
        assert!(!looks_like_solana_address("hello world"));
        assert!(!looks_like_solana_address("too_short"));
        // Contains 'O' which is not in base58
        assert!(!looks_like_solana_address(
            "OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO"
        ));
    }

    #[test]
    fn test_token_data_to_stimulus() {
        let data = TokenData {
            mint: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN".into(),
            name: Some("Jupiter".into()),
            symbol: Some("JUP".into()),
            supply: Some(6_863_982_190_903_847),
            decimals: Some(6),
            price_usd: Some(0.178),
            holder_count: 250_000,
            holder_data_available: true,
            top1_pct: 12.5,
            top10_pct: 45.2,
            herfindahl: Some(0.08),
            age_hours: 1200,
            mint_authority_active: false,
            freeze_authority_active: false,
            lp_status: "burned".into(),
            supply_burned_pct: Some(0.0),
            supply_locked_pct: Some(0.0),
            origin: Some("manual".into()),
            token_standard: Some("Fungible".into()),
            description: Some("JUP is the governance token for Jupiter.".into()),
            created_at: None,
            ..Default::default()
        };
        let stim = data.to_stimulus();
        assert!(stim.contains("Jupiter"));
        assert!(stim.contains("REVOKED"));
        assert!(stim.contains("holders: 250000"));
    }
}
