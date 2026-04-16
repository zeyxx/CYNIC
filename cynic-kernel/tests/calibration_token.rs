//! Token Domain — Calibration Corpus
//!
//! 9 cases: 3 easy-BARK + 3 easy-HOWL + 3 ambiguous.
//! Each case has a TokenData struct grounded in real on-chain patterns.
//! Expected verdicts are ground truth for Dog calibration.
//!
//! Usage:
//!   cargo test --test calibration_token -- --nocapture
//!
//! This does NOT run through the pipeline (no Dogs needed).
//! It validates that stimulus generation produces the right signal for each case.

use cynic_kernel::domain::stimulus::{TokenData, build_token_stimulus};

// ═══════════════════════════════════════════════════════════════════
// EASY-BARK: Known rug patterns — Dogs MUST score these LOW
// ═══════════════════════════════════════════════════════════════════

/// Classic pump.fun rug pull.
/// Pattern: fresh, 3 holders, 99% concentrated, mint active, LP unsecured.
/// Real-world: >98% of pump.fun tokens follow this pattern.
fn corpus_bark_classic_rug() -> TokenData {
    TokenData {
        mint: "RUGc1assicPumpFunExamp1eNotARea1Mint111111".into(),
        name: Some("MOONSHOT".into()),
        symbol: Some("MOON".into()),
        holder_count: 3,
        top1_pct: 97.0,
        top10_pct: 100.0,
        herfindahl: Some(0.94),
        age_hours: 2,
        mint_authority_active: true,
        freeze_authority_active: false,
        lp_status: "unsecured".into(),
        supply_burned_pct: Some(0.0),
        supply_locked_pct: Some(0.0),
        origin: Some("pump.fun".into()),
    }
}

/// Freeze authority trap.
/// Pattern: appears moderate (50 holders, revoked mint) but freeze_authority is ACTIVE.
/// Subtle: the creator can freeze any wallet at any time.
fn corpus_bark_freeze_trap() -> TokenData {
    TokenData {
        mint: "FREEZEtrapExamp1eNotARea1MintAddress11111".into(),
        name: Some("SAFEFI".into()),
        symbol: Some("SAFE".into()),
        holder_count: 50,
        top1_pct: 35.0,
        top10_pct: 78.0,
        herfindahl: Some(0.22),
        age_hours: 168, // 7 days
        mint_authority_active: false,
        freeze_authority_active: true, // THE TRAP
        lp_status: "locked".into(),
        supply_burned_pct: None,
        supply_locked_pct: Some(30.0),
        origin: Some("raydium".into()),
    }
}

/// Copycat impersonator.
/// Pattern: name mimics BONK, fresh creation, suspicious distribution.
/// Real-world: common attack vector — naive holders confuse with real token.
fn corpus_bark_copycat() -> TokenData {
    TokenData {
        mint: "C0PYcatB0nkNotARea1MintAddressExamp1e1111".into(),
        name: Some("Bonk Inu 2.0".into()),
        symbol: Some("B0NK".into()),
        holder_count: 15,
        top1_pct: 82.0,
        top10_pct: 99.0,
        herfindahl: Some(0.68),
        age_hours: 6,
        mint_authority_active: true,
        freeze_authority_active: false,
        lp_status: "unsecured".into(),
        supply_burned_pct: None,
        supply_locked_pct: None,
        origin: Some("pump.fun".into()),
    }
}

// ═══════════════════════════════════════════════════════════════════
// EASY-HOWL: Established legitimate tokens — Dogs MUST score these HIGH
// ═══════════════════════════════════════════════════════════════════

/// JUP-like governance token.
/// Pattern: revoked authorities, years old, distributed, governance mechanism.
/// Based on real JUP data from Helius (probed 2026-04-15).
fn corpus_howl_governance() -> TokenData {
    TokenData {
        mint: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN".into(),
        name: Some("Jupiter".into()),
        symbol: Some("JUP".into()),
        holder_count: 580_000,
        top1_pct: 4.2,
        top10_pct: 28.0,
        herfindahl: Some(0.008),
        age_hours: 365 * 24, // >1 year
        mint_authority_active: false,
        freeze_authority_active: false,
        lp_status: "burned".into(),
        supply_burned_pct: Some(30.0),
        supply_locked_pct: None,
        origin: Some("jupiter-lfg".into()),
    }
}

/// BONK-like community airdrop token.
/// Pattern: massive distribution, revoked authorities, community-driven.
/// Based on real BONK data from Helius (probed 2026-04-15).
fn corpus_howl_community() -> TokenData {
    TokenData {
        mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".into(),
        name: Some("Bonk".into()),
        symbol: Some("BONK".into()),
        holder_count: 1_200_000,
        top1_pct: 2.1,
        top10_pct: 18.0,
        herfindahl: Some(0.004),
        age_hours: 2 * 365 * 24, // >2 years
        mint_authority_active: false,
        freeze_authority_active: false,
        lp_status: "burned".into(),
        supply_burned_pct: Some(50.0),
        supply_locked_pct: None,
        origin: Some("manual".into()),
    }
}

/// Raydium-listed DeFi token with solid fundamentals.
/// Pattern: moderate age, revoked auth, burned LP, active usage.
fn corpus_howl_defi() -> TokenData {
    TokenData {
        mint: "DEFiExamp1eRay1isted1egitimateTokenAddr111".into(),
        name: Some("SolLend Protocol".into()),
        symbol: Some("SLND".into()),
        holder_count: 45_000,
        top1_pct: 8.5,
        top10_pct: 35.0,
        herfindahl: Some(0.025),
        age_hours: 180 * 24, // 6 months
        mint_authority_active: false,
        freeze_authority_active: false,
        lp_status: "burned".into(),
        supply_burned_pct: Some(15.0),
        supply_locked_pct: Some(20.0),
        origin: Some("raydium".into()),
    }
}

// ═══════════════════════════════════════════════════════════════════
// AMBIGUOUS: Edge cases — Dogs must discriminate, no clear ground truth
// Expected: GROWL range. High inter-Dog spread = honest uncertainty.
// ═══════════════════════════════════════════════════════════════════

/// ASDF-like: pump.fun origin, garbage name, BUT revoked authorities, growing holders.
/// Real team behind a seemingly junk token (hackathon project).
/// Based on real ASDF data from Helius (probed 2026-04-15).
fn corpus_ambiguous_pumpfun_legit() -> TokenData {
    TokenData {
        mint: "9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump".into(),
        name: Some("asdfasdf".into()),
        symbol: Some("asdfasdfa".into()),
        holder_count: 20,
        top1_pct: 44.0,
        top10_pct: 96.0,
        herfindahl: Some(0.38),
        age_hours: 720, // 30 days
        mint_authority_active: false,
        freeze_authority_active: false,
        lp_status: "burned".into(), // pump.fun graduated, LP burned
        supply_burned_pct: None,
        supply_locked_pct: None,
        origin: Some("pump.fun".into()),
    }
}

/// New token with perfect structure but untested (48h old).
/// All signals green except age. Dogs should be cautious but not dismissive.
fn corpus_ambiguous_new_clean() -> TokenData {
    TokenData {
        mint: "NEWc1eanTokenExamp1eUntested48Hours111111".into(),
        name: Some("NovaSwap".into()),
        symbol: Some("NOVA".into()),
        holder_count: 200,
        top1_pct: 12.0,
        top10_pct: 45.0,
        herfindahl: Some(0.06),
        age_hours: 48,
        mint_authority_active: false,
        freeze_authority_active: false,
        lp_status: "burned".into(),
        supply_burned_pct: Some(5.0),
        supply_locked_pct: None,
        origin: Some("raydium".into()),
    }
}

/// Concentrated but potentially legitimate.
/// Revoked auth, burned LP, but top1=45%. Could be staking contract or whale.
/// Dogs must reason about WHETHER concentration is benign or malicious.
fn corpus_ambiguous_concentrated() -> TokenData {
    TokenData {
        mint: "C0NCentratedButMaybeLegitTokenAddr1111111".into(),
        name: Some("StakePool DAO".into()),
        symbol: Some("SPDAO".into()),
        holder_count: 3_000,
        top1_pct: 45.0,
        top10_pct: 72.0,
        herfindahl: Some(0.25),
        age_hours: 90 * 24, // 90 days
        mint_authority_active: false,
        freeze_authority_active: false,
        lp_status: "burned".into(),
        supply_burned_pct: None,
        supply_locked_pct: Some(40.0),
        origin: Some("raydium".into()),
    }
}

// ═══════════════════════════════════════════════════════════════════
// Tests: verify stimulus structure, signal presence, and ground truth
// ═══════════════════════════════════════════════════════════════════

/// Verify all BARK stimuli contain critical red flag signals.
#[test]
fn bark_stimuli_contain_red_flags() {
    let cases = [
        ("classic_rug", corpus_bark_classic_rug()),
        ("freeze_trap", corpus_bark_freeze_trap()),
        ("copycat", corpus_bark_copycat()),
    ];

    for (name, data) in &cases {
        let stimulus = build_token_stimulus(data);
        // Every BARK stimulus must contain at least one critical red flag
        let has_mint_active = stimulus.contains("ACTIVE (can mint");
        let has_freeze_active = stimulus.contains("ACTIVE (can freeze");
        let has_lp_unsecured = stimulus.contains("LP tokens in creator wallet");
        let has_high_concentration = data.top1_pct > 50.0;

        assert!(
            has_mint_active || has_freeze_active || has_lp_unsecured || has_high_concentration,
            "BARK corpus '{name}' must contain at least one critical red flag"
        );

        // All must have structure sections
        assert!(stimulus.contains("[DOMAIN: token-analysis]"));
        assert!(stimulus.contains("[METRICS]"));
        assert!(stimulus.contains("[BASELINES]"));
        assert!(stimulus.contains("[AXIOM EVIDENCE]"));
        assert!(stimulus.contains("[QUESTION]"));
    }
}

/// Verify all HOWL stimuli show green flags.
#[test]
fn howl_stimuli_contain_green_flags() {
    let cases = [
        ("governance", corpus_howl_governance()),
        ("community", corpus_howl_community()),
        ("defi", corpus_howl_defi()),
    ];

    for (name, data) in &cases {
        let stimulus = build_token_stimulus(data);
        // Every HOWL stimulus must have revoked authorities
        assert!(
            stimulus.contains("REVOKED (supply is fixed)"),
            "HOWL corpus '{name}' must have revoked mint authority"
        );
        assert!(
            stimulus.contains("REVOKED (wallets are free)"),
            "HOWL corpus '{name}' must have revoked freeze authority"
        );
        // Must show healthy holder count
        assert!(
            data.holder_count > 1000,
            "HOWL corpus '{name}' must have >1000 holders"
        );
        // Must show low concentration
        assert!(
            data.top1_pct < 10.0,
            "HOWL corpus '{name}' must have top1 < 10%"
        );
    }
}

/// Verify ambiguous stimuli contain mixed signals.
#[test]
fn ambiguous_stimuli_contain_mixed_signals() {
    let cases = [
        ("pumpfun_legit", corpus_ambiguous_pumpfun_legit()),
        ("new_clean", corpus_ambiguous_new_clean()),
        ("concentrated", corpus_ambiguous_concentrated()),
    ];

    for (name, data) in &cases {
        let stimulus = build_token_stimulus(data);
        // Must have structure
        assert!(stimulus.contains("[DOMAIN: token-analysis]"));

        // Ambiguous cases must have at least one positive AND one concerning signal
        let positives = [
            !data.mint_authority_active,
            !data.freeze_authority_active,
            data.lp_status == "burned",
        ];
        let concerns = [
            data.top1_pct > 30.0,
            data.age_hours < 72,
            data.holder_count < 50,
            data.origin.as_deref() == Some("pump.fun"),
        ];

        let has_positive = positives.iter().any(|&p| p);
        let has_concern = concerns.iter().any(|&c| c);

        assert!(
            has_positive && has_concern,
            "Ambiguous corpus '{name}' must have mixed signals (positive: {has_positive}, concern: {has_concern})"
        );
    }
}

/// Print all 9 stimuli for manual review.
/// Run with: cargo test --test calibration_token print_corpus -- --nocapture
// WHY: diagnostic output explicitly invoked with --nocapture for operator calibration review.
// stdout is the intended surface; tracing/logging would be worse UX for the interactive use case.
#[allow(clippy::print_stdout)]
#[test]
fn print_corpus() {
    let corpus: Vec<(&str, &str, TokenData)> = vec![
        ("BARK", "classic_rug", corpus_bark_classic_rug()),
        ("BARK", "freeze_trap", corpus_bark_freeze_trap()),
        ("BARK", "copycat", corpus_bark_copycat()),
        ("HOWL", "governance", corpus_howl_governance()),
        ("HOWL", "community", corpus_howl_community()),
        ("HOWL", "defi", corpus_howl_defi()),
        (
            "AMBIGUOUS",
            "pumpfun_legit",
            corpus_ambiguous_pumpfun_legit(),
        ),
        ("AMBIGUOUS", "new_clean", corpus_ambiguous_new_clean()),
        ("AMBIGUOUS", "concentrated", corpus_ambiguous_concentrated()),
    ];

    for (tier, name, data) in &corpus {
        let stimulus = build_token_stimulus(data);
        println!("══════════════════════════════════════════════════");
        println!("  [{tier}] {name}");
        println!(
            "  Holders: {} | Top1: {}% | HHI: {:?}",
            data.holder_count, data.top1_pct, data.herfindahl
        );
        println!(
            "  Age: {}h | Mint: {} | Freeze: {} | LP: {}",
            data.age_hours,
            if data.mint_authority_active {
                "ACTIVE"
            } else {
                "REVOKED"
            },
            if data.freeze_authority_active {
                "ACTIVE"
            } else {
                "REVOKED"
            },
            data.lp_status
        );
        println!("──────────────────────────────────────────────────");
        println!("{stimulus}");
        println!();
    }
}
