//! Market data and DEX pricing port.
//!
//! Canonical owner: cynic-intel (§481 one_boundary_one_reason).
//! Defines market data structures and the observation port used to query token pricing.

use serde::{Deserialize, Serialize};

/// Market and liquidity data for a token from a DEX aggregator.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DexMarketData {
    /// Price in USD from the highest-liquidity pair.
    pub price_usd: Option<f64>,
    /// 24h trading volume in USD.
    pub volume_24h_usd: Option<f64>,
    /// Total liquidity in USD for the best pair.
    pub liquidity_usd: Option<f64>,
    /// Fully diluted valuation.
    pub fdv_usd: Option<f64>,
    /// Market capitalization in USD.
    pub market_cap_usd: Option<f64>,
}

/// Port for querying external market and pricing intelligence.
#[async_trait::async_trait]
pub trait MarketDataPort: Send + Sync {
    /// Fetch market data for a token by mint address.
    async fn fetch_token_market_data(&self, mint: &str) -> Option<DexMarketData>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockMarketData;

    #[async_trait::async_trait]
    impl MarketDataPort for MockMarketData {
        async fn fetch_token_market_data(&self, _mint: &str) -> Option<DexMarketData> {
            Some(DexMarketData {
                price_usd: Some(1.25),
                volume_24h_usd: Some(500_000.0),
                liquidity_usd: Some(100_000.0),
                fdv_usd: Some(1_250_000.0),
                market_cap_usd: Some(1_000_000.0),
            })
        }
    }

    #[tokio::test]
    async fn test_mock_market_data_port() {
        let port = MockMarketData;
        let data = port
            .fetch_token_market_data("So11111111111111111111111111111111111111112")
            .await;
        assert!(data.is_some());
        let d = data.unwrap();
        assert_eq!(d.price_usd, Some(1.25));
        assert_eq!(d.volume_24h_usd, Some(500_000.0));
    }
}
