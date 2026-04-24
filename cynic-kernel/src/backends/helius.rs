//! Helius adapter — enriches Solana token addresses via Helius DAS API.
//! Implements TokenEnricherPort. Uses mainnet RPC for real token data.

use crate::domain::enrichment::{EnrichmentError, TokenData, TokenEnricherPort};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

const HELIUS_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct HeliusEnricher {
    client: Client,
    rpc_url: String,
}

impl HeliusEnricher {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(HELIUS_TIMEOUT)
                .build()
                .unwrap_or_default(),
            rpc_url: format!("https://mainnet.helius-rpc.com/?api-key={api_key}"),
        }
    }

    /// Build from HELIUS_API_KEY env var or ~/.helius/config.json.
    pub fn from_env() -> Option<Self> {
        // Try env var first
        if let Ok(key) = std::env::var("HELIUS_API_KEY")
            && !key.is_empty()
        {
            return Some(Self::new(&key));
        }
        // Fallback: ~/.helius/config.json
        let config_path = dirs::home_dir()?.join(".helius/config.json");
        let data = std::fs::read_to_string(&config_path).ok()?;
        let config: serde_json::Value = serde_json::from_str(&data)
            .inspect_err(|e| tracing::warn!(path = %config_path.display(), error = %e, "helius config.json malformed — enricher disabled"))
            .ok()?;
        let key = config.get("apiKey")?.as_str()?;
        if key.is_empty() {
            return None;
        }
        Some(Self::new(key))
    }

    async fn get_asset(&self, mint: &str) -> Result<Option<HeliusAsset>, EnrichmentError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAsset",
            "params": { "id": mint }
        });

        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EnrichmentError::RequestFailed(format!(
                "Helius returned {}",
                resp.status()
            )));
        }

        let rpc: RpcResponse<HeliusAsset> = resp
            .json()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(e.to_string()))?;

        Ok(rpc.result)
    }

    /// Fetch off-chain metadata (description, links) via Helius REST API.
    async fn get_offchain_metadata(&self, mint: &str) -> Result<Option<String>, EnrichmentError> {
        // Extract API key from RPC URL
        let api_key = self.rpc_url.split("api-key=").nth(1).unwrap_or_default();

        let url = format!("https://api.helius.xyz/v0/token-metadata?api-key={api_key}");
        let body = serde_json::json!({
            "mintAccounts": [mint],
            "includeOffChain": true
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let data: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(e.to_string()))?;

        let desc = data
            .first()
            .and_then(|t| t.get("offChainMetadata"))
            .and_then(|m| m.get("metadata"))
            .and_then(|m| m.get("description"))
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());

        Ok(desc)
    }
}

#[async_trait]
impl TokenEnricherPort for HeliusEnricher {
    async fn enrich(&self, mint_address: &str) -> Result<Option<TokenData>, EnrichmentError> {
        let Some(asset) = self.get_asset(mint_address).await? else {
            return Ok(None);
        };

        // Only enrich fungible tokens
        let token_standard = asset
            .content
            .as_ref()
            .and_then(|c| c.metadata.as_ref())
            .and_then(|m| m.token_standard.clone());

        let name = asset
            .content
            .as_ref()
            .and_then(|c| c.metadata.as_ref())
            .and_then(|m| m.name.clone());

        let symbol = asset
            .content
            .as_ref()
            .and_then(|c| c.metadata.as_ref())
            .and_then(|m| m.symbol.clone());

        let supply = asset.token_info.as_ref().and_then(|t| t.supply);
        let decimals = asset.token_info.as_ref().and_then(|t| t.decimals);
        let price_usd = asset
            .token_info
            .as_ref()
            .and_then(|t| t.price_info.as_ref())
            .and_then(|p| p.price_per_token);

        let mint_authority_active = asset
            .token_info
            .as_ref()
            .and_then(|t| t.mint_authority.as_ref())
            .is_some();

        let freeze_authority_active = asset
            .token_info
            .as_ref()
            .and_then(|t| t.freeze_authority.as_ref())
            .is_some();

        // Get off-chain description (separate call, best-effort)
        let description = self
            .get_offchain_metadata(mint_address)
            .await
            .unwrap_or(None);

        Ok(Some(TokenData {
            mint: mint_address.to_string(),
            name,
            symbol,
            supply,
            decimals,
            price_usd,
            holder_count: 0, // DAS doesn't always return this; set to 0 for now
            top1_pct: 0.0,
            top10_pct: 0.0,
            herfindahl: None,
            age_hours: 0,
            mint_authority_active,
            freeze_authority_active,
            lp_status: "unsecured".into(), // Default to unsecured if unknown
            supply_burned_pct: None,
            supply_locked_pct: None,
            origin: None,
            token_standard,
            description,
            created_at: None,
        }))
    }
}

// ── Helius DAS response types ──

#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct HeliusAsset {
    content: Option<AssetContent>,
    token_info: Option<TokenInfo>,
}

#[derive(Debug, Deserialize)]
struct AssetContent {
    metadata: Option<AssetMetadata>,
}

#[derive(Debug, Deserialize)]
struct AssetMetadata {
    name: Option<String>,
    symbol: Option<String>,
    token_standard: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenInfo {
    supply: Option<u64>,
    decimals: Option<u8>,
    mint_authority: Option<String>,
    freeze_authority: Option<String>,
    price_info: Option<PriceInfo>,
}

#[derive(Debug, Deserialize)]
struct PriceInfo {
    price_per_token: Option<f64>,
}
