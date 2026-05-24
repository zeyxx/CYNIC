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
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenLargestAccounts",
            "params": [mint]
        });

        // Retry up to 3 times on transient "account index service overloaded" errors.
        // Backoff: 1s, 2s, 4s. Total worst-case: 7s additional latency.
        let outer_start = std::time::Instant::now();
        let max_retries = 3u32;
        let mut rpc: RpcResponseWithContext<Vec<LargestAccount>> = RpcResponseWithContext {
            result: None,
            error: None,
        };

        for attempt in 0..=max_retries {
            let start = std::time::Instant::now();
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
                        attempt,
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
                    attempt,
                    latency_ms = start.elapsed().as_millis(),
                    "Helius returned error status — holder concentration unavailable"
                );
                return Ok(None);
            }

            rpc = resp.json().await.map_err(|e| {
                tracing::warn!(
                    method = "getTokenLargestAccounts",
                    mint = %mint,
                    error = %e,
                    attempt,
                    latency_ms = start.elapsed().as_millis(),
                    "Helius response deserialize failed"
                );
                EnrichmentError::RequestFailed(e.to_string())
            })?;

            // Check for JSON-RPC error (returned with HTTP 200)
            if let Some(ref err) = rpc.error {
                let is_overloaded =
                    err.message.contains("overloaded") || err.message.contains("try again");
                if is_overloaded && attempt < max_retries {
                    let backoff = std::time::Duration::from_secs(1 << attempt);
                    tracing::info!(
                        method = "getTokenLargestAccounts",
                        mint = %mint,
                        attempt,
                        rpc_error = %err.message,
                        backoff_ms = backoff.as_millis(),
                        "Account index overloaded — retrying"
                    );
                    tokio::time::sleep(backoff).await;
                    continue;
                }
                tracing::warn!(
                    method = "getTokenLargestAccounts",
                    mint = %mint,
                    rpc_error_code = err.code,
                    rpc_error = %err.message,
                    attempt,
                    "Helius RPC returned JSON-RPC error — holder data unavailable"
                );
                return Ok(None);
            }

            // Success — break out of retry loop
            break;
        }

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

        let latency = outer_start.elapsed().as_millis();
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

    /// DAS fallback for holder concentration when `getTokenLargestAccounts` fails
    /// (e.g., "account index service overloaded"). Uses `getTokenAccounts` (10 credits)
    /// to fetch holders, then sorts by balance locally.
    /// Returns None if DAS also fails. Less precise than RPC (unsorted, needs decimals).
    async fn get_holders_via_das(
        &self,
        mint: &str,
        total_supply: Option<f64>,
        decimals: Option<u8>,
    ) -> Option<HolderConcentration> {
        let api_key = self.rpc_url.split("api-key=").nth(1).unwrap_or_default();
        let url = format!("https://mainnet.helius-rpc.com/?api-key={api_key}");
        let start = std::time::Instant::now();

        // Fetch up to 100 token accounts (covers top holders for most tokens)
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenAccounts",
            "params": {
                "mint": mint,
                "page": 1,
                "limit": 100
            }
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .inspect_err(|e| tracing::warn!(error = %e, "Helius request failed"))
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }

        let rpc: serde_json::Value = resp
            .json()
            .await
            .inspect_err(|e| tracing::warn!(error = %e, "Helius JSON parse failed"))
            .ok()?;
        self.credits
            .record_call(start.elapsed().as_millis(), true, 10);

        let accounts = rpc
            .pointer("/result/token_accounts")
            .and_then(|v| v.as_array())?;

        if accounts.is_empty() {
            return None;
        }

        let dec_factor = 10_f64.powi(decimals.unwrap_or(0) as i32);

        // Parse and sort by balance descending
        let mut holders: Vec<(String, f64)> = accounts
            .iter()
            .filter_map(|a| {
                let owner = a.get("owner")?.as_str()?.to_string();
                let raw_amount = a.get("amount")?.as_u64()? as f64;
                Some((owner, raw_amount / dec_factor))
            })
            .collect();
        holders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let supply_f64 = total_supply
            .filter(|&s| s > 0.0)
            .unwrap_or_else(|| holders.iter().map(|(_, b)| *b).sum::<f64>().max(1.0));

        let mut hhi = 0.0;
        let mut top1_pct = 0.0;
        let mut top10_pct = 0.0;

        for (idx, (_, balance)) in holders.iter().enumerate() {
            let share = balance / supply_f64;
            hhi += share * share;
            if idx == 0 {
                top1_pct = share * 100.0;
            }
            if idx < 10 {
                top10_pct += share * 100.0;
            }
        }

        let holder_addresses: Vec<String> = holders.iter().map(|(a, _)| a.clone()).collect();
        let holder_balances: Vec<f64> = holders.iter().map(|(_, b)| *b).collect();

        tracing::info!(
            mint = %mint,
            holders_fetched = holders.len(),
            top1_pct = format!("{:.2}", top1_pct),
            latency_ms = start.elapsed().as_millis(),
            "DAS getTokenAccounts fallback succeeded"
        );

        Some(HolderConcentration {
            accounts_seen: holders.len() as u64,
            top1_pct,
            top10_pct,
            herfindahl: hhi,
            holder_addresses,
            holder_balances,
        })
    }

    /// Estimate real holder count via DAS getTokenAccounts pagination probing.
    /// Uses exponential page probing (2→10→100→1000) with limit=1 per probe.
    /// Cost: 10-40 credits (1-4 DAS calls). Only called when getTokenLargestAccounts
    /// returned exactly 20 accounts (holder_count_is_exact=false).
    /// Returns estimated lower bound of holder count.
    async fn estimate_holder_count(&self, mint: &str) -> u64 {
        let api_key = self.rpc_url.split("api-key=").nth(1).unwrap_or_default();
        // 3 probes: gives us <2K, 2K-100K, 100K-1M, 1M+ — sufficient for scoring tiers
        let probe_pages: &[u64] = &[2, 100, 1000];
        let mut estimated = 20_u64; // we already know there are >= 20

        for &page in probe_pages {
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getTokenAccounts",
                "params": {
                    "mint": mint,
                    "page": page,
                    "limit": 1
                }
            });

            let url = format!("https://mainnet.helius-rpc.com/?api-key={api_key}");
            let resp = match self.client.post(&url).json(&body).send().await {
                Ok(r) if r.status().is_success() => r,
                _ => break,
            };

            let rpc: serde_json::Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => break,
            };

            self.credits.record_call(0, true, 10); // DAS: 10 credits

            let has_results = rpc
                .pointer("/result/token_accounts")
                .and_then(|v| v.as_array())
                .is_some_and(|a| !a.is_empty());

            if has_results {
                // page N with default page size of 1000 means at least N*1000 holders
                estimated = page * 1000;
                tracing::debug!(mint = %mint, page, estimated, "holder count probe: page has results");
            } else {
                tracing::debug!(mint = %mint, page, estimated, "holder count probe: page empty, stopping");
                break;
            }
        }

        tracing::info!(mint = %mint, estimated_holders = estimated, "holder count estimated via DAS pagination");
        estimated
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
    /// Returns (age_hours, is_exact).
    async fn get_token_age_hours(&self, mint: &str) -> Result<(u64, bool), EnrichmentError> {
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
            return Ok((0, false));
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

        Ok((age_hours, found_creation))
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

    /// Detect LP status and compute supply burned/locked percentages.
    /// Checks top holder token accounts' owners against known burn/locker addresses.
    /// Returns (lp_status, burned_pct, locked_pct).
    /// Cost: 1-5 credits (1 getAccountInfo per holder checked).
    async fn detect_lp_and_supply_status(
        &self,
        holder_addresses: &[String],
        holder_balances: &[f64],
        total_supply: Option<f64>,
    ) -> (String, Option<f64>, Option<f64>) {
        use crate::domain::solana_constants::{BURN_ADDRESSES, LOCKER_PROGRAMS};

        let check_count = holder_addresses.len().min(5);
        let mut lp_status = "unsecured".to_string();
        let mut burned_balance = 0.0_f64;
        let mut locked_balance = 0.0_f64;

        for (i, addr) in holder_addresses[..check_count].iter().enumerate() {
            let Some(owner) = self.resolve_owner(addr).await else {
                continue;
            };
            let balance = holder_balances.get(i).copied().unwrap_or(0.0);

            if BURN_ADDRESSES.contains(&owner.as_str()) {
                burned_balance += balance;
                if lp_status == "unsecured" {
                    tracing::debug!(token_account = %addr, owner = %owner, "LP detected: burned");
                    lp_status = "burned".into();
                }
            } else if LOCKER_PROGRAMS.contains(&owner.as_str()) {
                locked_balance += balance;
                if lp_status == "unsecured" {
                    tracing::debug!(token_account = %addr, owner = %owner, "LP detected: locked");
                    lp_status = "locked".into();
                }
            }
        }

        let (burned_pct, locked_pct) = if let Some(supply) = total_supply
            && supply > 0.0
        {
            (
                if burned_balance > 0.0 {
                    Some(burned_balance / supply * 100.0)
                } else {
                    None
                },
                if locked_balance > 0.0 {
                    Some(locked_balance / supply * 100.0)
                } else {
                    None
                },
            )
        } else {
            (None, None)
        };

        (lp_status, burned_pct, locked_pct)
    }

    /// Classify a token account holder by resolving its owner program.
    /// Known AMM/DEX programs → "lp_pool". Burn addresses → "burn". Lockers → "locker".
    /// Everything else → "wallet". Cost: 1 credit (getAccountInfo).
    async fn classify_holder(&self, token_account: &str) -> String {
        use crate::domain::solana_constants::{AMM_PROGRAMS, BURN_ADDRESSES, LOCKER_PROGRAMS};

        let Some(owner) = self.resolve_owner(token_account).await else {
            return "unknown".into();
        };

        if AMM_PROGRAMS.contains(&owner.as_str()) {
            return "lp_pool".into();
        }

        if BURN_ADDRESSES.contains(&owner.as_str()) {
            return "burn".into();
        }

        if LOCKER_PROGRAMS.contains(&owner.as_str()) {
            return "locker".into();
        }

        "wallet".into()
    }

    /// Classify top token holders by account type (wallet vs smart contract).
    ///
    /// Two-phase approach:
    /// 1. Resolve each token account to its authority (the `info.owner` in parsed token data).
    ///    Quick-classify authorities matching known AMM/burn/locker addresses.
    /// 2. For remaining authorities, call `getMultipleAccounts` to check what program owns them.
    ///    System Program = regular wallet. Anything else = smart contract.
    ///
    /// Cost: 2 credits (2× getMultipleAccounts). Returns HolderContext with per-type breakdown.
    async fn classify_holders_batch(
        &self,
        holder_addresses: &[String],
        holder_balances: &[f64],
        total_supply_ui: f64,
    ) -> Option<crate::domain::enrichment::HolderContext> {
        use crate::domain::enrichment::HolderType;
        use crate::domain::solana_constants::{
            AMM_PROGRAMS as AMM_AUTHORITIES, BURN_ADDRESSES as BURN_AUTHORITIES,
            LOCKER_PROGRAMS as LOCKER_AUTHORITIES, SYSTEM_PROGRAM,
        };

        let count = holder_addresses.len().min(20);
        if count == 0 {
            return None;
        }

        // Phase 1: Resolve token accounts → authorities (preserving order for balance mapping)
        let addrs: Vec<&str> = holder_addresses[..count]
            .iter()
            .map(|s| s.as_str())
            .collect();
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getMultipleAccounts",
            "params": [addrs, {"encoding": "jsonParsed"}]
        });

        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .inspect_err(
                |e| tracing::debug!(error = %e, "classify_holders_batch phase1 request failed"),
            )
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let rpc: serde_json::Value = resp
            .json()
            .await
            .inspect_err(
                |e| tracing::debug!(error = %e, "classify_holders_batch phase1 parse failed"),
            )
            .ok()?;
        let start = std::time::Instant::now();
        self.credits
            .record_call(start.elapsed().as_millis(), true, 1);

        let accounts = rpc.pointer("/result/value").and_then(|v| v.as_array())?;

        // Extract per-position authorities and quick-classify known addresses
        let mut classifications: Vec<HolderType> = Vec::with_capacity(count);
        let mut unknown_indices: Vec<usize> = vec![];
        let mut unknown_authorities: Vec<String> = vec![];

        for (i, acct) in accounts.iter().take(count).enumerate() {
            let authority = acct
                .pointer("/data/parsed/info/owner")
                .and_then(|v| v.as_str());

            match authority {
                Some(auth) if AMM_AUTHORITIES.contains(&auth) => {
                    classifications.push(HolderType::LpPool);
                }
                Some(auth) if BURN_AUTHORITIES.contains(&auth) => {
                    classifications.push(HolderType::Burn);
                }
                Some(auth) if LOCKER_AUTHORITIES.contains(&auth) => {
                    classifications.push(HolderType::Locker);
                }
                Some(auth) => {
                    // Unknown authority — need phase 2 to check its program owner
                    classifications.push(HolderType::Wallet); // placeholder
                    unknown_indices.push(i);
                    unknown_authorities.push(auth.to_string());
                }
                None => {
                    classifications.push(HolderType::Wallet); // fallback
                }
            }
        }

        // Phase 2: For unknown authorities, check what program owns them.
        // System Program → regular wallet. Anything else → smart contract.
        if !unknown_authorities.is_empty() {
            // Deduplicate authorities for the RPC call, then map back
            let mut unique_auths: Vec<String> = unknown_authorities.clone();
            unique_auths.sort();
            unique_auths.dedup();
            let auth_refs: Vec<&str> = unique_auths.iter().map(|s| s.as_str()).collect();

            let body2 = serde_json::json!({
                "jsonrpc": "2.0", "id": 1,
                "method": "getMultipleAccounts",
                "params": [auth_refs, {"encoding": "base64"}]
            });

            if let Ok(resp2) = self.client.post(&self.rpc_url).json(&body2).send().await
                && resp2.status().is_success()
                && let Ok(rpc2) = resp2.json::<serde_json::Value>().await
            {
                self.credits.record_call(0, true, 1);

                // Build program lookup: authority address → program owner
                let mut program_map: std::collections::HashMap<&str, &str> =
                    std::collections::HashMap::new();
                if let Some(accounts2) = rpc2.pointer("/result/value").and_then(|v| v.as_array()) {
                    for (j, acct) in accounts2.iter().enumerate() {
                        if let Some(auth_addr) = unique_auths.get(j) {
                            let program = acct
                                .get("owner")
                                .and_then(|v| v.as_str())
                                .unwrap_or(SYSTEM_PROGRAM);
                            program_map.insert(auth_addr.as_str(), program);
                        }
                    }
                }

                // Reclassify unknown authorities using their program owner
                for (k, idx) in unknown_indices.iter().enumerate() {
                    if let Some(auth) = unknown_authorities.get(k) {
                        let program = program_map
                            .get(auth.as_str())
                            .copied()
                            .unwrap_or(SYSTEM_PROGRAM);
                        classifications[*idx] = if program == SYSTEM_PROGRAM {
                            HolderType::Wallet
                        } else {
                            // Non-system program = smart contract (vesting, DAO, protocol)
                            HolderType::Contract
                        };
                    }
                }
            }
        }

        // Phase 3: Aggregate by type, weighted by balance as % of supply
        let mut lp_sum = 0.0_f64;
        let mut burn_sum = 0.0_f64;
        let mut locker_sum = 0.0_f64;
        let mut contract_sum = 0.0_f64;
        let mut wallet_sum = 0.0_f64;
        let mut classified = 0_u32;

        for (i, holder_type) in classifications.iter().enumerate() {
            let balance = holder_balances.get(i).copied().unwrap_or(0.0);
            classified += 1;
            match holder_type {
                HolderType::LpPool => lp_sum += balance,
                HolderType::Burn => burn_sum += balance,
                HolderType::Locker => locker_sum += balance,
                HolderType::Contract => contract_sum += balance,
                HolderType::Wallet => wallet_sum += balance,
            }
        }

        // Convert raw balances to % of total supply
        let to_pct = |sum: f64| -> f64 {
            if total_supply_ui > 0.0 {
                (sum / total_supply_ui) * 100.0
            } else {
                0.0
            }
        };
        let lp_pct = to_pct(lp_sum);
        let burn_pct = to_pct(burn_sum);
        let locker_pct = to_pct(locker_sum);
        let contract_pct = to_pct(contract_sum);
        let wallet_pct = to_pct(wallet_sum);

        let ctx = crate::domain::enrichment::HolderContext {
            classified,
            lp_pct,
            burn_pct,
            locker_pct,
            contract_pct,
            wallet_pct,
            effective_concentration: wallet_pct,
        };

        tracing::info!(
            classified = ctx.classified,
            wallet = format!("{:.1}%", ctx.wallet_pct),
            contract = format!("{:.1}%", ctx.contract_pct),
            lp = format!("{:.1}%", ctx.lp_pct),
            locker = format!("{:.1}%", ctx.locker_pct),
            burn = format!("{:.1}%", ctx.burn_pct),
            effective = format!("{:.1}%", ctx.effective_concentration),
            "holder context classified"
        );

        Some(ctx)
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

    /// Resolve token account addresses to their owner wallet addresses.
    /// Uses getMultipleAccounts (1 credit) with jsonParsed encoding.
    /// If addresses are already wallet addresses (from DAS fallback), returns them as-is.
    async fn resolve_owners(&self, addresses: &[String]) -> Vec<String> {
        let start = std::time::Instant::now();
        let addrs: Vec<&str> = addresses.iter().take(20).map(|s| s.as_str()).collect();

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getMultipleAccounts",
            "params": [addrs, {"encoding": "jsonParsed"}]
        });

        let resp = match self.client.post(&self.rpc_url).json(&body).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => return addresses.iter().take(20).cloned().collect(),
        };

        let rpc: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return addresses.iter().take(20).cloned().collect(),
        };

        self.credits
            .record_call(start.elapsed().as_millis(), true, 1);

        let accounts = rpc.pointer("/result/value").and_then(|v| v.as_array());

        let Some(accounts) = accounts else {
            return addresses.iter().take(20).cloned().collect();
        };

        let mut owners = Vec::with_capacity(accounts.len());
        for (i, acct) in accounts.iter().enumerate() {
            // Try to extract owner from parsed token account data
            let owner = acct
                .pointer("/data/parsed/info/owner")
                .and_then(|v| v.as_str())
                .map(String::from);

            if let Some(owner) = owner {
                owners.push(owner);
            } else if let Some(addr) = addresses.get(i) {
                // Not a token account (maybe already a wallet) — use as-is
                owners.push(addr.clone());
            }
        }

        // Deduplicate (multiple ATAs can belong to same owner)
        owners.sort();
        owners.dedup();

        tracing::debug!(
            input = addresses.len().min(20),
            resolved = owners.len(),
            latency_ms = start.elapsed().as_millis(),
            "resolved token accounts to owners"
        );

        owners
    }

    /// Resolve identities for holder addresses via Helius Wallet API batch-identity.
    /// Returns identified holders only (unknowns filtered out).
    /// Cost: 100 credits per call (up to 100 addresses).
    async fn batch_identity(
        &self,
        addresses: &[String],
    ) -> Vec<crate::domain::enrichment::HolderIdentity> {
        let api_key = self.rpc_url.split("api-key=").nth(1).unwrap_or_default();
        let url = format!("https://api.helius.xyz/v1/wallet/batch-identity?api-key={api_key}");
        let start = std::time::Instant::now();

        let batch: Vec<&str> = addresses.iter().take(20).map(|s| s.as_str()).collect();
        let body = serde_json::json!({ "addresses": batch });

        let resp = match self.client.post(&url).json(&body).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                tracing::warn!(
                    status = r.status().as_u16(),
                    latency_ms = start.elapsed().as_millis(),
                    "batch-identity HTTP error"
                );
                return vec![];
            }
            Err(e) => {
                tracing::warn!(error = %e, "batch-identity request failed");
                return vec![];
            }
        };

        let items: Vec<serde_json::Value> = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "batch-identity deserialize failed");
                return vec![];
            }
        };

        self.credits
            .record_call(start.elapsed().as_millis(), true, 100);

        let mut identified = Vec::new();
        let mut exchange_count = 0u32;
        let mut protocol_count = 0u32;
        let mut scammer_count = 0u32;
        let mut unknown_count = 0u32;

        for item in &items {
            let address = item
                .get("address")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let name = item.get("name").and_then(|v| v.as_str()).map(String::from);
            let category = item
                .get("category")
                .and_then(|v| v.as_str())
                .map(String::from);
            let entity_type = item.get("type").and_then(|v| v.as_str()).map(String::from);

            if name.is_some() {
                let cat = category.as_deref().unwrap_or("unknown");
                match cat {
                    c if c.contains("Exchange") => exchange_count += 1,
                    c if c.contains("DeFi") || c.contains("Swap") => protocol_count += 1,
                    c if c.contains("Rugger") || c.contains("Scam") || c.contains("Exploit") => {
                        scammer_count += 1;
                    }
                    _ => {}
                }
                identified.push(crate::domain::enrichment::HolderIdentity {
                    address,
                    name,
                    category,
                    entity_type,
                });
            } else {
                unknown_count += 1;
            }
        }

        tracing::info!(
            total = items.len(),
            identified = identified.len(),
            unknown = unknown_count,
            exchanges = exchange_count,
            protocols = protocol_count,
            scammers = scammer_count,
            latency_ms = start.elapsed().as_millis(),
            credits = 100,
            "batch-identity resolved"
        );

        identified
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
            // DAS fallback: getTokenAccounts when getTokenLargestAccounts overloaded.
            // Less precise (unsorted sample) but provides holder data vs nothing.
            tracing::info!(
                mint = %mint_address,
                holders = conc.accounts_seen,
                "Using DAS getTokenAccounts fallback for holder concentration"
            );
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

        // Estimate real holder count via DAS pagination when getTokenLargestAccounts hit the 20-cap.
        // This replaces "20+" with "~100000+" for established tokens like JUP.
        let holder_count = if holder_data_available && holder_count >= 20 {
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
    error: Option<RpcError>,
}

/// JSON-RPC error object returned by Solana RPC on failure.
#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
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
