//! Token-analysis domain scorer for DeterministicDog.
//!
//! Parses structured [METRICS] from build_token_stimulus() output.
//! Scores SUBSTANCE (on-chain signals) not FORM (text structure).
//! All 6 axioms scored — zero abstentions for this domain.

use crate::domain::dog::*;

use super::{ADJUST_LARGE, ADJUST_MEDIUM, ADJUST_SMALL, PHI_BASE, SOVEREIGNTY_BASE};

/// Parsed metrics from a token-analysis stimulus.
#[derive(Debug)]
pub(super) struct TokenMetrics {
    holder_data_available: bool,
    holders: u64,
    top1_pct: f64,
    top1_is_lp: bool,
    top10_pct: f64,
    herfindahl: Option<f64>,
    age_hours: u64,
    mint_authority_active: bool,
    freeze_authority_active: bool,
    lp_burned: bool,
    lp_locked: bool,
    supply_burned_pct: Option<f64>,
    origin_pump_fun: bool,
    // Market data fields (from stimulus [METRICS])
    price_usd: Option<f64>,
    market_cap_usd: Option<f64>,
    volume_24h_usd: Option<f64>,
    liquidity_usd: Option<f64>,
    // K-Score behavioral fields (from [BEHAVIORAL] section)
    k_score: Option<f64>,
    k_diamond_hands: Option<f64>,
    k_wallets_analyzed: u32,
    k_accumulators: u32,
    k_extractors: u32,
    // Holder identity fields (from [HOLDER IDENTITIES] section)
    identity_exchanges: u32,
    identity_protocols: u32,
    identity_scammers: u32,
    identity_total: u32,
    // Holder context fields (from [HOLDER CONTEXT] section)
    /// Effective wallet concentration: only freely-tradeable supply in top holders.
    /// When present, use INSTEAD of top10_pct for phi/sovereignty scoring.
    effective_concentration: Option<f64>,
    /// Percentage held by smart contracts (vesting, DAO, protocol)
    contract_pct: Option<f64>,
    /// Percentage held by lock/vesting programs
    locker_pct: Option<f64>,
    // Trajectory fields (from [TRAJECTORY] section — daily cron decay curve)
    /// Trajectory class: DEAD, DYING, DECLINING, STABLE, UNKNOWN
    trajectory_class: Option<String>,
    /// Conviction decay between shortest and longest window (0.0 to 1.0)
    trajectory_decay: Option<f64>,
    // K-Score sub-components (from [BEHAVIORAL] section)
    /// Longevity pillar: 1 - e^(-age_days/21). Strongest K-Score sub-signal (rho=+0.632).
    k_longevity: Option<f64>,
    /// Organic growth pillar.
    k_organic_growth: Option<f64>,
    /// Median hold time in hours across analyzed wallets (time dimension — orthogonal to retention).
    k_median_hold_hours: Option<f64>,
}

/// Parse human-readable USD values like "1.23B", "45.67M", "890K", "123.45".
/// Returns None for malformed input — caller treats absence as "no market data".
fn parse_human_usd(s: &str) -> Option<f64> {
    let s = s.trim();
    let (num_str, multiplier) = if let Some(num) = s.strip_suffix('B') {
        (num, 1e9)
    } else if let Some(num) = s.strip_suffix('M') {
        (num, 1e6)
    } else if let Some(num) = s.strip_suffix('K') {
        (num, 1e3)
    } else {
        (s, 1.0)
    };
    match num_str.parse::<f64>() {
        Ok(n) => Some(n * multiplier),
        Err(_) => None, // malformed stimulus field — scored without market data
    }
}

/// Extract token metrics from a formatted stimulus string.
/// Returns None if content is not a token-analysis stimulus.
pub(super) fn parse(content: &str) -> Option<TokenMetrics> {
    if !content.starts_with("[DOMAIN: token-analysis]") {
        return None;
    }

    let metrics_start = content.find("[METRICS]")?;
    // Section ends at next bracket-delimited header or EOF
    let section_rest = &content[metrics_start..];
    let metrics_end = section_rest[9..] // skip "[METRICS]"
        .find("\n[")
        .map(|i| metrics_start + 9 + i)
        .unwrap_or(content.len());
    let section = &content[metrics_start..metrics_end];

    let mut m = TokenMetrics {
        holder_data_available: true,
        holders: 0,
        top1_pct: 0.0,
        top1_is_lp: false,
        top10_pct: 0.0,
        herfindahl: None,
        age_hours: 0,
        mint_authority_active: false,
        freeze_authority_active: false,
        lp_burned: false,
        lp_locked: false,
        supply_burned_pct: None,
        origin_pump_fun: false,
        price_usd: None,
        market_cap_usd: None,
        volume_24h_usd: None,
        liquidity_usd: None,
        k_score: None,
        k_diamond_hands: None,
        k_wallets_analyzed: 0,
        k_accumulators: 0,
        k_extractors: 0,
        identity_exchanges: 0,
        identity_protocols: 0,
        identity_scammers: 0,
        identity_total: 0,
        effective_concentration: None,
        contract_pct: None,
        locker_pct: None,
        trajectory_class: None,
        trajectory_decay: None,
        k_longevity: None,
        k_organic_growth: None,
        k_median_hold_hours: None,
    };

    for line in section.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("holders: ") {
            if v.starts_with("UNAVAILABLE") {
                m.holder_data_available = false;
            } else {
                // Parse "20" or "20+" or "20+ (top accounts...)" — strip non-digit suffix
                m.holders = v
                    .trim_start()
                    .split(|c: char| !c.is_ascii_digit())
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
        } else if let Some(v) = line.strip_prefix("top_1_wallet_pct: ") {
            m.top1_pct = v.trim_end_matches('%').parse().unwrap_or(0.0);
        } else if let Some(v) = line.strip_prefix("top_1_wallet_type: ") {
            m.top1_is_lp = v.starts_with("lp_pool") || v.starts_with("burn");
        } else if let Some(v) = line.strip_prefix("top_10_wallets_pct: ") {
            m.top10_pct = v.trim_end_matches('%').parse().unwrap_or(0.0);
        } else if let Some(v) = line.strip_prefix("herfindahl_index: ") {
            m.herfindahl = v.parse().ok();
        } else if let Some(v) = line.strip_prefix("age_hours: ") {
            // Parse "720" or ">=720 (estimated...)" — strip prefix/suffix
            let digits = v.trim_start_matches(|c: char| !c.is_ascii_digit());
            m.age_hours = digits
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("mint_authority: ") {
            m.mint_authority_active = v.starts_with("ACTIVE");
        } else if let Some(v) = line.strip_prefix("freeze_authority: ") {
            m.freeze_authority_active = v.starts_with("ACTIVE");
        } else if let Some(v) = line.strip_prefix("lp_secured: ") {
            m.lp_burned = v.starts_with("YES");
            m.lp_locked = v.starts_with("PARTIAL");
        } else if let Some(v) = line.strip_prefix("supply_burned_pct: ") {
            m.supply_burned_pct = v.trim_end_matches('%').parse().ok();
        } else if let Some(v) = line.strip_prefix("origin: ") {
            m.origin_pump_fun = v.eq_ignore_ascii_case("pump.fun");
        } else if let Some(v) = line.strip_prefix("price_usd: $") {
            m.price_usd = v.parse().ok();
        } else if let Some(v) = line.strip_prefix("market_cap_usd: $") {
            // Parse "$1.23B" / "$45.67M" / "$890K" / "$123.45"
            m.market_cap_usd = parse_human_usd(v);
        } else if let Some(v) = line.strip_prefix("volume_24h_usd: $") {
            m.volume_24h_usd = parse_human_usd(v);
        } else if let Some(v) = line.strip_prefix("liquidity_usd: $") {
            m.liquidity_usd = parse_human_usd(v);
        }
    }

    // Parse [BEHAVIORAL] section if present
    if let Some(beh_start) = content.find("[BEHAVIORAL]") {
        let beh_rest = &content[beh_start..];
        let beh_end = beh_rest[12..]
            .find("\n[")
            .map(|i| beh_start + 12 + i)
            .unwrap_or(content.len());
        let beh_section = &content[beh_start..beh_end];

        for line in beh_section.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("k_score: ") {
                m.k_score = v.split_whitespace().next().and_then(|s| s.parse().ok());
            } else if let Some(v) = line.strip_prefix("diamond_hands: ") {
                m.k_diamond_hands = v.split_whitespace().next().and_then(|s| s.parse().ok());
            } else if let Some(v) = line.strip_prefix("organic_growth: ") {
                m.k_organic_growth = v.split_whitespace().next().and_then(|s| s.parse().ok());
            } else if let Some(v) = line.strip_prefix("longevity: ") {
                m.k_longevity = v.split_whitespace().next().and_then(|s| s.parse().ok());
            } else if let Some(v) = line.strip_prefix("median_hold_hours: ") {
                m.k_median_hold_hours = v.split_whitespace().next().and_then(|s| s.parse().ok());
            } else if let Some(v) = line.strip_prefix("wallet_breakdown: ") {
                // "N analyzed — X accumulators, Y holders, Z reducers, W extractors"
                m.k_wallets_analyzed = v
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                // Split on em-dash to separate "N analyzed" from class breakdown
                let classes = v
                    .split('\u{2014}')
                    .nth(1)
                    .or_else(|| v.split("--").nth(1))
                    .unwrap_or("");
                for part in classes.split(',') {
                    let part = part.trim();
                    if part.ends_with("accumulators") {
                        m.k_accumulators = part
                            .split_whitespace()
                            .next()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                    } else if part.ends_with("extractors") {
                        m.k_extractors = part
                            .split_whitespace()
                            .next()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                    }
                }
            }
        }
    }

    // Parse [HOLDER IDENTITIES] section if present
    if let Some(id_start) = content.find("[HOLDER IDENTITIES]") {
        let id_rest = &content[id_start..];
        let id_end = id_rest[19..]
            .find("\n[")
            .map(|i| id_start + 19 + i)
            .unwrap_or(content.len());
        let id_section = &content[id_start..id_end];

        for line in id_section.lines() {
            let line = line.trim();
            if line.contains("(Centralized Exchange)") {
                m.identity_exchanges += 1;
                m.identity_total += 1;
            } else if line.contains("(DeFi)") || line.contains("(Swap)") {
                m.identity_protocols += 1;
                m.identity_total += 1;
            } else if line.contains("(Rugger)")
                || line.contains("(Scam")
                || line.contains("(Exploit")
            {
                m.identity_scammers += 1;
                m.identity_total += 1;
            } else if line.starts_with("exchanges_in_holders:") {
                // Summary line — already counted individually above
            } else if line.starts_with("WARNING:") && line.contains("scammer") {
                // Summary warning — already counted
            } else if line.contains('(') && line.contains(')') && !line.starts_with("identified:") {
                // Other identified entity (not exchange/defi/scammer)
                m.identity_total += 1;
            }
        }
    }

    // Parse [HOLDER CONTEXT] section if present
    if let Some(hc_start) = content.find("[HOLDER CONTEXT]") {
        let hc_rest = &content[hc_start..];
        let hc_end = hc_rest[16..]
            .find("\n[")
            .map(|i| hc_start + 16 + i)
            .unwrap_or(content.len());
        let hc_section = &content[hc_start..hc_end];

        for line in hc_section.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("effective_wallet_concentration: ") {
                m.effective_concentration = v.trim_end_matches('%').parse().ok();
            } else if let Some(v) = line.strip_prefix("contracts: ") {
                m.contract_pct = v
                    .trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.')
                    .split('%')
                    .next()
                    .and_then(|s| s.parse().ok());
            } else if let Some(v) = line.strip_prefix("locked: ") {
                m.locker_pct = v
                    .trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.')
                    .split('%')
                    .next()
                    .and_then(|s| s.parse().ok());
            }
        }
    }

    // Parse [TRAJECTORY] section if present
    if let Some(tr_start) = content.find("[TRAJECTORY]") {
        let tr_rest = &content[tr_start..];
        let tr_end = tr_rest[12..]
            .find("\n[")
            .map(|i| tr_start + 12 + i)
            .unwrap_or(content.len());
        let tr_section = &content[tr_start..tr_end];

        for line in tr_section.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("class: ") {
                m.trajectory_class = Some(v.trim().to_string());
            } else if let Some(v) = line.strip_prefix("decay: ") {
                m.trajectory_decay = v.parse().ok();
            }
        }
    }

    Some(m)
}

/// Detect established infrastructure tokens (stablecoins, wrapped assets).
///
/// These tokens have active mint/freeze authorities BY DESIGN — penalizing them
/// inverts discrimination (USDC scores lower than dead memecoins with revoked authorities).
///
/// Gate: at least one authority active (tokens with revoked authorities score fine already)
/// PLUS scale proof via one of:
/// - Path A: market_cap >= $500M (when enrichment provides it)
/// - Path B: holders >= 100K + age >= 720h — covers CEX-primary tokens (USDC/USDT)
///   where DexScreener/Helius don't return market_cap
///
/// Falsify: scam token reaches 100K holders + 30d age + active authorities.
/// Extremely rare — 98.6% rug before day 7.
fn is_established_infrastructure(m: &TokenMetrics) -> bool {
    // Only applies when authorities would be penalized — tokens with revoked
    // authorities already score well under standard rules.
    let has_active_authority = m.mint_authority_active || m.freeze_authority_active;
    if !has_active_authority {
        return false;
    }
    let has_large_mcap = m.market_cap_usd.is_some_and(|mc| mc >= 500_000_000.0);
    let has_massive_holders = m.holders >= 100_000;
    let has_age = m.age_hours >= 720;
    (has_large_mcap || has_massive_holders) && has_age
}

/// Score a token-analysis stimulus on all 6 axioms using on-chain metrics.
/// No abstentions — every axiom has deterministic signals from TokenData.
///
/// Falsification: each heuristic states what would make it wrong.
///
/// - Authority checks fail for governance tokens with scheduled emissions.
/// - Concentration checks fail for exchange cold wallets (top1 misleading).
/// - Age checks fail for legitimate token migrations.
///
/// The ensemble (LLM Dogs) compensates for these known blind spots.
pub(super) fn score(m: &TokenMetrics) -> AxiomScores {
    // ── FIDELITY: Does the token faithfully represent what it claims? ──
    // Falsify: legitimate governance token with active mint for scheduled emissions.
    let established = is_established_infrastructure(m);
    let mut fidelity: f64 = PHI_BASE;
    if established {
        // Established infrastructure tokens (stablecoins, etc.) NEED active authorities.
        // Active mint+freeze on a $500M+ token is faithful to its design, not dishonest.
        fidelity += ADJUST_MEDIUM; // proven track record = fidelity
    } else {
        if !m.mint_authority_active {
            fidelity += ADJUST_MEDIUM; // supply locked = honest commitment
        } else {
            fidelity -= ADJUST_MEDIUM; // can inflate = potential dishonesty
        }
        if !m.freeze_authority_active {
            fidelity += ADJUST_SMALL; // wallets free
        } else {
            fidelity -= ADJUST_MEDIUM; // can freeze = deceptive control
        }
    }
    // K-Score diamond_hands: empirically INVERTED (rho=-0.396, n=30).
    // High diamond_hands = FOMO buyers accumulating, not genuine conviction.
    // Low diamond_hands = passive holders who stayed = actual fidelity.
    // Corrected: invert the signal direction.
    if let Some(dh) = m.k_diamond_hands {
        if dh > 0.6 {
            fidelity -= ADJUST_MEDIUM; // rho=-0.396 (diamond_hands, 2026-05) — FOMO signal
        } else if dh < 0.3 {
            fidelity += ADJUST_SMALL; // rho=-0.396 — low DH = passive holders = real conviction
        }
    }
    // Hold time: temporal dimension orthogonal to retention ratio.
    // Retention (current_balance/total_bought) answers "how much remains".
    // Hold time answers "how long did they stay". Both can disagree — a wallet that
    // bought 2h ago and hasn't sold yet has retention=1.0 but hold_time=2h (flipper profile).
    // >7d = committed holder. <24h = flipper pattern. Falsify: project in early phase
    // should show short hold times without being penalized — add age gating if needed.
    if let Some(mh) = m.k_median_hold_hours {
        if mh > 168.0 {
            // >7 days median — holders committed across the analyzed window
            fidelity += ADJUST_SMALL;
        } else if mh < 24.0 {
            // <24h median — flipper-dominant holder base
            fidelity -= ADJUST_SMALL;
        }
    }
    // Trajectory: temporal conviction signal — holders leaving = broken promise.
    // Falsify: token in legitimate migration phase shows DYING despite healthy fundamentals.
    if let Some(ref tclass) = m.trajectory_class {
        match tclass.as_str() {
            "DEAD" => fidelity -= ADJUST_MEDIUM, // virtually no one stayed — promise broken
            "DYING" => fidelity -= ADJUST_SMALL, // steep churn — holders losing faith
            "STABLE" => fidelity += ADJUST_SMALL, // diamond hands — promise kept
            _ => {}                              // DECLINING/UNKNOWN = no adjustment
        }
    }
    // Market cap: NOISE (rho=-0.083, n=30). Removed from scoring.
    // Supply burned: strongest structural signal (rho=+0.672, LARGE tier).
    // Burned supply = irrevocable commitment = fidelity to holders.
    if let Some(bp) = m.supply_burned_pct {
        if bp > 20.0 {
            fidelity += ADJUST_LARGE; // rho=+0.672 (supply_burned, 2026-05)
        } else if bp > 5.0 {
            fidelity += ADJUST_MEDIUM; // rho=+0.672 (supply_burned, 2026-05)
        }
    }
    let fidelity = fidelity.clamp(ADJUST_SMALL, PHI_INV);
    let traj_label = m.trajectory_class.as_deref().unwrap_or("n/a");
    let fidelity_reason = format!(
        "{}mint_authority={}, freeze_authority={}, diamond_hands={}, mcap={}, trajectory={traj_label}.",
        if established {
            "ESTABLISHED INFRASTRUCTURE — "
        } else {
            ""
        },
        if m.mint_authority_active {
            if established {
                "ACTIVE (expected for infrastructure)"
            } else {
                "ACTIVE (red flag)"
            }
        } else {
            "revoked"
        },
        if m.freeze_authority_active {
            if established {
                "ACTIVE (expected for infrastructure)"
            } else {
                "ACTIVE (red flag)"
            }
        } else {
            "revoked"
        },
        m.k_diamond_hands
            .map(|d| format!("{d:.3}"))
            .unwrap_or_else(|| "n/a".into()),
        m.market_cap_usd
            .map(|mc| format!("${mc:.0}"))
            .unwrap_or_else(|| "n/a".into()),
    );

    // ── PHI: Structural harmony of holder distribution ──
    // When holder context is available, use effective_wallet_concentration (wallet-only %)
    // instead of raw top1/top10 — institutional holdings (vesting, LP) don't indicate manipulation.
    // Falsify: exchange cold wallet inflates top1% without real concentration.
    let mut phi: f64 = PHI_BASE;
    if m.holder_data_available {
        if let Some(h) = m.herfindahl {
            if h < 0.15 {
                phi += ADJUST_MEDIUM;
            } else if h > 0.50 {
                phi -= ADJUST_MEDIUM;
            }
        }
        // Use effective concentration when holder context is available
        if let Some(eff) = m.effective_concentration {
            if eff < 15.0 {
                phi += ADJUST_MEDIUM; // real retail distribution is healthy
            } else if eff > 50.0 {
                phi -= ADJUST_MEDIUM; // real retail concentration is concerning
            } else if eff < 30.0 {
                phi += ADJUST_SMALL; // moderate retail distribution
            }
            // Institutional commitment signal: contracts holding supply = planned distribution
            let institutional = m.contract_pct.unwrap_or(0.0) + m.locker_pct.unwrap_or(0.0);
            if institutional > 30.0 {
                phi += ADJUST_SMALL;
            }
        } else {
            // Fallback: raw concentration (no holder context available)
            if m.top1_pct < 15.0 {
                phi += ADJUST_SMALL;
            } else if m.top1_pct > 50.0 && !m.top1_is_lp {
                phi -= ADJUST_MEDIUM;
            }
        }
        if m.holders > 1000 {
            phi += ADJUST_MEDIUM;
        } else if m.holders > 100 {
            phi += ADJUST_SMALL;
        } else if m.holders < 20 {
            phi -= ADJUST_SMALL;
        }
    }
    // else: holder data unavailable — PHI stays at neutral base
    // K-Score composite: REMOVED — empirically INVERTED (rho=-0.327, n=30).
    // The composite DH*OG*L destroys longevity signal by multiplying with inverted components.
    //
    // Longevity sub-component: KEPT — strongest K-Score signal (rho=+0.632).
    // Tokens that survived longer have better holder distribution (phi harmony).
    if let Some(lon) = m.k_longevity {
        if lon > 0.8 {
            phi += ADJUST_LARGE; // rho=+0.632 (longevity, 2026-05) — strongest positive predictor
        } else if lon > 0.5 {
            phi += ADJUST_MEDIUM; // rho=+0.632 — moderate age signal
        } else if lon < 0.3 {
            phi -= ADJUST_SMALL; // very new = distribution unproven
        }
    }
    // Liquidity: REMOVED from scoring — NOISE (rho=+0.038, n=30).
    // Previously scored with ADJUST_MEDIUM. Wash trading makes this unreliable.
    let phi = phi.clamp(ADJUST_SMALL, PHI_INV);
    let phi_reason = if m.holder_data_available {
        if let Some(eff) = m.effective_concentration {
            format!(
                "holders={}+, effective_wallet_concentration={:.1}% (raw top1={:.1}%), contracts={:.1}%, liquidity={}.",
                m.holders,
                eff,
                m.top1_pct,
                m.contract_pct.unwrap_or(0.0) + m.locker_pct.unwrap_or(0.0),
                m.liquidity_usd
                    .map(|l| format!("${l:.0}"))
                    .unwrap_or_else(|| "n/a".into()),
            )
        } else {
            format!(
                "holders={}+, top1={:.1}%, HHI={}, liquidity={}.",
                m.holders,
                m.top1_pct,
                m.herfindahl
                    .map(|h| format!("{h:.3}"))
                    .unwrap_or_else(|| "n/a".into()),
                m.liquidity_usd
                    .map(|l| format!("${l:.0}"))
                    .unwrap_or_else(|| "n/a".into()),
            )
        }
    } else {
        "holder data unavailable (RPC degraded) — scoring at neutral base.".into()
    };

    // ── VERIFY: Can metrics be independently verified on-chain? ──
    // Base above NEUTRAL: on-chain data IS verifiable by definition.
    // Falsify: fake on-chain data from compromised RPC.
    let mut verify: f64 = PHI_BASE + ADJUST_SMALL; // 0.35 base
    if m.lp_burned {
        verify += ADJUST_SMALL; // permanently verifiable commitment
    } else if !m.lp_locked {
        verify -= ADJUST_SMALL; // unverifiable commitment
    }
    if m.age_hours > 720 {
        verify += ADJUST_SMALL; // time-tested, verified by survival
    } else if m.age_hours < 24 {
        verify -= ADJUST_SMALL; // unproven
    }
    // Supply burned: verifiable on-chain commitment. rho=+0.672 (supply_burned, 2026-05).
    if m.supply_burned_pct.is_some_and(|bp| bp > 10.0) {
        verify += ADJUST_MEDIUM; // rho=+0.672 — burned supply verifiable forever
    }
    // Trajectory: temporal verification = multi-day evidence, stronger than snapshot.
    if m.trajectory_class.is_some() {
        verify += ADJUST_SMALL;
    }
    // Volume: real trading activity = market-verified existence
    // Falsify: wash trading inflates volume without real price discovery.
    if let Some(vol) = m.volume_24h_usd {
        if vol >= 1_000_000.0 {
            verify += ADJUST_MEDIUM; // $1M+ daily volume = market-validated
        } else if vol >= 10_000.0 {
            verify += ADJUST_SMALL; // measurable activity
        }
    }
    let verify = verify.clamp(ADJUST_SMALL, PHI_INV);
    let verify_reason = format!(
        "On-chain verifiable. LP={}, age={}h, volume_24h={}.",
        if m.lp_burned {
            "burned"
        } else if m.lp_locked {
            "locked"
        } else {
            "unsecured"
        },
        m.age_hours,
        m.volume_24h_usd
            .map(|v| format!("${v:.0}"))
            .unwrap_or_else(|| "n/a".into()),
    );

    // ── CULTURE: Does it follow established token standards? ──
    // Falsify: new cultural norm not yet in the heuristic set.
    let mut culture: f64 = PHI_BASE;
    if m.lp_burned {
        culture += ADJUST_MEDIUM; // follows established practice
    } else if !m.lp_locked {
        culture -= ADJUST_SMALL; // deviates from standard
    }
    if established {
        culture += ADJUST_MEDIUM; // established ecosystem standard
    } else {
        if !m.mint_authority_active {
            culture += ADJUST_SMALL;
        }
        if !m.freeze_authority_active {
            culture += ADJUST_SMALL;
        }
        if m.origin_pump_fun {
            culture -= ADJUST_SMALL; // 98.6% rug rate baseline (Solidus Labs 2025)
        }
    }
    if m.holders > 100 {
        culture += ADJUST_SMALL;
    }
    let culture = culture.clamp(ADJUST_SMALL, PHI_INV);
    let culture_reason = if established {
        "Established infrastructure token — ecosystem standard.".to_string()
    } else {
        format!(
            "LP={}, origin={}.",
            if m.lp_burned {
                "burned (standard)"
            } else if m.lp_locked {
                "locked"
            } else {
                "unsecured"
            },
            if m.origin_pump_fun {
                "pump.fun (98.6% rug baseline)"
            } else {
                "other"
            },
        )
    };

    // ── BURN: Efficiency, minimal waste ──
    // Falsify: token with active mint for legitimate yield distribution.
    let mut burn: f64 = PHI_BASE;
    if let Some(burned_pct) = m.supply_burned_pct {
        // rho=+0.672 (supply_burned, 2026-05) — strongest structural signal
        // Gradient: up to +ADJUST_MEDIUM for 50%+ supply burned
        burn += ADJUST_MEDIUM * (burned_pct / 50.0).min(1.0);
    }
    if established {
        // Established tokens: authorities are necessary infrastructure, not waste
        burn += ADJUST_SMALL; // proven efficient design for its purpose
    } else if !m.mint_authority_active && !m.freeze_authority_active {
        burn += ADJUST_MEDIUM; // no unnecessary authorities retained
    } else if m.mint_authority_active && m.freeze_authority_active {
        burn -= ADJUST_SMALL; // both retained = governance waste
    }
    if m.lp_burned {
        burn += ADJUST_SMALL; // efficient capital commitment
    } else if !m.lp_locked {
        burn -= ADJUST_SMALL; // LP waste risk
    }
    if m.age_hours > 720 {
        burn += ADJUST_SMALL; // survived = efficient enough
    }
    let burn = burn.clamp(ADJUST_SMALL, PHI_INV);
    let burn_reason = format!(
        "authorities={}, LP={}, burned_pct={}.",
        if established {
            "active (infrastructure — expected)"
        } else if !m.mint_authority_active && !m.freeze_authority_active {
            "both revoked (efficient)"
        } else {
            "active (waste)"
        },
        if m.lp_burned {
            "burned"
        } else if m.lp_locked {
            "locked"
        } else {
            "unsecured"
        },
        m.supply_burned_pct
            .map(|p| format!("{p:.1}%"))
            .unwrap_or_else(|| "n/a".into()),
    );

    // ── SOVEREIGNTY: Distributed control, holder freedom ──
    // When holder context is available, use effective_wallet_concentration.
    // Tokens in vesting/lock contracts are scheduled for release — not whale domination.
    // Falsify: legitimate DAO with top1% held by treasury multisig.
    let mut sovereignty: f64 = SOVEREIGNTY_BASE;
    if m.holder_data_available {
        if let Some(eff) = m.effective_concentration {
            if eff < 20.0 {
                sovereignty += ADJUST_MEDIUM; // real retail control is distributed
            } else if eff > 50.0 {
                sovereignty -= ADJUST_MEDIUM; // real whale concentration
            } else if eff < 35.0 {
                sovereignty += ADJUST_SMALL;
            }
            // Vesting/lock contracts = path to eventual distribution
            let locked = m.contract_pct.unwrap_or(0.0) + m.locker_pct.unwrap_or(0.0);
            if locked > 30.0 {
                sovereignty += ADJUST_SMALL; // controlled distribution pathway
            }
        } else {
            // Fallback: raw concentration (no holder context)
            if m.top1_pct < 15.0 {
                sovereignty += ADJUST_MEDIUM;
            } else if m.top1_pct > 50.0 && !m.top1_is_lp {
                sovereignty -= ADJUST_MEDIUM;
            }
        }
        if m.holders > 100 {
            sovereignty += ADJUST_SMALL;
        }
        if let Some(h) = m.herfindahl {
            if h < 0.15 {
                sovereignty += ADJUST_SMALL;
            } else if h > 0.50 {
                sovereignty -= ADJUST_SMALL;
            }
        }
    }
    // Authority checks: skip for established infrastructure (stablecoins need freeze/mint)
    if !established {
        if m.freeze_authority_active {
            sovereignty -= ADJUST_MEDIUM;
        }
        if m.mint_authority_active {
            sovereignty -= ADJUST_SMALL;
        }
    }
    // Trajectory: STABLE conviction = holders exercise agency to STAY.
    // DYING/DEAD = holders leaving (sovereignty intact — they chose to leave — but token is weakened).
    // Falsify: STABLE trajectory on a token with frozen wallets (holders CAN'T leave, not WON'T).
    if let Some(ref tclass) = m.trajectory_class {
        match tclass.as_str() {
            "STABLE" => sovereignty += ADJUST_SMALL, // holders freely choosing to hold
            "DEAD" => sovereignty -= ADJUST_SMALL,   // mass exit = project lost mandate
            _ => {}
        }
    }
    // K-Score accumulators: empirically INVERTED (rho=-0.622, n=30).
    // More accumulators = FOMO buying (weak tokens). More extractors = mature holders who already
    // took profit but core holders stayed = stronger sovereignty signal.
    // Corrected: extractor dominance is now POSITIVE (was negative).
    if m.k_wallets_analyzed > 0 && m.k_accumulators > m.k_extractors {
        sovereignty -= ADJUST_LARGE; // rho=-0.622 (accumulator_ratio, 2026-05) — FOMO signal
    }
    // Holder identity signals (from Helius batch-identity)
    if m.identity_exchanges >= 3 {
        sovereignty += ADJUST_MEDIUM; // 3+ exchanges = strong institutional backing
    } else if m.identity_exchanges >= 1 {
        sovereignty += ADJUST_SMALL;
    }
    if m.identity_scammers > 0 {
        sovereignty -= ADJUST_MEDIUM * 2.0; // Any known scammer = critical red flag
    }
    let sovereignty = sovereignty.clamp(ADJUST_SMALL, PHI_INV);
    let identity_note = if m.identity_total > 0 {
        format!(
            " Known holders: {} exchanges, {} protocols, {} scammers.",
            m.identity_exchanges, m.identity_protocols, m.identity_scammers
        )
    } else {
        String::new()
    };
    let traj_note = m
        .trajectory_class
        .as_deref()
        .map(|t| format!(" trajectory={t}."))
        .unwrap_or_default();
    let sovereignty_reason = format!(
        "{}freeze={}, extractors_vs_acc={}/{}.{identity_note}{traj_note}",
        if m.holder_data_available {
            if let Some(eff) = m.effective_concentration {
                format!(
                    "effective_wallet={:.1}% (raw top1={:.1}%), holders={}+, ",
                    eff, m.top1_pct, m.holders
                )
            } else {
                format!("top1={:.1}%, holders={}+, ", m.top1_pct, m.holders)
            }
        } else {
            "holders=unavailable, ".into()
        },
        if m.freeze_authority_active {
            "ACTIVE (restricts freedom)"
        } else {
            "revoked"
        },
        m.k_extractors,
        m.k_accumulators,
    );

    AxiomScores {
        fidelity,
        phi,
        verify,
        culture,
        burn,
        sovereignty,
        prompt_tokens: 0,
        completion_tokens: 0,
        thinking_tokens: 0,
        reasoning: AxiomReasoning {
            fidelity: fidelity_reason,
            phi: phi_reason,
            verify: verify_reason,
            culture: culture_reason,
            burn: burn_reason,
            sovereignty: sovereignty_reason,
        },
        reasoning_trace: None,
        abstentions: vec![], // All 6 axioms judged from on-chain data
    }
}

#[cfg(test)]
#[cfg(test)]
include!("tests/token_tests.rs");
