//! Holder resolution, classification, HolderContext, and behavioral analysis (K-Score).

use super::HeliusEnricher;

impl HeliusEnricher {
    /// Detect LP status and compute supply burned/locked percentages.
    /// Checks top holder token accounts' owners against known burn/locker addresses.
    /// Returns (lp_status, burned_pct, locked_pct).
    /// Cost: 1-5 credits (1 getAccountInfo per holder checked).
    pub(super) async fn detect_lp_and_supply_status(
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
    pub(super) async fn classify_holder(&self, token_account: &str) -> String {
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
    pub(super) async fn classify_holders_batch(
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
                |e| tracing::debug!(error = %super::redact_secrets(&e), "classify_holders_batch phase1 request failed"),
            )
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let rpc: serde_json::Value = resp
            .json()
            .await
            .inspect_err(
                |e| tracing::debug!(error = %super::redact_secrets(&e), "classify_holders_batch phase1 parse failed"),
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
        use crate::domain::enrichment::WalletBehavior;

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
            let Ok((total_bought, swaps, hold_secs)) = self
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

            let class = crate::domain::kscore::classify_wallet(retention, config);

            behaviors.push(WalletBehavior {
                class,
                retention_ratio: retention,
                swap_count: swaps,
                hold_time_hours: hold_secs.map(|s| s as f64 / 3600.0),
            });
        }

        // Compute K-Score from behaviors
        let kscore = crate::domain::kscore::compute_kscore(
            &behaviors,
            holder_count,
            top10_pct,
            age_hours,
            config,
        );

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
