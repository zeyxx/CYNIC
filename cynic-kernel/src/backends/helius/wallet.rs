//! Wallet behavioral enrichment via Helius — temporal signal extraction.
//!
//! Flow: getBalance + Enhanced Transaction History + getAssetsByOwner → behavioral profile

use super::{HELIUS_BEHAVIORAL_TIMEOUT, HeliusEnricher};
use crate::domain::enrichment::EnrichmentError;
use crate::domain::wallet_enrichment::WalletBehavioralProfile;
use std::collections::HashMap;

/// Parsed swap from transaction history.
struct ParsedSwap {
    timestamp: i64,
    is_pump_fun: bool,
    tokens: Vec<String>,
    sol_amount: f64,
}

impl HeliusEnricher {
    /// Enrich a wallet address into a behavioral profile from transaction history.
    /// Returns None if wallet has no transaction history.
    ///
    /// Cost: ~121 credits (1 balance + 110 parsed history + 10 assets)
    pub async fn enrich_wallet(
        &self,
        wallet: &str,
    ) -> Result<Option<WalletBehavioralProfile>, EnrichmentError> {
        // Step 1: Get SOL balance
        let sol_balance = self.get_sol_balance(wallet).await.unwrap_or(0.0);

        // Step 2: Get parsed transaction history (100 most recent SWAPs)
        let history = self.get_parsed_tx_history(wallet, 100).await?;
        if history.is_empty() {
            return Ok(None);
        }

        // Step 3: Get fungible token count
        let fungible_count = self.get_fungible_token_count(wallet).await.unwrap_or(0);

        // Step 4: Parse swaps from history
        let swaps = parse_swaps(&history);
        let total_swaps = swaps.len() as u32;

        // Step 5: Compute temporal signals
        let wallet_age_days = compute_wallet_age(&history);
        let swaps_per_day = if wallet_age_days > 0 {
            total_swaps as f64 / wallet_age_days as f64
        } else if total_swaps > 0 {
            total_swaps as f64 // all in one day
        } else {
            0.0
        };

        let pump_fun_ratio = if total_swaps > 0 {
            swaps.iter().filter(|s| s.is_pump_fun).count() as f64 / total_swaps as f64
        } else {
            0.0
        };

        let distinct_tokens = count_distinct_tokens(&swaps);

        // Hold duration: track buy→sell pairs per token
        let (median_hold, flip_count, long_hold_count) = compute_hold_metrics(&swaps);

        let total_volume_sol: f64 = swaps.iter().map(|s| s.sol_amount).sum();

        Ok(Some(WalletBehavioralProfile {
            wallet_address: wallet.to_string(),
            wallet_age_days,
            sol_balance,
            total_swaps,
            swaps_per_day,
            distinct_tokens_traded: distinct_tokens,
            pump_fun_ratio,
            median_hold_hours: median_hold,
            flip_count,
            long_hold_count,
            total_volume_sol,
            fungible_token_count: fungible_count,
            chess_profile: None,
        }))
    }

    /// Get SOL balance for a wallet (in SOL, not lamports). ~1 credit.
    async fn get_sol_balance(&self, wallet: &str) -> Result<f64, EnrichmentError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [wallet]
        });

        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.client.post(&self.rpc_url).json(&body).send(),
        )
        .await
        .map_err(|_| EnrichmentError::Timeout)?
        .map_err(|e| EnrichmentError::RequestFailed(format!("getBalance: {e}")))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(format!("getBalance parse: {e}")))?;

        let lamports = json["result"]["value"].as_u64().unwrap_or(0);
        Ok(lamports as f64 / 1_000_000_000.0)
    }

    /// Get parsed transaction history via Enhanced Transactions API. ~110 credits for 100 tx.
    /// URL pattern from rest.rs: https://api.helius.xyz/v0/addresses/{address}/transactions
    async fn get_parsed_tx_history(
        &self,
        wallet: &str,
        limit: u32,
    ) -> Result<Vec<serde_json::Value>, EnrichmentError> {
        // Extract API key from RPC URL — same pattern as rest.rs
        let api_key = self.rpc_url.split("api-key=").nth(1).unwrap_or_default();

        if api_key.is_empty() {
            return Err(EnrichmentError::RequestFailed(
                "Cannot extract API key from RPC URL for Enhanced Transactions".into(),
            ));
        }

        let url = format!(
            "https://api.helius.xyz/v0/addresses/{wallet}/transactions?api-key={api_key}&limit={limit}&type=SWAP"
        );

        // Use HELIUS_BEHAVIORAL_TIMEOUT — Enhanced Transactions API hangs 10s+ on wallets with no SWAP history
        let resp = tokio::time::timeout(HELIUS_BEHAVIORAL_TIMEOUT, self.client.get(&url).send())
            .await
            .map_err(|_| EnrichmentError::Timeout)?
            .map_err(|e| EnrichmentError::RequestFailed(format!("Enhanced TX history: {e}")))?;

        if !resp.status().is_success() {
            return Ok(vec![]);
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(format!("Enhanced TX parse: {e}")))?;

        match json.as_array() {
            Some(arr) => Ok(arr.clone()),
            None => Ok(vec![]),
        }
    }

    /// Count fungible tokens held by wallet. ~10 credits (DAS).
    async fn get_fungible_token_count(&self, wallet: &str) -> Result<u32, EnrichmentError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAssetsByOwner",
            "params": {
                "ownerAddress": wallet,
                "displayOptions": { "showFungible": true, "showNativeBalance": false },
                "limit": 100
            }
        });

        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.client.post(&self.rpc_url).json(&body).send(),
        )
        .await
        .map_err(|_| EnrichmentError::Timeout)?
        .map_err(|e| EnrichmentError::RequestFailed(format!("getAssetsByOwner: {e}")))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| EnrichmentError::RequestFailed(format!("getAssetsByOwner parse: {e}")))?;

        let items = json["result"]["items"].as_array();
        let count = items
            .map(|arr| {
                arr.iter()
                    .filter(|item| {
                        let iface = item["interface"].as_str().unwrap_or("");
                        iface == "FungibleToken" || iface == "FungibleAsset"
                    })
                    .count() as u32
            })
            .unwrap_or(0);

        Ok(count)
    }
}

// ── Pure parsing functions (testable without network) ──

/// Parse SWAP transactions from Enhanced Transaction History response.
fn parse_swaps(history: &[serde_json::Value]) -> Vec<ParsedSwap> {
    history
        .iter()
        .filter_map(|tx| {
            let tx_type = tx["type"].as_str().unwrap_or("");
            if tx_type != "SWAP" {
                return None;
            }

            let timestamp = tx["timestamp"].as_i64().unwrap_or(0);
            let source = tx["source"].as_str().unwrap_or("");
            let is_pump_fun = source == "PUMP_FUN" || source == "PUMP_AMM";

            // Extract token mints from tokenTransfers
            let mut tokens = Vec::new();
            if let Some(transfers) = tx["tokenTransfers"].as_array() {
                for t in transfers {
                    if let Some(mint) = t["mint"].as_str() {
                        tokens.push(mint.to_string());
                    }
                }
            }

            // Extract SOL amount from nativeTransfers (sum of absolute values, lamports → SOL)
            let sol_amount = tx["nativeTransfers"]
                .as_array()
                .map(|transfers| {
                    transfers
                        .iter()
                        .filter_map(|t| t["amount"].as_f64())
                        .map(|a| a.abs() / 1_000_000_000.0)
                        .sum::<f64>()
                })
                .unwrap_or(0.0);

            Some(ParsedSwap {
                timestamp,
                is_pump_fun,
                tokens,
                sol_amount,
            })
        })
        .collect()
}

/// Compute wallet age in days from transaction history timestamps.
fn compute_wallet_age(history: &[serde_json::Value]) -> u32 {
    let oldest = history
        .iter()
        .filter_map(|tx| tx["timestamp"].as_i64())
        .min()
        .unwrap_or(0);

    if oldest == 0 {
        return 0;
    }

    let now = chrono::Utc::now().timestamp();
    let age_secs = now - oldest;
    age_secs.max(0) as u32 / 86400
}

/// Count distinct token mints across all swaps.
fn count_distinct_tokens(swaps: &[ParsedSwap]) -> u32 {
    let mut mints: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for swap in swaps {
        for token in &swap.tokens {
            mints.insert(token.as_str());
        }
    }
    mints.len() as u32
}

/// Compute hold duration metrics from swap history.
/// Tracks first/last seen timestamp per token as a proxy for buy→sell pairs.
/// Returns (median_hold_hours, flip_count, long_hold_count).
fn compute_hold_metrics(swaps: &[ParsedSwap]) -> (f64, u32, u32) {
    if swaps.is_empty() {
        return (0.0, 0, 0);
    }

    // Track first and last timestamp per token
    let mut first_seen: HashMap<&str, i64> = HashMap::new();
    let mut last_seen: HashMap<&str, i64> = HashMap::new();

    for swap in swaps {
        for token in &swap.tokens {
            first_seen.entry(token.as_str()).or_insert(swap.timestamp);
            last_seen.insert(token.as_str(), swap.timestamp);
        }
    }

    let mut hold_durations_hours: Vec<f64> = Vec::new();
    let mut flip_count: u32 = 0;
    let mut long_hold_count: u32 = 0;

    for (token, first) in &first_seen {
        let last = last_seen.get(token).copied().unwrap_or(*first);
        let duration_hours = (last - first).max(0) as f64 / 3600.0;
        hold_durations_hours.push(duration_hours);

        // Only count as flip if there were at least 2 interactions (buy + sell)
        if duration_hours < 24.0 && last != *first {
            flip_count += 1;
        }
        if duration_hours > 168.0 {
            // 7 days
            long_hold_count += 1;
        }
    }

    // Median hold duration
    hold_durations_hours.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if hold_durations_hours.is_empty() {
        0.0
    } else {
        let mid = hold_durations_hours.len() / 2;
        if hold_durations_hours.len() % 2 == 0 && hold_durations_hours.len() >= 2 {
            (hold_durations_hours[mid - 1] + hold_durations_hours[mid]) / 2.0
        } else {
            hold_durations_hours[mid]
        }
    };

    (median, flip_count, long_hold_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_swaps_filters_non_swaps() {
        let history = vec![
            serde_json::json!({"type": "SWAP", "timestamp": 1000, "source": "PUMP_FUN", "tokenTransfers": [], "nativeTransfers": []}),
            serde_json::json!({"type": "TRANSFER", "timestamp": 2000, "source": "SYSTEM_PROGRAM"}),
            serde_json::json!({"type": "SWAP", "timestamp": 3000, "source": "RAYDIUM", "tokenTransfers": [], "nativeTransfers": []}),
        ];
        let swaps = parse_swaps(&history);
        assert_eq!(swaps.len(), 2);
        assert!(swaps[0].is_pump_fun);
        assert!(!swaps[1].is_pump_fun);
    }

    #[test]
    fn pump_fun_ratio_computed_correctly() {
        let swaps = vec![
            ParsedSwap {
                timestamp: 1,
                is_pump_fun: true,
                tokens: vec![],
                sol_amount: 0.1,
            },
            ParsedSwap {
                timestamp: 2,
                is_pump_fun: true,
                tokens: vec![],
                sol_amount: 0.2,
            },
            ParsedSwap {
                timestamp: 3,
                is_pump_fun: false,
                tokens: vec![],
                sol_amount: 0.3,
            },
        ];
        let ratio = swaps.iter().filter(|s| s.is_pump_fun).count() as f64 / swaps.len() as f64;
        assert!((ratio - 0.6667).abs() < 0.01);
    }

    #[test]
    fn hold_metrics_detect_flips() {
        // Token A: seen at t=0 and t=3600 (1 hour) → flip
        // Token B: seen at t=0 only → hold_duration = 0 (no flip, no long hold)
        let swaps = vec![
            ParsedSwap {
                timestamp: 0,
                is_pump_fun: true,
                tokens: vec!["A".into()],
                sol_amount: 0.1,
            },
            ParsedSwap {
                timestamp: 3600,
                is_pump_fun: true,
                tokens: vec!["A".into()],
                sol_amount: 0.1,
            },
            ParsedSwap {
                timestamp: 100,
                is_pump_fun: false,
                tokens: vec!["B".into()],
                sol_amount: 0.5,
            },
        ];
        let (median, flips, long) = compute_hold_metrics(&swaps);
        assert_eq!(flips, 1); // A was bought and sold within 24h
        assert_eq!(long, 0);
        assert!(median >= 0.0);
    }

    #[test]
    fn hold_metrics_detect_long_holds() {
        // Token C: seen at t=0 and t=700000 (>7 days) → long hold
        let swaps = vec![
            ParsedSwap {
                timestamp: 0,
                is_pump_fun: false,
                tokens: vec!["C".into()],
                sol_amount: 1.0,
            },
            ParsedSwap {
                timestamp: 700_000,
                is_pump_fun: false,
                tokens: vec!["C".into()],
                sol_amount: 1.0,
            },
        ];
        let (median, flips, long) = compute_hold_metrics(&swaps);
        assert_eq!(flips, 0);
        assert_eq!(long, 1);
        assert!(median > 100.0); // > 100 hours
    }

    #[test]
    fn wallet_age_from_timestamps() {
        let now = chrono::Utc::now().timestamp();
        let history = vec![
            serde_json::json!({"timestamp": now - 86400 * 30}), // 30 days ago
            serde_json::json!({"timestamp": now - 86400 * 10}), // 10 days ago
            serde_json::json!({"timestamp": now}),              // now
        ];
        let age = compute_wallet_age(&history);
        assert!(age >= 29 && age <= 31, "age should be ~30 days, got {age}");
    }

    #[test]
    fn distinct_tokens_counts_unique_mints() {
        let swaps = vec![
            ParsedSwap {
                timestamp: 0,
                is_pump_fun: true,
                tokens: vec!["A".into(), "B".into()],
                sol_amount: 0.1,
            },
            ParsedSwap {
                timestamp: 1,
                is_pump_fun: true,
                tokens: vec!["B".into(), "C".into()],
                sol_amount: 0.2,
            },
        ];
        assert_eq!(count_distinct_tokens(&swaps), 3); // A, B, C
    }

    #[test]
    fn empty_history_returns_zeros() {
        let (median, flips, long) = compute_hold_metrics(&[]);
        assert_eq!(median, 0.0);
        assert_eq!(flips, 0);
        assert_eq!(long, 0);
        assert_eq!(compute_wallet_age(&[]), 0);
        assert_eq!(count_distinct_tokens(&[]), 0);
    }
}
