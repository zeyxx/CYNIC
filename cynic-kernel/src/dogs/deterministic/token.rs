//! Token-analysis domain scorer for DeterministicDog.
//!
//! Parses structured [METRICS] from build_token_stimulus() output.
//! Scores SUBSTANCE (on-chain signals) not FORM (text structure).
//! All 6 axioms scored — zero abstentions for this domain.

use crate::domain::dog::*;

use super::{ADJUST_MEDIUM, ADJUST_SMALL, PHI_BASE, SOVEREIGNTY_BASE};

/// Parsed metrics from a token-analysis stimulus.
#[derive(Debug)]
pub(super) struct TokenMetrics {
    holder_data_available: bool,
    holders: u64,
    top1_pct: f64,
    top10_pct: f64,
    herfindahl: Option<f64>,
    age_hours: u64,
    mint_authority_active: bool,
    freeze_authority_active: bool,
    lp_burned: bool,
    lp_locked: bool,
    supply_burned_pct: Option<f64>,
    origin_pump_fun: bool,
    // K-Score behavioral fields (from [BEHAVIORAL] section)
    k_score: Option<f64>,
    k_diamond_hands: Option<f64>,
    k_wallets_analyzed: u32,
    k_accumulators: u32,
    k_extractors: u32,
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
        top10_pct: 0.0,
        herfindahl: None,
        age_hours: 0,
        mint_authority_active: false,
        freeze_authority_active: false,
        lp_burned: false,
        lp_locked: false,
        supply_burned_pct: None,
        origin_pump_fun: false,
        k_score: None,
        k_diamond_hands: None,
        k_wallets_analyzed: 0,
        k_accumulators: 0,
        k_extractors: 0,
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

    Some(m)
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
    let mut fidelity: f64 = PHI_BASE;
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
    // K-Score: diamond hands = genuine conviction
    if let Some(dh) = m.k_diamond_hands {
        if dh > 0.6 {
            fidelity += ADJUST_MEDIUM;
        } else if dh < 0.2 {
            fidelity -= ADJUST_SMALL;
        }
    }
    let fidelity = fidelity.clamp(ADJUST_SMALL, PHI_INV);
    let fidelity_reason = format!(
        "mint_authority={}, freeze_authority={}, diamond_hands={}.",
        if m.mint_authority_active {
            "ACTIVE (red flag)"
        } else {
            "revoked"
        },
        if m.freeze_authority_active {
            "ACTIVE (red flag)"
        } else {
            "revoked"
        },
        m.k_diamond_hands
            .map(|d| format!("{d:.3}"))
            .unwrap_or_else(|| "n/a".into()),
    );

    // ── PHI: Structural harmony of holder distribution ──
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
        if m.top1_pct < 15.0 {
            phi += ADJUST_SMALL;
        } else if m.top1_pct > 50.0 {
            phi -= ADJUST_MEDIUM;
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
    // K-Score: organic growth reflects distribution quality beyond static HHI
    if let Some(ks) = m.k_score {
        if ks > 0.5 {
            phi += ADJUST_MEDIUM;
        } else if ks < 0.3 {
            phi -= ADJUST_SMALL;
        }
    }
    let phi = phi.clamp(ADJUST_SMALL, PHI_INV);
    let phi_reason = if m.holder_data_available {
        format!(
            "holders={}+, top1={:.1}%, HHI={}.",
            m.holders,
            m.top1_pct,
            m.herfindahl
                .map(|h| format!("{h:.3}"))
                .unwrap_or_else(|| "n/a".into()),
        )
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
    let verify = verify.clamp(ADJUST_SMALL, PHI_INV);
    let verify_reason = format!(
        "On-chain verifiable. LP={}, age={}h.",
        if m.lp_burned {
            "burned"
        } else if m.lp_locked {
            "locked"
        } else {
            "unsecured"
        },
        m.age_hours,
    );

    // ── CULTURE: Does it follow established token standards? ──
    // Falsify: new cultural norm not yet in the heuristic set.
    let mut culture: f64 = PHI_BASE;
    if m.lp_burned {
        culture += ADJUST_MEDIUM; // follows established practice
    } else if !m.lp_locked {
        culture -= ADJUST_SMALL; // deviates from standard
    }
    if !m.mint_authority_active {
        culture += ADJUST_SMALL;
    }
    if !m.freeze_authority_active {
        culture += ADJUST_SMALL;
    }
    if m.origin_pump_fun {
        culture -= ADJUST_SMALL; // 98.6% rug rate baseline (Solidus Labs 2025)
    }
    if m.holders > 100 {
        culture += ADJUST_SMALL;
    }
    let culture = culture.clamp(ADJUST_SMALL, PHI_INV);
    let culture_reason = format!(
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
    );

    // ── BURN: Efficiency, minimal waste ──
    // Falsify: token with active mint for legitimate yield distribution.
    let mut burn: f64 = PHI_BASE;
    if let Some(burned_pct) = m.supply_burned_pct {
        // Gradient: up to +ADJUST_SMALL for 50%+ supply burned
        burn += ADJUST_SMALL * (burned_pct / 50.0).min(1.0);
    }
    if !m.mint_authority_active && !m.freeze_authority_active {
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
        if !m.mint_authority_active && !m.freeze_authority_active {
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
    // Falsify: legitimate DAO with top1% held by treasury multisig.
    let mut sovereignty: f64 = SOVEREIGNTY_BASE;
    if m.holder_data_available {
        if m.top1_pct < 15.0 {
            sovereignty += ADJUST_MEDIUM;
        } else if m.top1_pct > 50.0 {
            sovereignty -= ADJUST_MEDIUM;
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
    // Authority checks always apply (from getAsset, not holder data)
    if m.freeze_authority_active {
        sovereignty -= ADJUST_MEDIUM;
    }
    if m.mint_authority_active {
        sovereignty -= ADJUST_SMALL;
    }
    // K-Score: extractor dominance = centralized exit
    if m.k_wallets_analyzed > 0 && m.k_extractors > m.k_accumulators {
        sovereignty -= ADJUST_SMALL;
    }
    let sovereignty = sovereignty.clamp(ADJUST_SMALL, PHI_INV);
    let sovereignty_reason = format!(
        "{}freeze={}, extractors_vs_acc={}/{}.",
        if m.holder_data_available {
            format!("top1={:.1}%, holders={}+, ", m.top1_pct, m.holders)
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
    async fn kscore_boosts_healthy_token() {
        let mut with_ks = make_token_stimulus(&HEALTHY_MINT);
        let baselines_pos = with_ks.find("\n[BASELINES]").unwrap();
        let behavioral = "\n[BEHAVIORAL]\nk_score: 0.700\ndiamond_hands: 0.800 (conviction)\nwallet_breakdown: 10 analyzed \u{2014} 5 accumulators, 3 holders, 1 reducers, 1 extractors\n";
        with_ks.insert_str(baselines_pos, behavioral);
        let m_with = parse(&with_ks).unwrap();
        let scores_with = score(&m_with);

        let without = make_token_stimulus(&HEALTHY_MINT);
        let m_without = parse(&without).unwrap();
        let scores_without = score(&m_without);

        assert!(
            scores_with.fidelity >= scores_without.fidelity,
            "diamond_hands should boost fidelity: with={}, without={}",
            scores_with.fidelity,
            scores_without.fidelity
        );
        assert!(
            scores_with.phi >= scores_without.phi,
            "K-Score should boost phi: with={}, without={}",
            scores_with.phi,
            scores_without.phi
        );
    }

    #[tokio::test]
    async fn extractor_dominance_penalizes_sovereignty() {
        let mut content = make_token_stimulus(&MODERATE_MINT);
        let baselines_pos = content.find("\n[BASELINES]").unwrap();
        let behavioral = "\n[BEHAVIORAL]\nk_score: 0.200\ndiamond_hands: 0.150 (conviction)\nwallet_breakdown: 8 analyzed \u{2014} 1 accumulators, 1 holders, 1 reducers, 5 extractors\n";
        content.insert_str(baselines_pos, behavioral);
        let m_ext = parse(&content).unwrap();
        let scores_ext = score(&m_ext);

        let without = make_token_stimulus(&MODERATE_MINT);
        let m_without = parse(&without).unwrap();
        let scores_without = score(&m_without);

        assert!(
            scores_ext.sovereignty <= scores_without.sovereignty,
            "Extractor dominance should penalize sovereignty: ext={}, base={}",
            scores_ext.sovereignty,
            scores_without.sovereignty
        );
    }
}
