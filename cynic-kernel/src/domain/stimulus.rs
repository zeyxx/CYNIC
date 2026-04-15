//! Stimulus Protocol — structured bridge between raw data and epistemic judgment.
//!
//! The stimulus IS the prompt for the Dogs. Its quality bounds CYNIC's judgment quality.
//! Garbage stimulus → noisy scores → high disagreement → quarantined → no crystals.
//!
//! Design principles:
//! - METRICS: factual data without interpretation (Dogs interpret)
//! - BASELINES: reference points so Dogs know what "normal" looks like
//! - AXIOM EVIDENCE: explicit mapping data→axiom (reduces inter-Dog spread)
//! - QUESTION: frames the judgment (not "evaluate this text" but "is this legitimate?")
//!
//! Each domain has its own builder. The format is universal, the content domain-specific.

/// Build a structured token analysis stimulus from on-chain metrics.
///
/// The caller (screener, API consumer) fetches data from Helius/DexScreener/etc.
/// This function formats it so Dogs can judge rigorously with minimal disagreement.
///
/// All fields are strings to keep the builder agnostic to data source.
pub fn build_token_stimulus(data: &TokenData) -> String {
    let mut s = String::with_capacity(1500);

    s.push_str("[DOMAIN: token-analysis]\n\n");

    // ── Metrics: raw facts, no interpretation ──
    s.push_str("[METRICS]\n");
    s.push_str(&format!("mint: {}\n", data.mint));
    if let Some(ref name) = data.name {
        s.push_str(&format!("name: {name}\n"));
    }
    if let Some(ref symbol) = data.symbol {
        s.push_str(&format!("symbol: {symbol}\n"));
    }
    s.push_str(&format!("holders: {}\n", data.holder_count));
    s.push_str(&format!("top_1_wallet_pct: {}%\n", data.top1_pct));
    s.push_str(&format!("top_10_wallets_pct: {}%\n", data.top10_pct));
    if let Some(hhi) = data.herfindahl {
        s.push_str(&format!("herfindahl_index: {hhi:.3}\n"));
    }
    s.push_str(&format!("age_hours: {}\n", data.age_hours));
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
            other => other,
        }
    ));
    if let Some(burned) = data.supply_burned_pct {
        s.push_str(&format!("supply_burned_pct: {burned}%\n"));
    }
    if let Some(locked) = data.supply_locked_pct {
        s.push_str(&format!("supply_locked_pct: {locked}%\n"));
    }
    if let Some(ref origin) = data.origin {
        s.push_str(&format!("origin: {origin}\n"));
    }

    // ── Baselines: what "normal" looks like ──
    s.push_str("\n[BASELINES]\n");
    s.push_str("healthy_token: holders>100, top_1<15%, herfindahl<0.15, age>30d, mint_authority=revoked, lp=burned\n");
    s.push_str("moderate_risk: holders 20-100, top_1 15-40%, age 1-30d, lp=locked\n");
    s.push_str(
        "high_risk_rug: holders<20, top_1>50%, age<24h, mint_authority=active, lp=unsecured\n",
    );
    s.push_str("note: 98.6% of pump.fun tokens are rug pulls (Solidus Labs 2025). Baseline for new tokens is skepticism, not trust.\n");

    // ── Axiom evidence: what to evaluate per axiom ──
    s.push_str("\n[AXIOM EVIDENCE]\n");
    s.push_str("FIDELITY: Does the on-chain state match what a legitimate project would show? Is the token faithful to its claimed purpose?\n");
    s.push_str("PHI: Is the holder distribution proportional? Is the tokenomics structure harmonious or concentrated?\n");
    s.push_str("VERIFY: Can these metrics be independently verified on-chain? Are there verifiable red flags or green flags?\n");
    s.push_str("CULTURE: Does this token follow established Solana token standards? Is the authority model consistent with good practices?\n");
    s.push_str("BURN: Is the token efficiently structured? Burned supply, minimal waste, no unnecessary authorities retained?\n");
    s.push_str("SOVEREIGNTY: Is control distributed or concentrated? Can individual holders act freely without one wallet dominating?\n");

    // ── Question ──
    s.push_str("\n[QUESTION]\n");
    s.push_str("Based on the on-chain metrics above, evaluate this token's legitimacy and risk level. Score each axiom from 0.05 to 0.618.\n");

    s
}

/// On-chain token data for stimulus building.
/// All fields populated by the caller from Helius/DexScreener/CultScreener.
/// The kernel does not fetch external data — callers bring the data, kernel judges.
#[derive(Debug)]
pub struct TokenData {
    pub mint: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub holder_count: u32,
    pub top1_pct: f64,
    pub top10_pct: f64,
    pub herfindahl: Option<f64>,
    pub age_hours: u64,
    pub mint_authority_active: bool,
    pub freeze_authority_active: bool,
    /// "burned", "locked", or "unsecured"
    pub lp_status: String,
    pub supply_burned_pct: Option<f64>,
    pub supply_locked_pct: Option<f64>,
    /// "pump.fun", "raydium", "manual", etc.
    pub origin: Option<String>,
}

/// Build a structured dev commit stimulus.
pub fn build_dev_stimulus(hash: &str, message: &str, files_changed: &[String]) -> String {
    let mut s = String::with_capacity(500);
    s.push_str("[DOMAIN: dev-commit]\n\n");

    s.push_str("[METRICS]\n");
    s.push_str(&format!("commit: {hash}\n"));
    s.push_str(&format!("message: {message}\n"));
    s.push_str(&format!("files_changed: {}\n", files_changed.len()));
    for f in files_changed.iter().take(10) {
        s.push_str(&format!("  - {f}\n"));
    }

    s.push_str("\n[BASELINES]\n");
    s.push_str("good_commit: clear message, <5 files, single concern, tests included\n");
    s.push_str("risky_commit: vague message, >10 files, mixed concerns, no tests\n");

    s.push_str("\n[AXIOM EVIDENCE]\n");
    s.push_str("FIDELITY: Does the commit message accurately describe the change?\n");
    s.push_str("PHI: Is the change proportional (right size for its scope)?\n");
    s.push_str("VERIFY: Are tests included or referenced? Is the change testable?\n");
    s.push_str("CULTURE: Does it follow conventional commits, project naming, patterns?\n");
    s.push_str("BURN: Is it minimal? No dead code, no unrelated changes, no waste?\n");
    s.push_str("SOVEREIGNTY: Does it preserve system independence? No new vendor lock-in?\n");

    s.push_str("\n[QUESTION]\n");
    s.push_str("Evaluate this commit's quality and rigor. Score each axiom from 0.05 to 0.618.\n");

    s
}

/// Build a structured governance proposal stimulus.
pub fn build_governance_stimulus(
    proposal: &str,
    community_name: &str,
    holder_count: u32,
    votes_for: u32,
    votes_against: u32,
) -> String {
    let mut s = String::with_capacity(800);
    s.push_str("[DOMAIN: governance-proposal]\n\n");

    s.push_str("[METRICS]\n");
    s.push_str(&format!("community: {community_name}\n"));
    s.push_str(&format!("holders: {holder_count}\n"));
    s.push_str(&format!("votes_for: {votes_for}\n"));
    s.push_str(&format!("votes_against: {votes_against}\n"));
    let total = votes_for + votes_against;
    if total > 0 {
        s.push_str(&format!(
            "participation_rate: {:.1}%\n",
            (total as f64 / holder_count.max(1) as f64) * 100.0
        ));
    }
    s.push_str(&format!("proposal:\n{proposal}\n"));

    s.push_str("\n[BASELINES]\n");
    s.push_str("legitimate_governance: participation>10%, clear proposal, balanced debate\n");
    s.push_str("sybil_risk: participation<1% OR single-wallet majority\n");

    s.push_str("\n[AXIOM EVIDENCE]\n");
    s.push_str("FIDELITY: Is the proposal honest about its intent and consequences?\n");
    s.push_str("PHI: Is the voting balanced and proportional to the community?\n");
    s.push_str("VERIFY: Can the proposal's claims be verified? Is the vote auditable?\n");
    s.push_str("CULTURE: Does the proposal respect the community's established norms?\n");
    s.push_str("BURN: Is the proposal efficient? Does it avoid unnecessary complexity?\n");
    s.push_str("SOVEREIGNTY: Does it preserve individual holder agency? No capture?\n");

    s.push_str("\n[QUESTION]\n");
    s.push_str("Evaluate this governance proposal's quality and legitimacy. Score each axiom from 0.05 to 0.618.\n");

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
            holder_count: 20,
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
        };

        let stimulus = build_token_stimulus(&data);

        assert!(stimulus.contains("[DOMAIN: token-analysis]"));
        assert!(stimulus.contains("[METRICS]"));
        assert!(stimulus.contains("[BASELINES]"));
        assert!(stimulus.contains("[AXIOM EVIDENCE]"));
        assert!(stimulus.contains("[QUESTION]"));
        assert!(stimulus.contains("holders: 20"));
        assert!(stimulus.contains("top_1_wallet_pct: 94%"));
        assert!(stimulus.contains("ACTIVE (can mint more tokens)"));
        assert!(stimulus.contains("NO — LP tokens in creator wallet"));
        assert!(stimulus.contains("98.6%"));
    }

    #[test]
    fn token_stimulus_optional_fields_omitted() {
        let data = TokenData {
            mint: "test".into(),
            name: None,
            symbol: None,
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
        };

        let stimulus = build_token_stimulus(&data);
        assert!(!stimulus.contains("name:"));
        assert!(!stimulus.contains("herfindahl_index:"));
        assert!(!stimulus.contains("supply_burned_pct:"));
    }

    #[test]
    fn dev_stimulus_contains_structure() {
        let stimulus = build_dev_stimulus(
            "abc1234",
            "feat(inference): add timeout handling",
            &[
                "src/dogs/inference.rs".into(),
                "tests/inference_test.rs".into(),
            ],
        );

        assert!(stimulus.contains("[DOMAIN: dev-commit]"));
        assert!(stimulus.contains("files_changed: 2"));
        assert!(stimulus.contains("inference.rs"));
    }

    #[test]
    fn governance_stimulus_computes_participation() {
        let stimulus = build_governance_stimulus("Burn 10% of treasury", "$ASDF", 100, 15, 5);

        assert!(stimulus.contains("participation_rate: 20.0%"));
        assert!(stimulus.contains("[DOMAIN: governance-proposal]"));
    }

    // ── Calibration test: known-rug should produce consistent low stimulus ──
    #[test]
    fn known_rug_stimulus_contains_red_flags() {
        let data = TokenData {
            mint: "rugpull123".into(),
            name: Some("SCAM".into()),
            symbol: Some("RUG".into()),
            holder_count: 3,
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
        };

        let stimulus = build_token_stimulus(&data);
        // Every metric should map to "high_risk_rug" baseline
        assert!(stimulus.contains("holders: 3"));
        assert!(stimulus.contains("99%"));
        assert!(stimulus.contains("ACTIVE (can mint"));
        assert!(stimulus.contains("ACTIVE (can freeze"));
        assert!(stimulus.contains("NO — LP tokens in creator wallet"));
    }
}
