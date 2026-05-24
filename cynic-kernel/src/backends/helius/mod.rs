//! Helius adapter — enriches Solana token addresses via Helius DAS API.
//! Implements TokenEnricherPort. Uses mainnet RPC for real token data.

use crate::domain::enrichment::{EnrichmentError, TokenData, TokenEnricherPort};
use crate::domain::helius_credit::HeliumsCreditTracker;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

pub(crate) mod holders;
pub(crate) mod rest;
pub(crate) mod rpc;

pub(crate) const HELIUS_TIMEOUT: Duration = Duration::from_secs(10);
/// Shorter timeout for Enhanced Transactions API — hangs for 10s+ on wallets with no SWAP history.
pub(crate) const HELIUS_BEHAVIORAL_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Debug)]
pub struct HeliusEnricher {
    pub(crate) client: Client,
    pub(crate) rpc_url: String,
    pub(crate) credits: Arc<HeliumsCreditTracker>,
    pub(crate) kscore_config: crate::infra::config::KScoreConfig,
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
            kscore_config: crate::infra::config::KScoreConfig::default(),
        }
    }

    /// Set K-Score config (from backends.toml [kscore]).
    pub fn with_kscore_config(mut self, config: crate::infra::config::KScoreConfig) -> Self {
        self.kscore_config = config;
        self
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
        let (
            holder_count,
            top1_pct,
            top10_pct,
            herfindahl,
            holder_addresses,
            holder_balances,
            holder_data_available,
        ) = if let Ok(Some(conc)) = self.get_largest_accounts(mint_address, real_supply).await {
            (
                conc.accounts_seen,
                conc.top1_pct,
                conc.top10_pct,
                Some(conc.herfindahl),
                conc.holder_addresses,
                conc.holder_balances,
                true,
            )
        } else if let Some(conc) = self
            .get_holders_via_das(mint_address, real_supply, decimals)
            .await
        {
            // DAS fallback: random sample when getTokenLargestAccounts overloaded.
            // Holder count and basic stats are usable, but the sample is NOT sorted
            // by balance — the "top holders" aren't the real top holders. This means
            // LP/burn detection will fail (checking random wallets, not actual LP pools).
            // Mark holder_data_available=false so lp_status → "unknown" (not false "unsecured").
            tracing::info!(
                mint = %mint_address,
                holders = conc.accounts_seen,
                "DAS fallback — holder count usable, LP detection unreliable"
            );
            (
                conc.accounts_seen,
                conc.top1_pct,
                conc.top10_pct,
                Some(conc.herfindahl),
                vec![], // empty → detect_lp_and_supply_status returns "unknown"
                vec![], // empty → no false burn/locker classifications
                false,  // stimulus shows "holders: UNAVAILABLE" + lp="UNKNOWN"
            )
        } else {
            (0, 0.0, 0.0, None, vec![], vec![], false)
        };

        // Estimate real holder count via DAS pagination when getTokenLargestAccounts hit the 20-cap.
        // This replaces "20+" with "~100000+" for established tokens like JUP.
        // Always estimate when sample is large enough — holder_data_available only guards LP detection.
        let holder_count = if holder_count >= 20 {
            self.estimate_holder_count(mint_address).await
        } else {
            holder_count
        };

        // Detect LP status + supply burned/locked from holder account owners
        let (lp_status, supply_burned_pct, supply_locked_pct) = if !holder_addresses.is_empty() {
            self.detect_lp_and_supply_status(&holder_addresses, &holder_balances, real_supply)
                .await
        } else {
            // No holder data → can't determine LP status. "unknown" not "unsecured".
            // BONK LP is 100% burned but showed "unsecured" when RPC degraded.
            ("unknown".into(), None, None)
        };

        // Classify top1 holder type (LP pool, burn, locker, or wallet)
        let top1_type = if let Some(addr) = holder_addresses.first() {
            self.classify_holder(addr).await
        } else {
            "unknown".into()
        };

        // Classify ALL top holders by account type (wallet vs contract).
        // This enables Dogs to distinguish vesting/LP concentration from retail whale concentration.
        // Cost: 2 credits (2× getMultipleAccounts). Timeout: 5s.
        let holder_context = if !holder_addresses.is_empty() {
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                self.classify_holders_batch(
                    &holder_addresses,
                    &holder_balances,
                    real_supply.unwrap_or(0.0),
                ),
            )
            .await
            {
                Ok(ctx) => ctx,
                Err(_) => {
                    tracing::warn!(
                        mint = %mint_address,
                        "holder context classification timed out (5s)"
                    );
                    None
                }
            }
        } else {
            None
        };

        let (age_hours, age_is_exact) = self
            .get_token_age_hours(mint_address)
            .await
            .unwrap_or((0, false));

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

        // Resolve token accounts to owner wallets for identity lookup.
        // getTokenLargestAccounts returns ATA addresses, not wallet addresses.
        // DAS fallback already returns owner addresses directly.
        let owner_addresses = if !holder_addresses.is_empty() {
            self.resolve_owners(&holder_addresses).await
        } else {
            vec![]
        };

        // Identity resolution for top holders (Helius Wallet API batch-identity).
        // Cost: 100 credits per batch of up to 100 addresses. Timeout: 5s.
        let holder_identities = if !owner_addresses.is_empty() {
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                self.batch_identity(&owner_addresses),
            )
            .await
            {
                Ok(ids) => ids,
                Err(_) => {
                    tracing::warn!(
                        mint = %mint_address,
                        "batch-identity timed out (5s) — proceeding without identity data"
                    );
                    vec![]
                }
            }
        } else {
            vec![]
        };

        // Behavioral analysis (K-Score) — own timeout so basic enrichment isn't blocked.
        // 20s budget: N wallets × (resolve 1s + SWAP 4s) at ~5s/wallet × 5 wallets.
        // If timeout: return enriched data WITHOUT K-Score (deterministic dog still scores).
        let (wallet_behaviors, kscore) = if !holder_addresses.is_empty() {
            match tokio::time::timeout(
                std::time::Duration::from_secs(20),
                self.analyze_behaviors(
                    mint_address,
                    &holder_addresses,
                    &holder_balances,
                    &self.kscore_config,
                    holder_count,
                    top10_pct,
                    age_hours,
                ),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => {
                    tracing::warn!(
                        mint = %mint_address,
                        "behavioral analysis timed out (20s) — returning enrichment without K-Score"
                    );
                    (vec![], crate::domain::enrichment::KScore::default())
                }
            }
        } else {
            (vec![], crate::domain::enrichment::KScore::default())
        };
        let kscore = if kscore.wallets_analyzed > 0 {
            Some(kscore)
        } else {
            None
        };

        // Compute derived fields before struct init (kscore is moved into the struct).
        let buy_sell_ratio = kscore.as_ref().and_then(|k| {
            (k.wallets_analyzed > 0)
                .then(|| (k.accumulators + k.holders) as f64 / k.wallets_analyzed as f64)
        });
        let divergence_class = kscore.as_ref().and_then(|k| {
            (k.wallets_analyzed > 0).then(|| {
                let r = (k.accumulators + k.holders) as f64 / k.wallets_analyzed as f64;
                if r >= 0.7 {
                    "STRONG_HOLD"
                } else if r <= 0.3 {
                    "DISTRIBUTION"
                } else {
                    "MIXED"
                }
                .to_string()
            })
        });

        Ok(Some(TokenData {
            mint: mint_address.to_string(),
            name,
            symbol,
            supply,
            decimals,
            price_usd,
            holder_count,
            holder_count_is_exact: holder_count < 20,
            holder_data_available,
            top1_pct,
            top1_type,
            top10_pct,
            herfindahl,
            age_hours,
            age_is_exact,
            mint_authority_active,
            freeze_authority_active,
            lp_status,
            supply_burned_pct,
            supply_locked_pct,
            volume_24h_usd: None, // populated by Jupiter adapter when available
            liquidity_usd: None,  // populated by Jupiter adapter when available
            origin,
            token_standard,
            description,
            created_at: None,
            kscore,
            wallet_behaviors,
            holder_identities,
            holder_context,
            buy_sell_ratio,
            divergence_class,
            percentile_divergence: None,
            trajectory_class: None, // populated from observation store during enrichment
            trajectory_decay: None,
        }))
    }
}

// ── Helius DAS response types ──

#[derive(Debug, Deserialize)]
pub(crate) struct RpcResponse<T> {
    pub(crate) result: Option<T>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HeliusAsset {
    pub(crate) content: Option<AssetContent>,
    pub(crate) token_info: Option<TokenInfo>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AssetContent {
    pub(crate) metadata: Option<AssetMetadata>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AssetMetadata {
    pub(crate) name: Option<String>,
    pub(crate) symbol: Option<String>,
    pub(crate) token_standard: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenInfo {
    pub(crate) supply: Option<u64>,
    pub(crate) decimals: Option<u8>,
    pub(crate) mint_authority: Option<String>,
    pub(crate) freeze_authority: Option<String>,
    pub(crate) price_info: Option<PriceInfo>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PriceInfo {
    pub(crate) price_per_token: Option<f64>,
}

/// Solana RPC responses with context wrapper (getTokenLargestAccounts, etc.)
#[derive(Debug, Deserialize)]
pub(crate) struct RpcResponseWithContext<T> {
    pub(crate) result: Option<RpcContext<T>>,
    pub(crate) error: Option<RpcError>,
}

/// JSON-RPC error object returned by Solana RPC on failure.
#[derive(Debug, Deserialize)]
pub(crate) struct RpcError {
    pub(crate) code: i64,
    pub(crate) message: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RpcContext<T> {
    pub(crate) value: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LargestAccount {
    pub(crate) address: String,
    pub(crate) ui_amount: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SignatureInfo {
    pub(crate) signature: Option<String>,
    pub(crate) block_time: Option<i64>,
}

#[derive(Debug, Clone)]
pub(crate) struct HolderConcentration {
    /// Number of accounts returned by getTokenLargestAccounts (max 20).
    /// NOT the real holder count — a lower bound.
    pub(crate) accounts_seen: u64,
    pub(crate) top1_pct: f64,
    pub(crate) top10_pct: f64,
    pub(crate) herfindahl: f64,
    /// Token account addresses of top holders (for LP burn detection + behavioral).
    pub(crate) holder_addresses: Vec<String>,
    /// ui_amounts per holder (parallel to holder_addresses, for retention calculation).
    pub(crate) holder_balances: Vec<f64>,
}
