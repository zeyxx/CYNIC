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
mod tests {
    use super::*;

    /// Test token profile — mirrors TokenData fields relevant to scoring.
    struct TestMint {
        holders: u64,
        top1_pct: f64,
        top10_pct: f64,
        herfindahl: Option<f64>,
        age_hours: u64,
        mint_active: bool,
        freeze_active: bool,
        lp: &'static str, // "burned", "locked", or "unsecured"
        burned_pct: Option<f64>,
        origin: Option<&'static str>,
    }

    /// Build a token-analysis stimulus from test parameters.
    /// Matches the format produced by build_token_stimulus() in stimulus.rs.
    fn make_token_stimulus(t: &TestMint) -> String {
        let mut s = String::from("[DOMAIN: token-analysis]\n\n[METRICS]\n");
        s.push_str("mint: test_mint\n");
        s.push_str(&format!("holders: {}\n", t.holders));
        s.push_str(&format!("top_1_wallet_pct: {:.2}%\n", t.top1_pct));
        s.push_str(&format!("top_10_wallets_pct: {:.2}%\n", t.top10_pct));
        if let Some(h) = t.herfindahl {
            s.push_str(&format!("herfindahl_index: {h:.3}\n"));
        }
        s.push_str(&format!("age_hours: {}\n", t.age_hours));
        s.push_str(&format!(
            "mint_authority: {}\n",
            if t.mint_active {
                "ACTIVE (can mint more tokens)"
            } else {
                "REVOKED (supply is fixed)"
            }
        ));
        s.push_str(&format!(
            "freeze_authority: {}\n",
            if t.freeze_active {
                "ACTIVE (can freeze wallets)"
            } else {
                "REVOKED (wallets are free)"
            }
        ));
        s.push_str(&format!(
            "lp_secured: {}\n",
            match t.lp {
                "burned" => "YES — LP tokens burned (permanent liquidity)",
                "locked" => "PARTIAL — LP tokens locked (temporary)",
                _ => "NO — LP tokens in creator wallet (can rug)",
            }
        ));
        if let Some(bp) = t.burned_pct {
            s.push_str(&format!("supply_burned_pct: {bp:.2}%\n"));
        }
        if let Some(o) = t.origin {
            s.push_str(&format!("origin: {o}\n"));
        }
        s.push_str("\n[BASELINES]\nhealthy_token: holders>100\n");
        s
    }

    const HEALTHY_MINT: TestMint = TestMint {
        holders: 250_000,
        top1_pct: 12.5,
        top10_pct: 45.2,
        herfindahl: Some(0.08),
        age_hours: 1200,
        mint_active: false,
        freeze_active: false,
        lp: "burned",
        burned_pct: Some(0.0),
        origin: None,
    };

    const RUG_MINT: TestMint = TestMint {
        holders: 3,
        top1_pct: 99.0,
        top10_pct: 100.0,
        herfindahl: Some(0.98),
        age_hours: 1,
        mint_active: true,
        freeze_active: true,
        lp: "unsecured",
        burned_pct: Some(0.0),
        origin: Some("pump.fun"),
    };

    const MODERATE_MINT: TestMint = TestMint {
        holders: 50,
        top1_pct: 25.0,
        top10_pct: 60.0,
        herfindahl: Some(0.25),
        age_hours: 168,
        mint_active: false,
        freeze_active: false,
        lp: "locked",
        burned_pct: None,
        origin: None,
    };

    #[test]
    fn parse_token_metrics_healthy_token() {
        let content = make_token_stimulus(&HEALTHY_MINT);
        let m = parse(&content).expect("should parse");
        assert_eq!(m.holders, 250_000);
        assert!((m.top1_pct - 12.5).abs() < 0.01);
        assert!((m.top10_pct - 45.2).abs() < 0.01);
        assert!((m.herfindahl.unwrap() - 0.08).abs() < 0.01);
        assert_eq!(m.age_hours, 1200);
        assert!(!m.mint_authority_active);
        assert!(!m.freeze_authority_active);
        assert!(m.lp_burned);
        assert!(!m.origin_pump_fun);
    }

    #[test]
    fn parse_token_metrics_rug_token() {
        let content = make_token_stimulus(&RUG_MINT);
        let m = parse(&content).expect("should parse");
        assert_eq!(m.holders, 3);
        assert!(m.mint_authority_active);
        assert!(m.freeze_authority_active);
        assert!(!m.lp_burned);
        assert!(!m.lp_locked);
        assert!(m.origin_pump_fun);
    }

    #[test]
    fn parse_token_metrics_rejects_non_token() {
        assert!(parse("Just a normal sentence.").is_none());
        assert!(parse("[DOMAIN: chess]\n[METRICS]\n").is_none());
    }

    #[tokio::test]
    async fn token_healthy_scores_high() {
        let content = make_token_stimulus(&HEALTHY_MINT);
        let m = parse(&content).unwrap();
        let scores = score(&m);

        assert!(
            scores.fidelity > 0.40,
            "healthy token fidelity should be high, got {}",
            scores.fidelity
        );
        assert!(
            scores.phi > 0.45,
            "healthy token phi should be high, got {}",
            scores.phi
        );
        assert!(
            scores.verify > 0.35,
            "healthy token verify should be above neutral, got {}",
            scores.verify
        );
        assert!(
            scores.culture > 0.45,
            "healthy token culture should be high, got {}",
            scores.culture
        );
        assert!(
            scores.burn > 0.40,
            "healthy token burn should be high, got {}",
            scores.burn
        );
        assert!(
            scores.sovereignty > 0.50,
            "healthy token sovereignty should be high, got {}",
            scores.sovereignty
        );
        assert!(
            scores.abstentions.is_empty(),
            "token-analysis should have zero abstentions, got {:?}",
            scores.abstentions
        );
    }

    #[tokio::test]
    async fn token_rug_scores_low() {
        let content = make_token_stimulus(&RUG_MINT);
        let m = parse(&content).unwrap();
        let scores = score(&m);

        assert!(
            scores.fidelity < 0.20,
            "rug fidelity should be very low, got {}",
            scores.fidelity
        );
        assert!(
            scores.phi < 0.15,
            "rug phi should be very low, got {}",
            scores.phi
        );
        assert!(
            scores.sovereignty < 0.15,
            "rug sovereignty should be very low, got {}",
            scores.sovereignty
        );
        let q_avg = (scores.fidelity
            + scores.phi
            + scores.verify
            + scores.culture
            + scores.burn
            + scores.sovereignty)
            / 6.0;
        assert!(
            q_avg < 0.236,
            "rug q_score should be BARK range, got {q_avg:.3}",
        );
    }

    #[tokio::test]
    async fn token_moderate_scores_middle() {
        let content = make_token_stimulus(&MODERATE_MINT);
        let m = parse(&content).unwrap();
        let scores = score(&m);

        assert!(
            scores.fidelity > 0.35,
            "moderate fidelity should be decent (authorities revoked), got {}",
            scores.fidelity
        );
        assert!(
            scores.phi > 0.20 && scores.phi < 0.50,
            "moderate phi should be middle range, got {}",
            scores.phi
        );
    }

    #[tokio::test]
    async fn token_scores_vary_across_profiles() {
        let healthy = make_token_stimulus(&HEALTHY_MINT);
        let rug = make_token_stimulus(&RUG_MINT);
        let mh = parse(&healthy).unwrap();
        let mr = parse(&rug).unwrap();
        let sc_h = score(&mh);
        let sc_r = score(&mr);

        assert!(
            (sc_h.fidelity - sc_r.fidelity).abs() > 0.15,
            "fidelity should differ: healthy={}, rug={}",
            sc_h.fidelity,
            sc_r.fidelity
        );
        assert!(
            (sc_h.phi - sc_r.phi).abs() > 0.15,
            "phi should differ: healthy={}, rug={}",
            sc_h.phi,
            sc_r.phi
        );
        assert!(
            (sc_h.sovereignty - sc_r.sovereignty).abs() > 0.15,
            "sovereignty should differ: healthy={}, rug={}",
            sc_h.sovereignty,
            sc_r.sovereignty
        );
    }

    #[tokio::test]
    async fn token_pump_fun_penalizes_culture() {
        let base = TestMint {
            holders: 100,
            top1_pct: 20.0,
            top10_pct: 50.0,
            herfindahl: Some(0.20),
            age_hours: 48,
            mint_active: false,
            freeze_active: false,
            lp: "burned",
            burned_pct: None,
            origin: Some("pump.fun"),
        };
        let pump = make_token_stimulus(&base);
        let manual = make_token_stimulus(&TestMint {
            origin: Some("manual"),
            ..base
        });
        let mp = parse(&pump).unwrap();
        let mm = parse(&manual).unwrap();
        let sc_pump = score(&mp);
        let sc_manual = score(&mm);

        assert!(
            sc_pump.culture < sc_manual.culture,
            "pump.fun should penalize culture: pump={}, manual={}",
            sc_pump.culture,
            sc_manual.culture
        );
    }

    #[tokio::test]
    async fn token_fallback_to_form_for_non_token() {
        // Non-token content: parse returns None
        assert!(parse("A simple neutral statement about the world.").is_none());
    }

    #[test]
    fn parse_token_with_behavioral_section() {
        let mut content = make_token_stimulus(&HEALTHY_MINT);
        let baselines_pos = content.find("\n[BASELINES]").unwrap();
        let behavioral = "\n[BEHAVIORAL]\nk_score: 0.650\ndiamond_hands: 0.720 (conviction of top holders)\norganic_growth: 0.580 (distribution quality)\nlongevity: 0.900 (age-adjusted survival)\nwallet_breakdown: 10 analyzed \u{2014} 4 accumulators, 3 holders, 2 reducers, 1 extractors\n";
        content.insert_str(baselines_pos, behavioral);

        let m = parse(&content).expect("should parse with behavioral");
        assert!((m.k_score.unwrap() - 0.65).abs() < 0.01);
        assert!((m.k_diamond_hands.unwrap() - 0.72).abs() < 0.01);
        assert_eq!(m.k_wallets_analyzed, 10);
        assert_eq!(m.k_accumulators, 4);
        assert_eq!(m.k_extractors, 1);
    }

    #[test]
    fn parse_token_without_behavioral_section() {
        let content = make_token_stimulus(&HEALTHY_MINT);
        let m = parse(&content).expect("should parse without behavioral");
        assert!(m.k_score.is_none());
        assert!(m.k_diamond_hands.is_none());
        assert_eq!(m.k_wallets_analyzed, 0);
    }

    #[tokio::test]
    async fn longevity_boosts_phi() {
        // Longevity (rho=+0.632) is the strongest K-Score sub-signal.
        // K-Score composite was REMOVED (rho=-0.327, dilutes signal).
        let mut with_lon = make_token_stimulus(&HEALTHY_MINT);
        let baselines_pos = with_lon.find("\n[BASELINES]").unwrap();
        let behavioral = "\n[BEHAVIORAL]\nk_score: 0.700\ndiamond_hands: 0.200 (conviction)\norganic_growth: 0.500 (distribution)\nlongevity: 0.950 (age-adjusted survival)\nwallet_breakdown: 10 analyzed \u{2014} 2 accumulators, 3 holders, 3 reducers, 2 extractors\n";
        with_lon.insert_str(baselines_pos, behavioral);
        let m_with = parse(&with_lon).unwrap();
        let scores_with = score(&m_with);

        let without = make_token_stimulus(&HEALTHY_MINT);
        let m_without = parse(&without).unwrap();
        let scores_without = score(&m_without);

        assert!(
            scores_with.phi >= scores_without.phi,
            "longevity should boost phi: with={}, without={}",
            scores_with.phi,
            scores_without.phi
        );
    }

    #[tokio::test]
    async fn high_diamond_hands_penalizes_fidelity() {
        // diamond_hands empirically INVERTED (rho=-0.396): high DH = FOMO buying.
        let mut with_dh = make_token_stimulus(&MODERATE_MINT);
        let baselines_pos = with_dh.find("\n[BASELINES]").unwrap();
        let behavioral = "\n[BEHAVIORAL]\nk_score: 0.700\ndiamond_hands: 0.800 (conviction)\nwallet_breakdown: 10 analyzed \u{2014} 5 accumulators, 3 holders, 1 reducers, 1 extractors\n";
        with_dh.insert_str(baselines_pos, behavioral);
        let m_with = parse(&with_dh).unwrap();
        let scores_with = score(&m_with);

        let without = make_token_stimulus(&MODERATE_MINT);
        let m_without = parse(&without).unwrap();
        let scores_without = score(&m_without);

        assert!(
            scores_with.fidelity <= scores_without.fidelity,
            "high diamond_hands should penalize fidelity (inverted signal): with={}, without={}",
            scores_with.fidelity,
            scores_without.fidelity
        );
    }

    #[tokio::test]
    async fn accumulator_dominance_penalizes_sovereignty() {
        // Empirically INVERTED (rho=-0.622): accumulator dominance = FOMO buying.
        let mut content = make_token_stimulus(&MODERATE_MINT);
        let baselines_pos = content.find("\n[BASELINES]").unwrap();
        let behavioral = "\n[BEHAVIORAL]\nk_score: 0.700\ndiamond_hands: 0.800 (conviction)\nwallet_breakdown: 8 analyzed \u{2014} 5 accumulators, 1 holders, 1 reducers, 1 extractors\n";
        content.insert_str(baselines_pos, behavioral);
        let m_acc = parse(&content).unwrap();
        let scores_acc = score(&m_acc);

        let without = make_token_stimulus(&MODERATE_MINT);
        let m_without = parse(&without).unwrap();
        let scores_without = score(&m_without);

        assert!(
            scores_acc.sovereignty <= scores_without.sovereignty,
            "Accumulator dominance should penalize sovereignty (inverted signal): acc={}, base={}",
            scores_acc.sovereignty,
            scores_without.sovereignty
        );
    }

    #[test]
    fn parse_holder_context_section() {
        let mut content = make_token_stimulus(&HEALTHY_MINT);
        let baselines_pos = content.find("\n[BASELINES]").unwrap();
        let holder_ctx = "\n[HOLDER CONTEXT]\ntop_20_analyzed: 85.3% of supply\n  lp_pools: 15.2% \u{2014} DEX liquidity, market-making\n  contracts: 60.1% \u{2014} tokens held by smart contracts (vesting, DAO, protocol), not freely tradeable\n  wallets: 10.0% \u{2014} freely tradeable by individual holders\neffective_wallet_concentration: 10.0%\nnote: High raw concentration (85%) driven by institutional/programmatic holdings (75%). Effective retail concentration is 10.0%.\n";
        content.insert_str(baselines_pos, holder_ctx);

        let m = parse(&content).expect("should parse with holder context");
        assert!(
            (m.effective_concentration.unwrap() - 10.0).abs() < 0.1,
            "effective_concentration should be 10.0%, got {:?}",
            m.effective_concentration
        );
        assert!(
            (m.contract_pct.unwrap() - 60.1).abs() < 0.1,
            "contract_pct should be 60.1%, got {:?}",
            m.contract_pct
        );
    }

    #[test]
    fn parse_holder_context_with_locker() {
        let mut content = make_token_stimulus(&HEALTHY_MINT);
        let baselines_pos = content.find("\n[BASELINES]").unwrap();
        let holder_ctx = "\n[HOLDER CONTEXT]\ntop_20_analyzed: 70.0% of supply\n  locked: 40.0% \u{2014} tokens in lock/vesting contracts, not freely tradeable\n  wallets: 30.0% \u{2014} freely tradeable by individual holders\neffective_wallet_concentration: 30.0%\n";
        content.insert_str(baselines_pos, holder_ctx);

        let m = parse(&content).expect("should parse with locker");
        assert!(
            (m.locker_pct.unwrap() - 40.0).abs() < 0.1,
            "locker_pct should be 40.0%, got {:?}",
            m.locker_pct
        );
        assert!(
            (m.effective_concentration.unwrap() - 30.0).abs() < 0.1,
            "effective_concentration should be 30.0%"
        );
    }

    #[tokio::test]
    async fn holder_context_boosts_phi_for_institutional_concentration() {
        // Token with 85% top-10 concentration — looks bad without context
        let high_conc = TestMint {
            holders: 250_000,
            top1_pct: 60.0, // high raw concentration
            top10_pct: 85.0,
            herfindahl: Some(0.45),
            age_hours: 2000,
            mint_active: false,
            freeze_active: false,
            lp: "burned",
            burned_pct: Some(0.0),
            origin: None,
        };

        // Without holder context: raw concentration penalizes phi
        let raw_stim = make_token_stimulus(&high_conc);
        let m_raw = parse(&raw_stim).unwrap();
        let scores_raw = score(&m_raw);

        // With holder context: most is contracts, effective wallet concentration is low
        let mut ctx_stim = make_token_stimulus(&high_conc);
        let baselines_pos = ctx_stim.find("\n[BASELINES]").unwrap();
        let holder_ctx = "\n[HOLDER CONTEXT]\ntop_20_analyzed: 85.0% of supply\n  contracts: 60.0% \u{2014} vesting\n  wallets: 10.0% \u{2014} retail\neffective_wallet_concentration: 10.0%\n";
        ctx_stim.insert_str(baselines_pos, holder_ctx);
        let m_ctx = parse(&ctx_stim).unwrap();
        let scores_ctx = score(&m_ctx);

        assert!(
            scores_ctx.phi > scores_raw.phi,
            "Holder context should boost phi when institutional concentration explains raw numbers: with_ctx={}, without={}",
            scores_ctx.phi,
            scores_raw.phi
        );
        assert!(
            scores_ctx.sovereignty > scores_raw.sovereignty,
            "Holder context should boost sovereignty: with_ctx={}, without={}",
            scores_ctx.sovereignty,
            scores_raw.sovereignty
        );
    }

    #[tokio::test]
    async fn holder_context_still_penalizes_real_whale_concentration() {
        // Token where most concentration IS wallets — should still score low
        let high_conc = TestMint {
            holders: 50,
            top1_pct: 70.0,
            top10_pct: 90.0,
            herfindahl: Some(0.55),
            age_hours: 48,
            mint_active: false,
            freeze_active: false,
            lp: "locked",
            burned_pct: None,
            origin: Some("pump.fun"),
        };

        let mut stim = make_token_stimulus(&high_conc);
        let baselines_pos = stim.find("\n[BASELINES]").unwrap();
        // Holder context shows most is wallets — real whale concentration
        let holder_ctx = "\n[HOLDER CONTEXT]\ntop_20_analyzed: 90.0% of supply\n  wallets: 85.0% \u{2014} freely tradeable\n  lp_pools: 5.0% \u{2014} DEX\neffective_wallet_concentration: 85.0%\n";
        stim.insert_str(baselines_pos, holder_ctx);
        let m = parse(&stim).unwrap();
        let scores = score(&m);

        assert!(
            scores.phi < 0.35,
            "85% wallet concentration should keep phi low even with holder context: phi={}",
            scores.phi
        );
        assert!(
            scores.sovereignty < 0.35,
            "85% wallet concentration should keep sovereignty low: sov={}",
            scores.sovereignty
        );
    }

    #[test]
    fn parse_trajectory_section() {
        let mut content = make_token_stimulus(&HEALTHY_MINT);
        let baselines_pos = content.find("\n[BASELINES]").unwrap();
        let trajectory = "\n[TRAJECTORY]\nclass: DYING\ndecay: 0.3500\ninterpretation: Steep conviction decay (>30%) \u{2014} holders actively leaving\n";
        content.insert_str(baselines_pos, trajectory);

        let m = parse(&content).expect("should parse with trajectory");
        assert_eq!(m.trajectory_class.as_deref(), Some("DYING"));
        assert!((m.trajectory_decay.unwrap() - 0.35).abs() < 0.001);
    }

    #[test]
    fn parse_without_trajectory_section() {
        let content = make_token_stimulus(&HEALTHY_MINT);
        let m = parse(&content).expect("should parse without trajectory");
        assert!(m.trajectory_class.is_none());
        assert!(m.trajectory_decay.is_none());
    }

    #[tokio::test]
    async fn trajectory_dying_penalizes_fidelity() {
        let base = make_token_stimulus(&MODERATE_MINT);
        let m_base = parse(&base).unwrap();
        let scores_base = score(&m_base);

        let mut dying = make_token_stimulus(&MODERATE_MINT);
        let pos = dying.find("\n[BASELINES]").unwrap();
        dying.insert_str(pos, "\n[TRAJECTORY]\nclass: DYING\ndecay: 0.3500\n");
        let m_dying = parse(&dying).unwrap();
        let scores_dying = score(&m_dying);

        assert!(
            scores_dying.fidelity < scores_base.fidelity,
            "DYING trajectory should penalize fidelity: dying={}, base={}",
            scores_dying.fidelity,
            scores_base.fidelity
        );
    }

    #[tokio::test]
    async fn trajectory_stable_boosts_sovereignty() {
        let base = make_token_stimulus(&MODERATE_MINT);
        let m_base = parse(&base).unwrap();
        let scores_base = score(&m_base);

        let mut stable = make_token_stimulus(&MODERATE_MINT);
        let pos = stable.find("\n[BASELINES]").unwrap();
        stable.insert_str(pos, "\n[TRAJECTORY]\nclass: STABLE\ndecay: 0.0800\n");
        let m_stable = parse(&stable).unwrap();
        let scores_stable = score(&m_stable);

        assert!(
            scores_stable.sovereignty > scores_base.sovereignty,
            "STABLE trajectory should boost sovereignty: stable={}, base={}",
            scores_stable.sovereignty,
            scores_base.sovereignty
        );
        assert!(
            scores_stable.fidelity > scores_base.fidelity,
            "STABLE trajectory should boost fidelity: stable={}, base={}",
            scores_stable.fidelity,
            scores_base.fidelity
        );
    }

    #[tokio::test]
    async fn trajectory_dead_heavily_penalizes() {
        let mut dead = make_token_stimulus(&MODERATE_MINT);
        let pos = dead.find("\n[BASELINES]").unwrap();
        dead.insert_str(pos, "\n[TRAJECTORY]\nclass: DEAD\ndecay: 0.9500\n");
        let m_dead = parse(&dead).unwrap();
        let scores_dead = score(&m_dead);

        let base = make_token_stimulus(&MODERATE_MINT);
        let m_base = parse(&base).unwrap();
        let scores_base = score(&m_base);

        assert!(
            scores_dead.fidelity < scores_base.fidelity - 0.05,
            "DEAD trajectory should strongly penalize fidelity: dead={}, base={}",
            scores_dead.fidelity,
            scores_base.fidelity
        );
        assert!(
            scores_dead.sovereignty < scores_base.sovereignty,
            "DEAD trajectory should penalize sovereignty: dead={}, base={}",
            scores_dead.sovereignty,
            scores_base.sovereignty
        );
    }

    /// Stablecoin-like token with active authorities should score HIGHER than
    /// the same profile without market cap (i.e., not recognized as infrastructure).
    /// This is the core discrimination fix: USDC/USDT with active freeze/mint
    /// should not be penalized — active authorities are faithful to purpose.
    const STABLECOIN_MINT: TestMint = TestMint {
        holders: 250_000,
        top1_pct: 30.0,
        top10_pct: 65.0,
        herfindahl: Some(0.12),
        age_hours: 8760, // 365 days
        mint_active: true,
        freeze_active: true,
        lp: "unsecured", // stablecoins don't have traditional LP
        burned_pct: Some(0.0),
        origin: None,
    };

    fn make_stablecoin_stimulus(mcap: &str) -> String {
        let mut s = make_token_stimulus(&STABLECOIN_MINT);
        // Insert market_cap line into [METRICS]
        let age_line = s.find("age_hours:").unwrap();
        s.insert_str(age_line, &format!("market_cap_usd: ${mcap}\n"));
        s
    }

    #[tokio::test]
    async fn established_infrastructure_not_penalized_for_active_authorities() {
        // $1B market cap → established infrastructure
        let stablecoin = make_stablecoin_stimulus("1.00B");
        let m = parse(&stablecoin).unwrap();
        assert!(is_established_infrastructure(&m));
        let scores = score(&m);

        // Should score well despite active mint+freeze
        assert!(
            scores.fidelity > 0.35,
            "stablecoin fidelity should be decent (infrastructure), got {}",
            scores.fidelity
        );
        assert!(
            scores.sovereignty > 0.25,
            "stablecoin sovereignty should not be crushed by active freeze, got {}",
            scores.sovereignty
        );
        assert!(
            scores.culture > 0.35,
            "stablecoin culture should be positive (ecosystem standard), got {}",
            scores.culture
        );
    }

    #[tokio::test]
    async fn small_token_with_active_authorities_still_penalized() {
        // Small token: active authorities + low holders → NOT established
        let small_mint = TestMint {
            holders: 500, // below 100K threshold
            top1_pct: 30.0,
            top10_pct: 65.0,
            herfindahl: Some(0.25),
            age_hours: 168, // 1 week
            mint_active: true,
            freeze_active: true,
            lp: "unsecured",
            burned_pct: Some(0.0),
            origin: Some("pump.fun"),
        };
        let small = make_token_stimulus(&small_mint);
        let m = parse(&small).unwrap();
        assert!(!is_established_infrastructure(&m));
        let scores = score(&m);

        // Should score lower than the established version
        let big = make_stablecoin_stimulus("1.00B");
        let m_big = parse(&big).unwrap();
        let scores_big = score(&m_big);

        assert!(
            scores.fidelity < scores_big.fidelity,
            "small token with active authorities should score lower fidelity: small={}, big={}",
            scores.fidelity,
            scores_big.fidelity
        );
        assert!(
            scores.sovereignty < scores_big.sovereignty,
            "small token should have lower sovereignty: small={}, big={}",
            scores.sovereignty,
            scores_big.sovereignty
        );
    }

    #[tokio::test]
    async fn established_scores_higher_than_rug() {
        // Core discrimination: stablecoin >> rug
        let stablecoin = make_stablecoin_stimulus("1.00B");
        let ms = parse(&stablecoin).unwrap();
        let scores_s = score(&ms);
        let q_stablecoin = (scores_s.fidelity
            + scores_s.phi
            + scores_s.verify
            + scores_s.culture
            + scores_s.burn
            + scores_s.sovereignty)
            / 6.0;

        let rug = make_token_stimulus(&RUG_MINT);
        let mr = parse(&rug).unwrap();
        let scores_r = score(&mr);
        let q_rug = (scores_r.fidelity
            + scores_r.phi
            + scores_r.verify
            + scores_r.culture
            + scores_r.burn
            + scores_r.sovereignty)
            / 6.0;

        assert!(
            q_stablecoin > q_rug + 0.10,
            "stablecoin should score much higher than rug: stable={q_stablecoin:.3}, rug={q_rug:.3}",
        );
    }
}
