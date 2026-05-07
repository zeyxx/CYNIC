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
/// Shorter timeout for Enhanced Transactions API — hangs for 10s+ on wallets with no SWAP history.
const HELIUS_BEHAVIORAL_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Debug)]
pub struct HeliusEnricher {
    client: Client,
    rpc_url: String,
    credits: Arc<HeliumsCreditTracker>,
    kscore_config: crate::infra::config::KScoreConfig,
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

        let holder_addresses: Vec<String> = accounts.iter().map(|a| a.address.clone()).collect();
        let holder_balances: Vec<f64> = accounts
            .iter()
            .map(|a| a.ui_amount.unwrap_or(0.0))
            .collect();

        Ok(Some(HolderConcentration {
            accounts_seen: accounts.len() as u64,
            top1_pct,
            top10_pct,
            herfindahl: hhi,
            holder_addresses,
            holder_balances,
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

    /// Estimate token age from transaction history via backward pagination.
    ///
    /// Strategy: paginate getSignaturesForAddress backward (max 5 pages × 1000 sigs = 5000 txs).
    /// - If < 1000 on any page: found creation tx → exact age.
    /// - If still 1000 after 5 pages: token has >5000 txs. Any token with 5000+ txs is
    ///   clearly established — use a floor of 720h (30 days) for longevity if the measured
    ///   window is shorter. This prevents active tokens getting longevity=0.004.
    ///
    /// Cost: 1-5 credits (10 credits each on Helius, paginated).
    /// Exact for tokens with <5000 txs (covers all pump.fun rugs and most new tokens).
    async fn get_token_age_hours(&self, mint: &str) -> Result<u64, EnrichmentError> {
        const MAX_PAGES: usize = 5;
        let mut oldest_block_time: i64 = 0;
        let mut before_sig: Option<String> = None;
        let mut total_sigs: usize = 0;
        let mut found_creation = false;

        for page in 0..MAX_PAGES {
            let mut params = serde_json::json!([mint, {"limit": 1000}]);
            if let Some(ref sig) = before_sig {
                params[1]["before"] = serde_json::json!(sig);
            }
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getSignaturesForAddress",
                "params": params
            });

            let resp = self
                .client
                .post(&self.rpc_url)
                .json(&body)
                .send()
                .await
                .map_err(|e| EnrichmentError::RequestFailed(e.to_string()))?;

            if !resp.status().is_success() {
                break;
            }

            let rpc: RpcResponse<Vec<SignatureInfo>> = resp
                .json()
                .await
                .map_err(|e| EnrichmentError::RequestFailed(e.to_string()))?;

            let Some(sigs) = rpc.result else { break };
            let batch_len = sigs.len();
            if batch_len == 0 {
                break;
            }

            total_sigs += batch_len;

            // Track oldest blockTime and cursor for next page
            if let Some(last) = sigs.last() {
                if let Some(bt) = last.block_time {
                    oldest_block_time = bt;
                }
                before_sig = last.signature.clone();
            }

            if batch_len < 1000 {
                found_creation = true;
                break;
            }

            // Don't paginate further if we're already deep enough
            if page >= MAX_PAGES - 1 {
                break;
            }
        }

        if oldest_block_time == 0 {
            return Ok(0);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) as i64;

        let measured_age_hours = ((now - oldest_block_time).max(0) as u64) / 3600;

        // If we couldn't reach creation and measured age < 30 days,
        // the token is clearly established (>5000 txs). Apply floor.
        let age_hours = if !found_creation && measured_age_hours < 720 {
            720 // 30-day floor for tokens with >5000 txs
        } else {
            measured_age_hours
        };

        tracing::info!(
            method = "getSignaturesForAddress",
            mint = %mint,
            total_sigs,
            oldest_block_time,
            measured_age_hours,
            age_hours,
            found_creation,
            "Token age estimated"
        );

        Ok(age_hours)
    }

    /// Resolve a token account address to its owner wallet address.
    /// Cost: 1 credit (getAccountInfo).
    async fn resolve_owner(&self, token_account: &str) -> Option<String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getAccountInfo",
            "params": [token_account, {"encoding": "jsonParsed"}]
        });
        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .inspect_err(
                |e| tracing::debug!(token_account, error = %e, "resolve_owner request failed"),
            )
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let rpc: serde_json::Value = resp
            .json()
            .await
            .inspect_err(
                |e| tracing::debug!(token_account, error = %e, "resolve_owner parse failed"),
            )
            .ok()?;
        self.credits.record_call(0, true, 1);
        rpc.pointer("/result/value/data/parsed/info/owner")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Detect LP status by checking if top holder token accounts are owned by burn/locker addresses.
    /// Returns "burned" | "locked" | "unsecured".
    /// Cost: 1-5 credits (1 getAccountInfo per holder checked).
    async fn detect_lp_status(&self, holder_addresses: &[String]) -> String {
        /// Known Solana burn addresses — tokens sent here are irrecoverable.
        const BURN_ADDRESSES: &[&str] = &[
            "1nc1nerator11111111111111111111111111111111",
            "1111111111111111111111111111111111111111111",
            // Raydium burn vault
            "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
        ];
        /// Known locker programs — LP tokens held by these are locked, not burned.
        const LOCKER_PROGRAMS: &[&str] = &[
            // Streamflow
            "8e72pYCDaxu3GqMfeQ5r8wFgoZSYk6oua1Qo9XpsZjX",
            // Team.finance / Uncx
            "2r5VekMNiWPzi1pWwvJczrdPaZnJG59u91unSrTunwJg",
        ];

        let check_count = holder_addresses.len().min(5);
        for addr in &holder_addresses[..check_count] {
            let Some(owner) = self.resolve_owner(addr).await else {
                continue;
            };

            if BURN_ADDRESSES.contains(&owner.as_str()) {
                tracing::debug!(token_account = %addr, owner = %owner, "LP detected: burned");
                return "burned".into();
            }
            if LOCKER_PROGRAMS.contains(&owner.as_str()) {
                tracing::debug!(token_account = %addr, owner = %owner, "LP detected: locked");
                return "locked".into();
            }
        }

        "unsecured".into()
    }

    /// Fetch SWAP transactions for a wallet, filtered to a specific token mint.
    /// Returns total_bought (tokens flowing IN to wallet) for retention calculation.
    /// Cost: 50 credits per call (Enhanced Transactions API).
    async fn get_wallet_total_bought(
        &self,
        wallet_owner: &str,
        target_mint: &str,
        limit: usize,
    ) -> Result<(f64, u32), EnrichmentError> {
        let api_key = self.rpc_url.split("api-key=").nth(1).unwrap_or_default();
        let url = format!(
            "https://api.helius.xyz/v0/addresses/{wallet_owner}/transactions?api-key={api_key}&limit={limit}&type=SWAP"
        );

        // Use shorter timeout — Enhanced Transactions API hangs 10s+ on wallets with no SWAP history
        let resp = self
            .client
            .get(&url)
            .timeout(HELIUS_BEHAVIORAL_TIMEOUT)
            .send()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Ok((0.0, 0));
        }

        self.credits.record_call(0, true, 50); // Enhanced Transactions: 50 credits

        let txs: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(e.to_string()))?;

        let mut total_bought = 0.0_f64;
        let mut swap_count = 0_u32;

        for tx in &txs {
            let Some(transfers) = tx.get("tokenTransfers").and_then(|v| v.as_array()) else {
                continue;
            };

            for transfer in transfers {
                let mint = transfer.get("mint").and_then(|v| v.as_str()).unwrap_or("");
                if mint != target_mint {
                    continue;
                }

                let amount = transfer
                    .get("tokenAmount")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let to = transfer
                    .get("toUserAccount")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if to == wallet_owner {
                    total_bought += amount;
                }
                swap_count += 1;
            }
        }

        Ok((total_bought, swap_count))
    }

    /// Analyze top-N holders' SWAP behavior for a token.
    /// Returns per-wallet classifications and K-Score composite.
    /// Cost: ~N×51 credits (1 resolve + 50 enhanced transactions per wallet).
    ///
    /// Fix C2: retention = current_balance / total_bought (not (bought-sold)/bought).
    /// current_balance comes from getTokenLargestAccounts (already fetched).
    /// total_bought comes from Enhanced Transactions SWAP history.
    // WHY: args are logically distinct (mint, holders, config, metrics) — grouping into
    // a struct would add indirection without simplifying the single call site in enrich().
    #[allow(clippy::too_many_arguments)]
    pub async fn analyze_behaviors(
        &self,
        mint: &str,
        holder_addresses: &[String],
        holder_balances: &[f64],
        config: &crate::infra::config::KScoreConfig,
        holder_count: u64,
        top10_pct: f64,
        age_hours: u64,
    ) -> (
        Vec<crate::domain::enrichment::WalletBehavior>,
        crate::domain::enrichment::KScore,
    ) {
        use crate::domain::enrichment::{HolderClass, KScore, WalletBehavior};

        let n = holder_addresses.len().min(config.top_n_wallets);
        let mut behaviors = Vec::with_capacity(n);

        for (i, addr) in holder_addresses[..n].iter().enumerate() {
            let current_balance = holder_balances.get(i).copied().unwrap_or(0.0);
            if current_balance <= 0.0 {
                continue;
            }

            // Resolve token account → wallet owner
            let Some(owner) = self.resolve_owner(addr).await else {
                continue;
            };

            // Fetch SWAP history for this wallet
            let Ok((total_bought, swaps)) = self
                .get_wallet_total_bought(&owner, mint, config.swap_history_limit)
                .await
            else {
                continue;
            };

            // Fix C2: retention = current_balance / total_bought
            // If total_bought is 0 (no SWAP history found), wallet may have received tokens
            // via transfer, airdrop, etc. — classify as Holder (conservative).
            let retention = if total_bought > 0.0 {
                current_balance / total_bought
            } else {
                1.0 // No SWAP data = assume holding
            };

            let class = if retention >= config.accumulator_threshold {
                HolderClass::Accumulator
            } else if retention >= config.holder_threshold {
                HolderClass::Holder
            } else if retention >= config.reducer_threshold {
                HolderClass::Reducer
            } else {
                HolderClass::Extractor
            };

            behaviors.push(WalletBehavior {
                class,
                retention_ratio: retention,
                swap_count: swaps,
            });
        }

        // Compute K-Score from behaviors
        let total = behaviors.len() as f64;
        if total == 0.0 {
            return (behaviors, KScore::default());
        }

        let acc = behaviors
            .iter()
            .filter(|b| b.class == HolderClass::Accumulator)
            .count() as f64;
        let hld = behaviors
            .iter()
            .filter(|b| b.class == HolderClass::Holder)
            .count() as f64;
        let ext = behaviors
            .iter()
            .filter(|b| b.class == HolderClass::Extractor)
            .count() as f64;
        let red = behaviors
            .iter()
            .filter(|b| b.class == HolderClass::Reducer)
            .count() as f64;

        // DiamondHands = sqrt(conviction × retention_signal)
        // conviction: fraction of analyzed wallets that are accumulating or holding
        // retention_signal: tanh(acc/ext ratio / 2)
        //   Fix C3 documentation: tanh maps [0,∞) → [0,1).
        //   When acc >> ext: tanh → 1.0 (strong diamond hands signal).
        //   When ext >> acc: tanh → 0.0 (everyone selling).
        //   Division by 2 normalizes so 1:1 ratio → tanh(0.5) ≈ 0.46 (neutral).
        let conviction = (acc + hld) / total;
        let retention_signal = (acc / ext.max(1.0) / 2.0).tanh();
        let diamond_hands = (conviction * retention_signal).sqrt();

        // OrganicGrowth = sqrt(holder_norm × inv_concentration)
        let holder_norm = 1.0 - 1.0 / (1.0 + (1.0 + holder_count as f64 / 100.0).ln());
        let inv_concentration = (1.0 - top10_pct / 100.0).max(0.0);
        let organic_growth = (holder_norm * inv_concentration).sqrt();

        // Longevity = 1 - e^(-age_days/21)
        let age_days = age_hours as f64 / 24.0;
        let longevity = 1.0 - (-age_days / 21.0).exp();

        // K = DH^w1 × OG^w2 × L^w3
        let score = diamond_hands.powf(config.weight_diamond_hands)
            * organic_growth.powf(config.weight_organic_growth)
            * longevity.powf(config.weight_longevity);

        let kscore = KScore {
            score,
            diamond_hands,
            organic_growth,
            longevity,
            wallets_analyzed: total as u32,
            accumulators: acc as u32,
            holders: hld as u32,
            reducers: red as u32,
            extractors: ext as u32,
        };

        tracing::info!(
            mint = %mint,
            k_score = format!("{:.3}", kscore.score),
            diamond_hands = format!("{:.3}", kscore.diamond_hands),
            organic_growth = format!("{:.3}", kscore.organic_growth),
            longevity = format!("{:.3}", kscore.longevity),
            wallets = kscore.wallets_analyzed,
            "K-Score computed"
        );

        (behaviors, kscore)
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
        } else {
            (0, 0.0, 0.0, None, vec![], vec![], false)
        };

        // Detect LP status from holder account owners (burn address = burned, locker = locked)
        let lp_status = if !holder_addresses.is_empty() {
            self.detect_lp_status(&holder_addresses).await
        } else {
            "unsecured".into()
        };

        let age_hours = self.get_token_age_hours(mint_address).await.unwrap_or(0);

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
            top10_pct,
            herfindahl,
            age_hours,
            mint_authority_active,
            freeze_authority_active,
            lp_status,
            supply_burned_pct: None,
            supply_locked_pct: None,
            origin,
            token_standard,
            description,
            created_at: None,
            kscore,
            wallet_behaviors,
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
    address: String,
    ui_amount: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignatureInfo {
    signature: Option<String>,
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
    /// Token account addresses of top holders (for LP burn detection + behavioral).
    holder_addresses: Vec<String>,
    /// ui_amounts per holder (parallel to holder_addresses, for retention calculation).
    holder_balances: Vec<f64>,
}
