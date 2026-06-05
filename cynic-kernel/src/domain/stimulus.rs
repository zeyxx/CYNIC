use crate::domain::enrichment::TokenData;
use crate::domain::sanitize::sanitize_stimulus_field;

// INTEGRATION: B&C + CYNIC Personality Cards
//
// **Entry Point**: When S. completes J6-7 (Mint Permit Service):
// 1. Client signs: {nonce, personality_card, wallet_address}
// 2. POST /mint-permit → S.'s backend
// 3. S. calls CYNIC: POST /judge with personality data
// 4. CYNIC formats personality signals into a stimulus
// 5. Dogs evaluate: FIDELITY (genuine human play?), PHI (signal coherence), etc.
// 6. Verdict returned to S.'s backend (async or sync, TBD)
// 7. If HOWL/WAG: proceed to Arweave upload + Metaplex mint
// 8. If BARK: reject, nonce stays in LRU, 1/wallet/hour enforced
//
// **Scope Questions** (waiting for S. on Slack):
// - Pre-mint validation (sync, blocks mint) or post-mint audit (async)?
// - Confidence threshold: require Dogs verdict >= φ⁻¹ (0.618) or is Dogs consensus enough?
// - Fallback if CYNIC is down: block mint or proceed?
//
// **Chess Domain** (cynic-kernel/domains/chess.md):
// - Already optimized for move/opening/position evaluation
// - Personality card fits: archetype = chess signature, signals = game patterns
// - Dogs will score: is this wallet's pattern genuine + sybil-resistant?

/// Build a structured token analysis stimulus from on-chain metrics.
///
/// The caller (screener, API consumer) fetches data from Helius/DexScreener/etc.
/// This function formats it so Dogs can judge rigorously with minimal disagreement.
pub fn build_token_stimulus(data: &TokenData) -> String {
    let mut s = String::with_capacity(1500);

    s.push_str("[DOMAIN: token-analysis]\n\n");

    // ── Metrics: raw facts, no interpretation ──
    s.push_str("[METRICS]\n");
    s.push_str(&format!("mint: {}\n", data.mint));
    if let Some(ref name) = data.name {
        let safe = sanitize_stimulus_field(name, 64);
        s.push_str(&format!("name: {safe}\n"));
    }
    if let Some(ref symbol) = data.symbol {
        let safe = sanitize_stimulus_field(symbol, 16);
        s.push_str(&format!("symbol: {safe}\n"));
    }
    if data.holder_data_available {
        if data.holder_count_is_exact {
            s.push_str(&format!("holders: {} (exact)\n", data.holder_count));
        } else {
            s.push_str(&format!(
                "holders: {}+ (top accounts analyzed, real count likely higher)\n",
                data.holder_count
            ));
        }
        s.push_str(&format!("top_1_wallet_pct: {:.2}%\n", data.top1_pct));
        if data.top1_type != "unknown" && data.top1_type != "wallet" {
            s.push_str(&format!(
                "top_1_wallet_type: {} (not a retail whale)\n",
                data.top1_type
            ));
        }
        s.push_str(&format!("top_10_wallets_pct: {:.2}%\n", data.top10_pct));
        if let Some(hhi) = data.herfindahl {
            s.push_str(&format!("herfindahl_index: {hhi:.3}\n"));
        }
    } else {
        s.push_str("holders: UNAVAILABLE (RPC degraded — concentration metrics omitted)\n");
    }
    if data.age_is_exact {
        s.push_str(&format!("age_hours: {}\n", data.age_hours));
    } else {
        s.push_str(&format!(
            "age_hours: >={} (estimated, active token with >5000 txs)\n",
            data.age_hours
        ));
    }
    s.push_str(&format!(
        "mint_authority: {}\n",
        if data.mint_authority_active {
            "ACTIVE (can mint more tokens)"
        } else {
            "REVOKED (supply is fixed)"
        }
    ));
    s.push_str(&format!(
        "freeze_authority: {}\n",
        if data.freeze_authority_active {
            "ACTIVE (can freeze wallets)"
        } else {
            "REVOKED (wallets are free)"
        }
    ));
    s.push_str(&format!(
        "lp_secured: {}\n",
        match data.lp_status.as_str() {
            "burned" => "YES — LP tokens burned (permanent liquidity)",
            "locked" => "PARTIAL — LP tokens locked (temporary)",
            "unsecured" => "NO — LP tokens in creator wallet (can rug)",
            "unknown" => "UNKNOWN — holder data unavailable (RPC degraded)",
            other => other,
        }
    ));
    if let Some(burned) = data.supply_burned_pct {
        s.push_str(&format!("supply_burned_pct: {burned:.2}%\n"));
    }
    if let Some(locked) = data.supply_locked_pct {
        s.push_str(&format!("supply_locked_pct: {locked:.2}%\n"));
    }
    if let Some(ref origin) = data.origin {
        let safe = sanitize_stimulus_field(origin, 32);
        s.push_str(&format!("origin: {safe}\n"));
    }
    // ── Market data: price and derived metrics ──
    if let Some(price) = data.price_usd
        && price > 0.0
    {
        s.push_str(&format!("price_usd: ${price:.6}\n"));
        // Compute market cap if supply available
        if let (Some(supply), Some(decimals)) = (data.supply, data.decimals) {
            let human_supply = supply as f64 / 10_f64.powi(decimals as i32);
            let mcap = human_supply * price;
            if mcap >= 1_000_000_000.0 {
                s.push_str(&format!("market_cap_usd: ${:.2}B\n", mcap / 1e9));
            } else if mcap >= 1_000_000.0 {
                s.push_str(&format!("market_cap_usd: ${:.2}M\n", mcap / 1e6));
            } else if mcap >= 1_000.0 {
                s.push_str(&format!("market_cap_usd: ${:.0}K\n", mcap / 1e3));
            } else {
                s.push_str(&format!("market_cap_usd: ${mcap:.2}\n"));
            }
        }
    }
    if let Some(volume) = data.volume_24h_usd
        && volume > 0.0
    {
        if volume >= 1_000_000.0 {
            s.push_str(&format!("volume_24h_usd: ${:.2}M\n", volume / 1e6));
        } else if volume >= 1_000.0 {
            s.push_str(&format!("volume_24h_usd: ${:.0}K\n", volume / 1e3));
        } else {
            s.push_str(&format!("volume_24h_usd: ${volume:.2}\n"));
        }
    }
    if let Some(liq) = data.liquidity_usd
        && liq > 0.0
    {
        if liq >= 1_000_000.0 {
            s.push_str(&format!("liquidity_usd: ${:.2}M\n", liq / 1e6));
        } else if liq >= 1_000.0 {
            s.push_str(&format!("liquidity_usd: ${:.0}K\n", liq / 1e3));
        } else {
            s.push_str(&format!("liquidity_usd: ${liq:.2}\n"));
        }
    }
    if let Some(ref std) = data.token_standard {
        s.push_str(&format!("standard: {std}\n"));
    }

    // ── Behavioral signals (K-Score) ──
    if let Some(ref ks) = data.kscore {
        s.push_str("\n[BEHAVIORAL]\n");
        s.push_str(&format!("k_score: {:.3}\n", ks.score));
        s.push_str(&format!(
            "diamond_hands: {:.3} (conviction of top holders)\n",
            ks.diamond_hands
        ));
        s.push_str(&format!(
            "organic_growth: {:.3} (distribution quality)\n",
            ks.organic_growth
        ));
        s.push_str(&format!(
            "longevity: {:.3} (age-adjusted survival)\n",
            ks.longevity
        ));
        s.push_str(&format!(
            "wallet_breakdown: {} analyzed \u{2014} {} accumulators, {} holders, {} reducers, {} extractors\n",
            ks.wallets_analyzed, ks.accumulators, ks.holders, ks.reducers, ks.extractors
        ));
        if let Some(mh) = ks.median_hold_hours {
            s.push_str(&format!(
                "median_hold_hours: {:.1} ({:.1}d — time from first buy to last sell or now)\n",
                mh,
                mh / 24.0
            ));
        }
    }

    // ── Buy/Sell Divergence (from token profiler snapshot) ──
    if let Some(ref div_class) = data.divergence_class {
        s.push_str("\n[BUY/SELL DIVERGENCE]\n");
        if let Some(ratio) = data.buy_sell_ratio {
            s.push_str(&format!("buy_sell_ratio: {ratio:.3}\n"));
        }
        s.push_str(&format!("divergence_class: {div_class}\n"));
        if let Some(pct) = data.percentile_divergence {
            s.push_str(&format!(
                "percentile: {pct}th (1-100, higher = more unusual)\n",
            ));
        }
        match div_class.as_str() {
            "EARLY_ACCUM" => s.push_str("interpretation: Conviction declining but buy-side holders increasing — possible recovery phase\n"),
            "DISTRIBUTION" => s.push_str("interpretation: Conviction high but sell-side dominates — possible exit phase\n"),
            "STRONG_HOLD" => s.push_str("interpretation: Both conviction and buy-side high — consensus accumulation\n"),
            _ => {}
        }
    }

    // ── Holder context: account type breakdown (wallet vs contract) ──
    if let Some(ref ctx) = data.holder_context {
        s.push_str("\n[HOLDER CONTEXT]\n");
        let total_analyzed =
            ctx.lp_pct + ctx.burn_pct + ctx.locker_pct + ctx.contract_pct + ctx.wallet_pct;
        s.push_str(&format!(
            "top_{}_analyzed: {:.1}% of supply\n",
            ctx.classified, total_analyzed
        ));
        if ctx.lp_pct > 0.0 {
            s.push_str(&format!(
                "  lp_pools: {:.1}% — DEX liquidity, market-making\n",
                ctx.lp_pct
            ));
        }
        if ctx.burn_pct > 0.0 {
            s.push_str(&format!(
                "  burned: {:.1}% — supply permanently removed\n",
                ctx.burn_pct
            ));
        }
        if ctx.locker_pct > 0.0 {
            s.push_str(&format!(
                "  locked: {:.1}% — tokens in lock/vesting contracts, not freely tradeable\n",
                ctx.locker_pct
            ));
        }
        if ctx.oracle_pct > 0.0 {
            s.push_str(&format!(
                "  oracles: {:.1}% — tokens held by oracle programs (Pyth, Switchboard)\n",
                ctx.oracle_pct
            ));
        }
        if ctx.infra_pct > 0.0 {
            s.push_str(&format!(
                "  infra: {:.1}% — tokens held by infrastructure programs (Jito, Compute Budget)\n",
                ctx.infra_pct
            ));
        }
        if ctx.contract_pct > 0.0 {
            s.push_str(&format!(
                "  contracts: {:.1}% — tokens held by other smart contracts (DAO, protocol, unknown), not freely tradeable\n",
                ctx.contract_pct
            ));
        }
        if ctx.wallet_pct > 0.0 {
            s.push_str(&format!(
                "  wallets: {:.1}% — freely tradeable by individual holders\n",
                ctx.wallet_pct
            ));
        }
        s.push_str(&format!(
            "effective_wallet_concentration: {:.1}%\n",
            ctx.effective_concentration
        ));
        // Contextual note when institutional holdings are significant
        let institutional =
            ctx.locker_pct + ctx.contract_pct + ctx.lp_pct + ctx.burn_pct + ctx.oracle_pct + ctx.infra_pct;
        if institutional > 30.0 {
            s.push_str(&format!(
                "note: High raw concentration ({:.0}%) driven by institutional/programmatic holdings ({:.0}%). Effective retail concentration is {:.1}%.\n",
                total_analyzed, institutional, ctx.effective_concentration
            ));
        }
    }

    // ── Holder identities (from Helius Wallet API) ──
    if !data.holder_identities.is_empty() {
        s.push_str("\n[HOLDER IDENTITIES]\n");
        for id in &data.holder_identities {
            let name = id.name.as_deref().unwrap_or("?");
            let cat = id.category.as_deref().unwrap_or("unknown");
            s.push_str(&format!("{}: {} ({})\n", &id.address[..8], name, cat));
        }
        let total = data.holder_identities.len();
        s.push_str(&format!(
            "identified: {total} of top holders. Unknown holders not listed.\n"
        ));
        // Signal summary for Dogs
        let exchanges: Vec<_> = data
            .holder_identities
            .iter()
            .filter(|i| {
                i.category
                    .as_deref()
                    .is_some_and(|c| c.contains("Exchange"))
            })
            .collect();
        let scammers: Vec<_> = data
            .holder_identities
            .iter()
            .filter(|i| {
                i.category.as_deref().is_some_and(|c| {
                    c.contains("Rugger") || c.contains("Scam") || c.contains("Exploit")
                })
            })
            .collect();
        if !exchanges.is_empty() {
            s.push_str(&format!(
                "exchanges_in_holders: {} (institutional backing signal)\n",
                exchanges.len()
            ));
        }
        if !scammers.is_empty() {
            s.push_str(&format!(
                "WARNING: {} known scammer/rugger in holders\n",
                scammers.len()
            ));
        }
    }

    // ── Trajectory: conviction decay curve from daily cron (temporal signal) ──
    if let Some(ref tclass) = data.trajectory_class {
        s.push_str("\n[TRAJECTORY]\n");
        s.push_str(&format!("class: {tclass}\n"));
        if let Some(decay) = data.trajectory_decay {
            s.push_str(&format!("decay: {decay:.4}\n"));
        }
        match tclass.as_str() {
            "DEAD" => {
                s.push_str("interpretation: 30d conviction < 1% — virtually no holders stayed\n")
            }
            "DYING" => s.push_str(
                "interpretation: Steep conviction decay (>30%) — holders actively leaving\n",
            ),
            "DECLINING" => s.push_str(
                "interpretation: Moderate conviction decay (15-30%) — gradual holder attrition\n",
            ),
            "STABLE" => s.push_str(
                "interpretation: Minimal conviction decay (<15%) — diamond hands dominant\n",
            ),
            _ => {}
        }
    }

    // ── Baselines: what "normal" looks like ──
    s.push_str("\n[BASELINES]\n");
    s.push_str("healthy_token: holders>100, top_1<15%, herfindahl<0.15, age>30d, mint_authority=revoked, lp=burned, market_cap>$1M, liquidity>$100K\n");
    s.push_str(
        "moderate_risk: holders 20-100, top_1 15-40%, age 1-30d, lp=locked, market_cap $10K-$1M\n",
    );
    s.push_str(
        "high_risk_rug: holders<20, top_1>50%, age<24h, mint_authority=active, lp=unsecured, liquidity<$1K\n",
    );
    s.push_str("k_score_baseline: healthy>0.5, moderate 0.3-0.5, rug<0.3. diamond_hands dominates (retention).\n");
    s.push_str("note: 98.6% of pump.fun tokens are rug pulls (Solidus Labs 2025). Baseline for new tokens is skepticism, not trust.\n");

    // ── Axiom evidence: what to evaluate per axiom ──
    s.push_str("\n[AXIOM EVIDENCE]\n");
    s.push_str("FIDELITY: Does the on-chain state match what a legitimate project would show? Is the token faithful to its claimed purpose?\n");
    s.push_str("PHI: Is the holder distribution proportional? Use effective_wallet_concentration (from [HOLDER CONTEXT]) when available — institutional holdings (vesting, LP, locks) don't indicate manipulation.\n");
    s.push_str("VERIFY: Can these metrics be independently verified on-chain? Are there verifiable red flags or green flags?\n");
    s.push_str("CULTURE: Does this token follow established Solana token standards? Is the authority model consistent with good practices?\n");
    s.push_str("BURN: Is the token efficiently structured? Burned supply, minimal waste, no unnecessary authorities retained?\n");
    s.push_str("SOVEREIGNTY: Is control distributed or concentrated? Tokens in vesting/lock contracts are scheduled for release — concentration via contracts ≠ concentration via whales.\n");

    // ── Question ──
    s.push_str("\n[QUESTION]\n");
    s.push_str("Based on the on-chain metrics above, evaluate this token's legitimacy and risk level. Score each axiom from 0.05 to 0.618.\n");

    s
}

/// Build a structured wallet judgment stimulus from game history metrics.
///
/// Used when CYNIC validates chess wallet authenticity for Personality Card anti-Sybil gate.
/// The WalletProfile contains on-chain game history: games played, archetype consistency,
/// temporal distribution, and Sybil risk markers.
pub fn build_wallet_stimulus(profile: &crate::domain::wallet_judgment::WalletProfile) -> String {
    let mut s = String::with_capacity(1000);
    s.push_str("[DOMAIN: wallet-judgment]\n\n");
    s.push_str("[METRICS]\n");
    s.push_str(&format!("wallet: {}\n", profile.wallet_address));
    s.push_str(&format!("games_completed: {}\n", profile.games_completed));
    s.push_str(&format!("wallet_age_days: {}\n", profile.wallet_age_days));
    s.push_str(&format!(
        "archetype_consistency: {:.2}\n",
        profile.archetype_consistency
    ));
    s.push_str(&format!(
        "avg_game_duration_secs: {}\n",
        profile.average_game_duration
    ));
    s.push_str(&format!(
        "duration_variance_cv: {:.2}\n",
        profile.duration_variance
    ));
    s.push_str(&format!(
        "suspicious_cluster: {}\n",
        profile.suspicious_cluster
    ));
    s.push_str(&format!("replay_risk: {}\n", profile.replay_risk));

    s.push_str("\n[PERSONALITY SIGNALS]\n");
    s.push_str(&format!(
        "tactical_complexity: {:.2} (High = tactical risk-taker)\n",
        profile.tactical_complexity
    ));
    s.push_str(&format!(
        "engine_adherence: {:.2} (High = precise, potential engine; Low = intuitive/blunderer)\n",
        profile.engine_adherence
    ));
    s.push_str(&format!(
        "opening_theory_depth: {:.2} (High = studied/Culture-aware)\n",
        profile.opening_theory_depth
    ));
    s.push_str(&format!(
        "blitz_speed_ratio: {:.2} (High = efficient/fast decisions)\n",
        profile.blitz_speed_ratio
    ));
    s.push_str(&format!(
        "endgame_accuracy: {:.2} (High = structural precision/Phi)\n",
        profile.endgame_accuracy
    ));

    s.push_str("\n[GATES]\n");
    s.push_str("gate_1_min_games: 5 games required\n");
    s.push_str("gate_2_sybil: suspicious_cluster=false AND replay_risk=false required\n");
    s.push_str("\n[AXIOM EVIDENCE]\n");
    s.push_str(
        "FIDELITY: Is the archetype consistent? Does engine_adherence suggest human play (0.4-0.8) or bot ( >0.9)?\n",
    );
    s.push_str("PHI: Structural harmony in time and endgame_accuracy? Positional coherence?\n");
    s.push_str("VERIFY: Are timestamps verifiable on-chain? Does opening_theory_depth match known books?\n");
    s.push_str(
        "CULTURE: Does the wallet honor chess traditions (theory depth)? Depth of engagement?\n",
    );
    s.push_str("BURN: Is play efficient? blitz_speed_ratio vs accuracy? No time wasted?\n");
    s.push_str(
        "SOVEREIGNTY: Decision autonomy? Tactical complexity indicates high agency choices.\n",
    );
    s.push_str("\n[QUESTION]\n");
    s.push_str(
        "Evaluate this chess wallet for authentic human play and crystallize its personality signature. Score each axiom 0.05-0.618.\n",
    );
    s
}

/// Build a structured phone number judgment stimulus from community report metrics.
///
/// Used when CYNIC evaluates a phone number for CallShield spam/scam classification.
/// The PhoneData contains aggregated community reports, label distributions, and
/// temporal patterns. The score represents spam likelihood: 0.0 = safe, 1.0 = confirmed scam.
pub fn build_phone_stimulus(data: &crate::domain::phone_number::PhoneData) -> String {
    let mut s = String::with_capacity(800);

    s.push_str("[DOMAIN: phone-number]\n\n");

    // ── Metrics: raw facts, no interpretation ──
    s.push_str("[METRICS]\n");
    s.push_str(&format!("number: {}\n", data.number));
    s.push_str(&format!("country_code: {}\n", data.country_code));
    s.push_str(&format!("total_events: {}\n", data.total_events));
    s.push_str(&format!("reporter_count: {}\n", data.reporter_count));
    s.push_str(&format!(
        "mean_reporter_trust: {:.3}\n",
        data.mean_reporter_trust
    ));
    s.push_str(&format!("age_days: {}\n", data.age_days));
    s.push_str(&format!(
        "days_since_last_report: {}\n",
        data.days_since_last_report
    ));
    s.push_str(&format!(
        "contestation_count: {}\n",
        data.contestation_count
    ));
    s.push_str(&format!(
        "owner_verified: {}\n",
        if data.owner_verified { "YES" } else { "NO" }
    ));

    // ── Label distribution ──
    s.push_str(&format!(
        "labels: legitimate={} nuisance={} scam={} unknown={}\n",
        data.label_distribution.legitimate,
        data.label_distribution.nuisance,
        data.label_distribution.scam,
        data.label_distribution.unknown,
    ));
    s.push_str(&format!(
        "spam_score: {:.3}\n",
        data.label_distribution.spam_score()
    ));

    // ── Challenge data ──
    match data.challenge_pass_rate {
        Some(rate) => s.push_str(&format!("challenge_pass_rate: {:.1}%\n", rate * 100.0)),
        None => s.push_str("challenge_pass_rate: N/A (never challenged)\n"),
    }

    // ── Baselines ──
    s.push_str("\n[BASELINES]\n");
    s.push_str(
        "safe: spam_score<0.2, reporter_count>5, mean_reporter_trust>0.7, contestations=0\n",
    );
    s.push_str("nuisance: spam_score 0.2-0.6, moderate reporter activity, low trust\n");
    s.push_str("scam: spam_score>0.6, high reporter_count, low challenge_pass_rate\n");
    s.push_str("note: Reporter count and trust are weighted — 1 trusted reporter outweighs 10 untrusted ones.\n");

    // ── Axiom evidence ──
    s.push_str("\n[AXIOM EVIDENCE]\n");

    // FIDELITY: weighted trust signal from reporter pool
    let fidelity_signal = if data.reporter_count == 0 {
        "no reporters — no fidelity signal".to_string()
    } else {
        format!(
            "{} reporters, mean trust {:.2} — {}",
            data.reporter_count,
            data.mean_reporter_trust,
            if data.mean_reporter_trust >= 0.7 {
                "high-confidence community signal"
            } else if data.mean_reporter_trust >= 0.4 {
                "moderate-confidence community signal"
            } else {
                "low-trust reporter pool"
            }
        )
    };
    s.push_str(&format!(
        "FIDELITY: {fidelity_signal}. Owner verified: {}.\n",
        if data.owner_verified { "YES" } else { "NO" }
    ));
    s.push_str("PHI: Is the label distribution proportional? Balanced reports across categories suggest genuine community signal.\n");
    s.push_str("VERIFY: Are report counts consistent with phone age? High reports on a new number = coordinated campaign.\n");
    s.push_str("CULTURE: Does the number match expected country-code patterns? Spoofed numbers often mismatch geography.\n");
    s.push_str("BURN: Is activity efficient? Bursts of reports in short windows may indicate automated flooding.\n");
    s.push_str("SOVEREIGNTY: Can the owner contest? Low contestation_count with high scam reports = no recourse.\n");

    // ── Question ──
    s.push_str("\n[QUESTION]\n");
    s.push_str("Based on the community report metrics above, evaluate this phone number's spam/scam likelihood. Score each axiom from 0.05 to 0.618.\n");

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_stimulus_contains_all_sections() {
        let data = TokenData {
            mint: "9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump".into(),
            name: Some("ASDF".into()),
            symbol: Some("ASDF".into()),
            supply: None,
            decimals: None,
            price_usd: None,
            holder_count: 20,
            holder_data_available: true,
            top1_pct: 94.0,
            top10_pct: 99.0,
            herfindahl: Some(0.88),
            age_hours: 3,
            mint_authority_active: true,
            freeze_authority_active: false,
            lp_status: "unsecured".into(),
            supply_burned_pct: Some(10.5),
            supply_locked_pct: None,
            origin: Some("pump.fun".into()),
            token_standard: None,
            description: None,
            created_at: None,
            ..Default::default()
        };

        let stimulus = build_token_stimulus(&data);

        assert!(stimulus.contains("[DOMAIN: token-analysis]"));
        assert!(stimulus.contains("[METRICS]"));
        assert!(stimulus.contains("[BASELINES]"));
        assert!(stimulus.contains("[AXIOM EVIDENCE]"));
        assert!(stimulus.contains("[QUESTION]"));
        assert!(stimulus.contains("holders: 20"));
        assert!(stimulus.contains("top_1_wallet_pct: 94.00%"));
        assert!(stimulus.contains("ACTIVE (can mint more tokens)"));
        assert!(stimulus.contains("NO — LP tokens in creator wallet"));
        assert!(stimulus.contains("98.6%"));
    }

    #[test]
    fn token_stimulus_with_buy_sell_divergence() {
        let data = TokenData {
            mint: "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump".into(),
            name: Some("Fartcoin".into()),
            symbol: Some("FART".into()),
            supply: Some(1_000_000_000),
            decimals: Some(6),
            price_usd: Some(0.00001),
            holder_count: 320,
            holder_data_available: true,
            top1_pct: 5.2,
            top10_pct: 12.1,
            herfindahl: Some(0.045),
            age_hours: 13800, // ~575 days
            mint_authority_active: false,
            freeze_authority_active: false,
            lp_status: "burned".into(),
            supply_burned_pct: Some(0.0),
            supply_locked_pct: None,
            origin: Some("pump.fun".into()),
            token_standard: None,
            description: None,
            created_at: None,
            buy_sell_ratio: Some(0.0),
            divergence_class: Some("DISTRIBUTION".into()),
            percentile_divergence: Some(67),
            ..Default::default()
        };

        let stimulus = build_token_stimulus(&data);

        assert!(stimulus.contains("[BUY/SELL DIVERGENCE]"));
        assert!(stimulus.contains("buy_sell_ratio: 0.000"));
        assert!(stimulus.contains("divergence_class: DISTRIBUTION"));
        assert!(stimulus.contains("percentile: 67th"));
        assert!(stimulus.contains("interpretation: Conviction high but sell-side dominates"));
    }

    #[test]
    fn token_stimulus_optional_fields_omitted() {
        let data = TokenData {
            mint: "test".into(),
            name: None,
            symbol: None,
            supply: None,
            decimals: None,
            price_usd: None,
            holder_count: 5,
            top1_pct: 99.0,
            top10_pct: 100.0,
            herfindahl: None,
            age_hours: 1,
            mint_authority_active: true,
            freeze_authority_active: true,
            lp_status: "unsecured".into(),
            supply_burned_pct: None,
            supply_locked_pct: None,
            origin: None,
            token_standard: None,
            description: None,
            created_at: None,
            ..Default::default()
        };

        let stimulus = build_token_stimulus(&data);
        assert!(!stimulus.contains("name:"));
        assert!(!stimulus.contains("herfindahl_index:"));
        assert!(!stimulus.contains("supply_burned_pct:"));
    }

    // ── Calibration test: known-rug should produce consistent low stimulus ──
    #[test]
    fn known_rug_stimulus_contains_red_flags() {
        let data = TokenData {
            mint: "rugpull123".into(),
            name: Some("SCAM".into()),
            symbol: Some("RUG".into()),
            supply: None,
            decimals: None,
            price_usd: None,
            holder_count: 3,
            holder_data_available: true,
            top1_pct: 99.0,
            top10_pct: 100.0,
            herfindahl: Some(0.98),
            age_hours: 1,
            mint_authority_active: true,
            freeze_authority_active: true,
            lp_status: "unsecured".into(),
            supply_burned_pct: Some(0.0),
            supply_locked_pct: Some(0.0),
            origin: Some("pump.fun".into()),
            token_standard: None,
            description: None,
            created_at: None,
            ..Default::default()
        };

        let stimulus = build_token_stimulus(&data);
        // Every metric should map to "high_risk_rug" baseline
        assert!(stimulus.contains("holders: 3"));
        assert!(stimulus.contains("99.00%"));
        assert!(stimulus.contains("ACTIVE (can mint"));
        assert!(stimulus.contains("ACTIVE (can freeze"));
        assert!(stimulus.contains("NO — LP tokens in creator wallet"));
    }

    #[test]
    fn test_build_phone_stimulus_structure() {
        use crate::domain::phone_number::{LabelDistribution, PhoneData};
        let data = PhoneData {
            number: "+33612345678".to_string(),
            country_code: "FR".to_string(),
            total_events: 47,
            label_distribution: LabelDistribution {
                legitimate: 3,
                nuisance: 30,
                scam: 12,
                unknown: 2,
            },
            reporter_count: 35,
            mean_reporter_trust: 0.65,
            age_days: 14,
            days_since_last_report: 1,
            challenge_pass_rate: Some(0.15),
            contestation_count: 0,
            owner_verified: false,
        };
        let stimulus = build_phone_stimulus(&data);
        assert!(stimulus.contains("[DOMAIN: phone-number]"));
        assert!(stimulus.contains("+33612345678"));
        assert!(stimulus.contains("total_events: 47"));
        assert!(stimulus.contains("spam_score:"));
        assert!(stimulus.contains("challenge_pass_rate: 15.0%"));
        assert!(stimulus.contains("[QUESTION]"));
    }

    #[test]
    fn token_stimulus_with_holder_context() {
        use crate::domain::enrichment::HolderContext;

        let data = TokenData {
            mint: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN".into(),
            name: Some("Jupiter".into()),
            symbol: Some("JUP".into()),
            supply: Some(6_863_982_190_903_847),
            decimals: Some(6),
            price_usd: Some(0.78),
            holder_count: 250_000,
            holder_data_available: true,
            top1_pct: 60.0,
            top10_pct: 85.0,
            herfindahl: Some(0.45),
            age_hours: 12000,
            mint_authority_active: false,
            freeze_authority_active: false,
            lp_status: "burned".into(),
            supply_burned_pct: Some(0.0),
            supply_locked_pct: Some(0.0),
            origin: None,
            token_standard: Some("Fungible".into()),
            description: None,
            created_at: None,
            holder_context: Some(HolderContext {
                classified: 20,
                lp_pct: 15.2,
                burn_pct: 0.0,
                locker_pct: 0.0,
                oracle_pct: 5.0,
                infra_pct: 1.0,
                contract_pct: 54.1,
                wallet_pct: 9.7,
                effective_concentration: 9.7,
            }),
            ..Default::default()
        };

        let stimulus = build_token_stimulus(&data);

        assert!(
            stimulus.contains("[HOLDER CONTEXT]"),
            "stimulus should have holder context section"
        );
        assert!(
            stimulus.contains("effective_wallet_concentration: 9.7%"),
            "stimulus should show effective concentration"
        );
        assert!(
            stimulus.contains("contracts: 54.1%"),
            "stimulus should show contract percentage"
        );
        assert!(
            stimulus.contains("oracles: 5.0%"),
            "stimulus should show oracle percentage"
        );
        assert!(
            stimulus.contains("infra: 1.0%"),
            "stimulus should show infra percentage"
        );
        assert!(
            stimulus.contains("institutional"),
            "stimulus should have institutional note when >30%"
        );
    }
}
