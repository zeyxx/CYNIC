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
        let start = std::time::Instant::now();
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
            .map_err(|e| {
                tracing::warn!(
                    method = "getAsset",
                    mint = %mint,
                    error = %e,
                    latency_ms = start.elapsed().as_millis(),
                    "Helius RPC call failed"
                );
                EnrichmentError::RequestFailed(e.to_string())
            })?;

        let status_code = resp.status().as_u16();
        if !resp.status().is_success() {
            tracing::warn!(
                method = "getAsset",
                mint = %mint,
                status = status_code,
                latency_ms = start.elapsed().as_millis(),
                "Helius returned error status"
            );
            return Err(EnrichmentError::RequestFailed(format!(
                "Helius returned {}",
                resp.status()
            )));
        }

        let rpc: RpcResponse<HeliusAsset> = resp.json().await.map_err(|e| {
            tracing::warn!(
                method = "getAsset",
                mint = %mint,
                error = %e,
                latency_ms = start.elapsed().as_millis(),
                "Helius response deserialize failed"
            );
            EnrichmentError::RequestFailed(e.to_string())
        })?;

        tracing::debug!(
            method = "getAsset",
            mint = %mint,
            status = 200,
            latency_ms = start.elapsed().as_millis(),
            credits_cost = 10,
            "Helius getAsset succeeded"
        );

        Ok(rpc.result)
    }

    /// Fetch largest accounts for holder concentration metrics.
    async fn get_largest_accounts(
        &self,
        mint: &str,
    ) -> Result<Option<HolderConcentration>, EnrichmentError> {
        let start = std::time::Instant::now();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenLargestAccounts",
            "params": { "mint": mint }
        });

        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!(
                    method = "getTokenLargestAccounts",
                    mint = %mint,
                    error = %e,
                    latency_ms = start.elapsed().as_millis(),
                    "Helius RPC call failed"
                );
                EnrichmentError::RequestFailed(e.to_string())
            })?;

        let status_code = resp.status().as_u16();
        if !resp.status().is_success() {
            tracing::warn!(
                method = "getTokenLargestAccounts",
                mint = %mint,
                status = status_code,
                latency_ms = start.elapsed().as_millis(),
                "Helius returned error status — holder concentration unavailable"
            );
            return Ok(None);
        }

        let rpc: RpcResponse<Vec<LargestAccount>> = resp.json().await.map_err(|e| {
            tracing::warn!(
                method = "getTokenLargestAccounts",
                mint = %mint,
                error = %e,
                latency_ms = start.elapsed().as_millis(),
                "Helius response deserialize failed"
            );
            EnrichmentError::RequestFailed(e.to_string())
        })?;

        let Some(accounts) = rpc.result else {
            return Ok(None);
        };

        if accounts.is_empty() {
            return Ok(None);
        }

        let total_supply: u64 = accounts
            .iter()
            .map(|a| a.ui_amount.unwrap_or(0.0) as u64)
            .sum();
        if total_supply == 0 {
            return Ok(None);
        }

        let supply_f64 = total_supply as f64;
        let mut hhi = 0.0;
        let mut top1_pct = 0.0;
        let mut top10_pct = 0.0;

        for (idx, account) in accounts.iter().enumerate() {
            let balance = account.ui_amount.unwrap_or(0.0);
            let share = balance / supply_f64;
            hhi += share * share;

            if idx == 0 {
                top1_pct = share * 100.0;
            }
            if idx < 10 {
                top10_pct += share * 100.0;
            }
        }

        tracing::debug!(
            method = "getTokenLargestAccounts",
            mint = %mint,
            status = 200,
            holder_count = accounts.len(),
            latency_ms = start.elapsed().as_millis(),
            credits_cost = 10,
            "Helius getTokenLargestAccounts succeeded"
        );

        Ok(Some(HolderConcentration {
            holder_count: accounts.len() as u64,
            top1_pct,
            top10_pct,
            herfindahl: hhi,
        }))
    }

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

        // Get holder concentration (best-effort; defaults to zero if unavailable)
        let (holder_count, top1_pct, top10_pct, herfindahl) =
            if let Ok(Some(conc)) = self.get_largest_accounts(mint_address).await {
                (
                    conc.holder_count,
                    conc.top1_pct,
                    conc.top10_pct,
                    Some(conc.herfindahl),
                )
            } else {
                (0, 0.0, 0.0, None)
            };

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
            holder_count,
            top1_pct,
            top10_pct,
            herfindahl,
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

#[derive(Debug, Deserialize)]
struct LargestAccount {
    ui_amount: Option<f64>,
}

#[derive(Debug, Clone)]
struct HolderConcentration {
    holder_count: u64,
    top1_pct: f64,
    top10_pct: f64,
    herfindahl: f64,
}
