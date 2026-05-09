//! DexScreener adapter — fetches market data (volume, liquidity, market cap) for Solana tokens.
//! Free API, no key required. Used to complement Helius enrichment with market signals.
//! Rate limit: 60 req/min (undocumented but observed).

use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

const DEXSCREENER_TIMEOUT: Duration = Duration::from_secs(5);

/// Market data from DexScreener for a single token (best pair).
#[derive(Debug, Clone, Default)]
pub struct DexMarketData {
    /// Price in USD from the highest-liquidity pair
    pub price_usd: Option<f64>,
    /// 24h trading volume in USD
    pub volume_24h_usd: Option<f64>,
    /// Total liquidity in USD for the best pair
    pub liquidity_usd: Option<f64>,
    /// Fully diluted valuation
    pub fdv_usd: Option<f64>,
    /// Market capitalization
    pub market_cap_usd: Option<f64>,
}

#[derive(Debug)]
pub struct DexScreenerClient {
    client: Client,
}

impl Default for DexScreenerClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DexScreenerClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(DEXSCREENER_TIMEOUT)
                .build()
                .unwrap_or_default(),
        }
    }

    /// Fetch market data for a Solana token by mint address.
    /// Returns the best (highest liquidity) pair's data.
    /// Cost: 0 credits (free API, no key).
    pub async fn get_market_data(&self, mint: &str) -> Option<DexMarketData> {
        let url = format!("https://api.dexscreener.com/tokens/v1/solana/{mint}");

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .inspect_err(|e| {
                tracing::debug!(mint = %mint, error = %e, "DexScreener request failed");
            })
            .ok()?;

        if !resp.status().is_success() {
            tracing::debug!(mint = %mint, status = resp.status().as_u16(), "DexScreener non-200");
            return None;
        }

        let pairs: Vec<DexPair> = resp
            .json()
            .await
            .inspect_err(|e| {
                tracing::debug!(mint = %mint, error = %e, "DexScreener deserialize failed");
            })
            .ok()?;

        // Take the first pair (DexScreener returns sorted by liquidity desc)
        let best = pairs.first()?;

        let data = DexMarketData {
            price_usd: best.price_usd.as_deref().and_then(|p| p.parse().ok()),
            volume_24h_usd: best.volume.as_ref().and_then(|v| v.h24),
            liquidity_usd: best.liquidity.as_ref().and_then(|l| l.usd),
            fdv_usd: best.fdv,
            market_cap_usd: best.market_cap,
        };

        tracing::info!(
            mint = %mint,
            price = ?data.price_usd,
            volume_24h = ?data.volume_24h_usd,
            liquidity = ?data.liquidity_usd,
            "DexScreener market data fetched"
        );

        Some(data)
    }
}

// ── DexScreener response types ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DexPair {
    price_usd: Option<String>,
    volume: Option<DexVolume>,
    liquidity: Option<DexLiquidity>,
    fdv: Option<f64>,
    market_cap: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct DexVolume {
    h24: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct DexLiquidity {
    usd: Option<f64>,
}
