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
        // median_hold_hours absent from this behavioral section — must be None
        assert!(m.k_median_hold_hours.is_none());
    }

    #[test]
    fn parse_median_hold_hours() {
        // K20: new stimulus field must be parseable by deterministic dog.
        let mut content = make_token_stimulus(&HEALTHY_MINT);
        let baselines_pos = content.find("\n[BASELINES]").unwrap();
        let behavioral = "\n[BEHAVIORAL]\nk_score: 0.600\ndiamond_hands: 0.500 (conviction of top holders)\norganic_growth: 0.500 (distribution quality)\nlongevity: 0.800 (age-adjusted survival)\nwallet_breakdown: 5 analyzed \u{2014} 2 accumulators, 2 holders, 1 reducers, 0 extractors\nmedian_hold_hours: 192.0 (8.0d — time from first buy to last sell or now)\n";
        content.insert_str(baselines_pos, behavioral);

        let m = parse(&content).expect("should parse median_hold_hours");
        assert!(
            m.k_median_hold_hours.is_some(),
            "median_hold_hours should be parsed"
        );
        assert!(
            (m.k_median_hold_hours.unwrap() - 192.0).abs() < 0.1,
            "expected 192.0, got {:?}",
            m.k_median_hold_hours
        );
    }

    #[tokio::test]
    async fn long_hold_time_boosts_fidelity() {
        // Holders with median >7d signal committed base — should boost fidelity.
        let mut with_hold = make_token_stimulus(&HEALTHY_MINT);
        let baselines_pos = with_hold.find("\n[BASELINES]").unwrap();
        let behavioral_long = "\n[BEHAVIORAL]\nk_score: 0.600\ndiamond_hands: 0.400 (conviction of top holders)\norganic_growth: 0.500 (distribution quality)\nlongevity: 0.800 (age-adjusted survival)\nwallet_breakdown: 5 analyzed \u{2014} 2 accumulators, 2 holders, 1 reducers, 0 extractors\nmedian_hold_hours: 336.0 (14.0d — time from first buy to last sell or now)\n";
        with_hold.insert_str(baselines_pos, behavioral_long);

        let mut without_hold = make_token_stimulus(&HEALTHY_MINT);
        let baselines_pos2 = without_hold.find("\n[BASELINES]").unwrap();
        let behavioral_none = "\n[BEHAVIORAL]\nk_score: 0.600\ndiamond_hands: 0.400 (conviction of top holders)\norganic_growth: 0.500 (distribution quality)\nlongevity: 0.800 (age-adjusted survival)\nwallet_breakdown: 5 analyzed \u{2014} 2 accumulators, 2 holders, 1 reducers, 0 extractors\n";
        without_hold.insert_str(baselines_pos2, behavioral_none);

        let m_with = parse(&with_hold).unwrap();
        let m_without = parse(&without_hold).unwrap();
        let scores_with = score(&m_with);
        let scores_without = score(&m_without);

        assert!(
            scores_with.fidelity >= scores_without.fidelity,
            "long hold_time should boost fidelity: with={}, without={}",
            scores_with.fidelity,
            scores_without.fidelity
        );
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
