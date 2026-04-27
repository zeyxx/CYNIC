//! Helius adapter — enriches Solana token addresses via Helius DAS API.
//! Implements TokenEnricherPort. Uses mainnet RPC for real token data.

use crate::domain::enrichment::{EnrichmentError, TokenData, TokenEnricherPort};
use crate::domain::helius_credit::HeliumsCreditTracker;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

const HELIUS_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct HeliusEnricher {
    client: Client,
    rpc_url: String,
    credits: Arc<HeliumsCreditTracker>,
}

impl HeliusEnricher {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(HELIUS_TIMEOUT)
                .build()
                .unwrap_or_default(),
            rpc_url: format!("https://mainnet.helius-rpc.com/?api-key={api_key}"),
            credits: Arc::new(HeliumsCreditTracker::new(chrono::Utc::now().to_rfc3339())),
        }
    }

    /// Get a snapshot of credit usage for /health reporting.
    pub fn credit_snapshot(&self) -> Option<crate::domain::helius_credit::HeliumsCreditBudget> {
        self.credits.snapshot()
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

        let latency = start.elapsed().as_millis();
        self.credits.record_call(latency, true, 10); // DAS: 10 credits
        tracing::debug!(
            method = "getAsset",
            mint = %mint,
            status = 200,
            latency_ms = latency,
            credits_cost = 10,
            "Helius getAsset succeeded"
        );

        Ok(rpc.result)
    }

    /// Fetch largest accounts for holder concentration metrics.
    /// `total_supply` from getAsset is the real denominator for share %.
    /// Without it, shares are relative to the top-20 sum (misleading).
    async fn get_largest_accounts(
        &self,
        mint: &str,
        total_supply: Option<f64>,
    ) -> Result<Option<HolderConcentration>, EnrichmentError> {
        let start = std::time::Instant::now();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenLargestAccounts",
            "params": [mint]
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

        let rpc: RpcResponseWithContext<Vec<LargestAccount>> = resp.json().await.map_err(|e| {
            tracing::warn!(
                method = "getTokenLargestAccounts",
                mint = %mint,
                error = %e,
                latency_ms = start.elapsed().as_millis(),
                "Helius response deserialize failed"
            );
            EnrichmentError::RequestFailed(e.to_string())
        })?;

        let Some(ctx) = rpc.result else {
            return Ok(None);
        };
        let accounts = ctx.value;

        if accounts.is_empty() {
            return Ok(None);
        }

        // Use real total supply if available; fall back to sum of top accounts
        let supply_f64 = if let Some(ts) = total_supply
            && ts > 0.0
        {
            ts
        } else {
            let sum: f64 = accounts.iter().map(|a| a.ui_amount.unwrap_or(0.0)).sum();
            if sum <= 0.0 {
                return Ok(None);
            }
            sum
        };
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

        let latency = start.elapsed().as_millis();
        self.credits.record_call(latency, true, 1); // Standard RPC: 1 credit
        tracing::debug!(
            method = "getTokenLargestAccounts",
            mint = %mint,
            status = 200,
            holder_count = accounts.len(),
            latency_ms = latency,
            credits_cost = 1,
            "Helius getTokenLargestAccounts succeeded"
        );

        Ok(Some(HolderConcentration {
            accounts_seen: accounts.len() as u64,
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

    /// Estimate token age from transaction history. 1 credit.
    ///
    /// Calls getSignaturesForAddress (newest-first, limit=1000).
    /// - If < 1000 results: last result = creation tx → exact age.
    /// - If = 1000 results: token has >1000 txs → mature (≥ oldest in batch).
    ///
    /// This is exact for young/pump.fun tokens (the critical case for scoring)
    /// and conservative for established tokens.
    async fn get_token_age_hours(&self, mint: &str) -> Result<u64, EnrichmentError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSignaturesForAddress",
            "params": [mint, {"limit": 1000}]
        });

        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Ok(0);
        }

        let rpc: RpcResponse<Vec<SignatureInfo>> = resp
            .json()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(e.to_string()))?;

        let Some(sigs) = rpc.result else {
            return Ok(0);
        };

        if sigs.is_empty() {
            return Ok(0);
        }

        // Last item in the array = oldest in this batch (newest-first ordering)
        let oldest_block_time = sigs.last().and_then(|s| s.block_time).unwrap_or(0);

        if oldest_block_time == 0 {
            return Ok(0);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) as i64;

        let age_secs = (now - oldest_block_time).max(0) as u64;
        let age_hours = age_secs / 3600;

        tracing::debug!(
            method = "getSignaturesForAddress",
            mint = %mint,
            sigs_returned = sigs.len(),
            oldest_block_time,
            age_hours,
            exact = sigs.len() < 1000,
            credits_cost = 1,
            "Token age estimated"
        );

        Ok(age_hours)
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

        // Compute real supply in human units for concentration denominator
        let real_supply = match (supply, decimals) {
            (Some(s), Some(d)) if s > 0 => Some(s as f64 / 10_f64.powi(d as i32)),
            _ => None,
        };

        // Get concentration metrics from top-20 accounts, using real supply as denominator
        let (holder_count, top1_pct, top10_pct, herfindahl) =
            if let Ok(Some(conc)) = self.get_largest_accounts(mint_address, real_supply).await {
                (
                    conc.accounts_seen,
                    conc.top1_pct,
                    conc.top10_pct,
                    Some(conc.herfindahl),
                )
            } else {
                (0, 0.0, 0.0, None)
            };

        // Detect pump.fun origin from mint suffix
        let origin = if mint_address.ends_with("pump") {
            Some("pump.fun".to_string())
        } else {
            None
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
            age_hours: self.get_token_age_hours(mint_address).await.unwrap_or(0),
            mint_authority_active,
            freeze_authority_active,
            lp_status: "unsecured".into(), // TODO: check Raydium/Jupiter LP state
            supply_burned_pct: None,
            supply_locked_pct: None,
            origin,
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

/// Solana RPC responses with context wrapper (getTokenLargestAccounts, etc.)
#[derive(Debug, Deserialize)]
struct RpcResponseWithContext<T> {
    result: Option<RpcContext<T>>,
}

#[derive(Debug, Deserialize)]
struct RpcContext<T> {
    value: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LargestAccount {
    ui_amount: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignatureInfo {
    block_time: Option<i64>,
}

#[derive(Debug, Clone)]
struct HolderConcentration {
    /// Number of accounts returned by getTokenLargestAccounts (max 20).
    /// NOT the real holder count — a lower bound.
    accounts_seen: u64,
    top1_pct: f64,
    top10_pct: f64,
    herfindahl: f64,
}
