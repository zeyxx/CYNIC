//! JSON-RPC calls: getAsset, getTokenLargestAccounts, getTokenAccounts (DAS),
//! estimate_holder_count, getSignaturesForAddress, getAccountInfo, getMultipleAccounts.

use super::{
    HeliusAsset, HeliusEnricher, HolderConcentration, LargestAccount, RpcResponse,
    RpcResponseWithContext, SignatureInfo,
};
use crate::domain::enrichment::EnrichmentError;

impl HeliusEnricher {
    pub(super) async fn get_asset(
        &self,
        mint: &str,
    ) -> Result<Option<HeliusAsset>, EnrichmentError> {
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
    pub(super) async fn get_largest_accounts(
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
    pub(super) async fn get_holders_via_das(
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
    pub(super) async fn estimate_holder_count(&self, mint: &str) -> u64 {
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
    pub(super) async fn get_token_age_hours(
        &self,
        mint: &str,
    ) -> Result<(u64, bool), EnrichmentError> {
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
    pub(crate) async fn resolve_owner(&self, token_account: &str) -> Option<String> {
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

    /// Resolve token account addresses to their owner wallet addresses.
    /// Uses getMultipleAccounts (1 credit) with jsonParsed encoding.
    /// If addresses are already wallet addresses (from DAS fallback), returns them as-is.
    pub(super) async fn resolve_owners(&self, addresses: &[String]) -> Vec<String> {
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
}
