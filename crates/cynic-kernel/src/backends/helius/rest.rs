//! REST API calls: token-metadata (v0), Enhanced Transactions (v0), batch-identity (v1).

use super::{HELIUS_BEHAVIORAL_TIMEOUT, HeliusEnricher};
use crate::domain::enrichment::EnrichmentError;

impl HeliusEnricher {
    pub(super) async fn get_offchain_metadata(
        &self,
        mint: &str,
    ) -> Result<Option<String>, EnrichmentError> {
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
            .map_err(|e| EnrichmentError::RequestFailed(super::redact_secrets(&e)))?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let data: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(super::redact_secrets(&e)))?;

        let desc = data
            .first()
            .and_then(|t| t.get("offChainMetadata"))
            .and_then(|m| m.get("metadata"))
            .and_then(|m| m.get("description"))
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());

        Ok(desc)
    }

    /// Fetch SWAP transactions for a wallet, filtered to a specific token mint.
    /// Returns (total_bought, swap_count, hold_secs) where hold_secs is the elapsed
    /// time from first buy to last sell (or now if still holding). None if no buy found.
    /// Cost: 50 credits per call (Enhanced Transactions API).
    pub(super) async fn get_wallet_total_bought(
        &self,
        wallet_owner: &str,
        target_mint: &str,
        limit: usize,
    ) -> Result<(f64, u32, Option<i64>), EnrichmentError> {
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
            .map_err(|e| EnrichmentError::RequestFailed(super::redact_secrets(&e)))?;

        if !resp.status().is_success() {
            return Ok((0.0, 0, None));
        }

        self.credits.record_call(0, true, 50); // Enhanced Transactions: 50 credits

        let txs: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(super::redact_secrets(&e)))?;

        let mut total_bought = 0.0_f64;
        let mut swap_count = 0_u32;
        let mut first_buy_ts: Option<i64> = None;
        let mut last_sell_ts: Option<i64> = None;

        // Transactions come newest-first from Helius — iterate in chronological order
        // so first_buy_ts captures the actual first buy, not the most recent one.
        for tx in txs.iter().rev() {
            let ts = tx.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
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
                let from = transfer
                    .get("fromUserAccount")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if to == wallet_owner {
                    total_bought += amount;
                    // First buy = chronologically first inflow
                    if first_buy_ts.is_none() && ts > 0 {
                        first_buy_ts = Some(ts);
                    }
                } else if from == wallet_owner && ts > 0 {
                    // Last sell = chronologically last outflow
                    last_sell_ts = Some(ts);
                }
                swap_count += 1;
            }
        }

        // Hold time: from first buy to last sell (closed) or to now (still holding).
        let hold_secs = first_buy_ts.map(|buy| {
            let end = last_sell_ts.unwrap_or_else(|| chrono::Utc::now().timestamp());
            (end - buy).max(0)
        });

        Ok((total_bought, swap_count, hold_secs))
    }

    /// Resolve identities for holder addresses via Helius Wallet API batch-identity.
    /// Returns identified holders only (unknowns filtered out).
    /// Cost: 100 credits per call (up to 100 addresses).
    pub(super) async fn batch_identity(
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
                tracing::warn!(error = %super::redact_secrets(&e), "batch-identity request failed");
                return vec![];
            }
        };

        let items: Vec<serde_json::Value> = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %super::redact_secrets(&e), "batch-identity deserialize failed");
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
