//! Twitter/social signal domain scorer for DeterministicDog.
//!
//! Parses enriched fields from x_ingest_daemon context.
//! Scores SIGNAL QUALITY — not text quality.
//! All 6 axioms scored — zero abstentions for this domain.

use crate::domain::dog::*;

use super::{
    ADJUST_LARGE, ADJUST_MEDIUM, ADJUST_SMALL, BURN_BASE, NEUTRAL, PHI_BASE, SOVEREIGNTY_BASE,
};

// ── Known patterns (from SKILL.md, hardcoded for deterministic speed) ──

/// Known bot accounts — identical "top memecoins" spam.
const BOT_RING: &[&str] = &[
    "ep_peter",
    "arthurbomb",
    "emcruzzz",
    "jeffersonyoan",
    "njabulobhabha",
    "ikslurpee",
    "cinisomasilela",
    "husseinalshrafy",
    "arishnap",
    "savannahhinkley",
    "jrjr1014",
    "ugochukwue73862",
    "kristenmyvida",
    "samyhaloui",
    "raulabeso",
    "enanikamal",
    "tytina0519",
];

/// Known recovery scammer accounts.
const SCAMMER_ACCOUNTS: &[&str] = &["gary_recovery_", "thelipglossguy_", "assertguard"];

/// High-signal author tiers.
const TIER_CURATED: &str = "curated";
const TIER_INFLUENCER: &str = "influencer";
const TIER_BOT_SUSPECT: &str = "bot_suspect";

/// Parsed signals from a twitter/social stimulus.
#[derive(Debug)]
pub(super) struct TwitterSignals {
    signal_score: i32,
    author_tier: String,
    _author_name: String,
    is_coordinated: bool,
    is_known_bot: bool,
    is_known_scammer: bool,
    has_cashtags: bool,
    cashtag_count: usize,
    narrative_count: usize,
    has_rug_warning: bool,
    has_onchain_ref: bool,
    has_address_or_tx: bool,
    word_count: usize,
    has_fomo_language: bool,
}

/// Parse twitter signals from stimulus content + context.
/// Returns None if this doesn't look like a twitter social signal.
pub(super) fn parse(content: &str, context: Option<&str>) -> Option<TwitterSignals> {
    // Gate: only match content that looks like ingest daemon output
    if !content.starts_with("X social signal") {
        return None;
    }

    let ctx = context.unwrap_or("");
    let content_lower = content.to_lowercase();
    let ctx_lower = ctx.to_lowercase();

    // Extract author from content: "X social signal — @author (signal N):"
    let author_name = content
        .find('@')
        .and_then(|start| {
            content[start + 1..]
                .find(|c: char| c.is_whitespace() || c == '(')
                .map(|end| content[start + 1..start + 1 + end].to_lowercase())
        })
        .unwrap_or_default();

    // Extract signal score from content: "(signal N)"
    let signal_score = content
        .find("(signal ")
        .and_then(|start| {
            content[start + 8..]
                .find(')')
                // R2-exempt: parse failure = no signal score → default 0 (filter_map pattern)
                .and_then(|end| content[start + 8..start + 8 + end].parse::<i32>().ok())
        })
        .unwrap_or(0);

    // Parse context fields
    let author_tier = extract_field(&ctx_lower, "author tier: ").unwrap_or_default();
    let is_coordinated =
        ctx_lower.contains("is_coordinated: true") || ctx_lower.contains("coordinated");
    let narratives_str = extract_field(&ctx_lower, "narratives: ").unwrap_or_default();
    let cashtags_str = extract_field(&ctx_lower, "cashtags: ").unwrap_or_default();

    let narrative_count = if narratives_str.is_empty() {
        0
    } else {
        narratives_str.split(',').count()
    };
    let cashtag_count = if cashtags_str.is_empty() || cashtags_str == "none" {
        0
    } else {
        cashtags_str
            .split('$')
            .count()
            .saturating_sub(1)
            .max(cashtags_str.split(',').count())
    };

    let is_known_bot = BOT_RING.contains(&author_name.as_str()) || author_tier == TIER_BOT_SUSPECT;
    let is_known_scammer = SCAMMER_ACCOUNTS.contains(&author_name.as_str())
        || content_lower.contains("crypto recovery")
        || content_lower.contains("recovery expert");

    // On-chain references: addresses (base58, 32+ chars), tx hashes
    let has_address_or_tx = content.split_whitespace().any(|w| {
        let clean = w.trim_matches(|c: char| !c.is_alphanumeric());
        clean.len() >= 32 && clean.chars().all(|c| c.is_alphanumeric())
    });

    let has_onchain_ref = has_address_or_tx
        || content_lower.contains("on-chain")
        || content_lower.contains("onchain")
        || content_lower.contains("tx hash")
        || content_lower.contains("wallet address");

    // FOMO language detection
    let fomo_terms = [
        "last chance",
        "before it's too late",
        "don't miss",
        "100x",
        "1000x",
        "moon",
        "next gem",
        "insiders know",
        "buy now",
        "guaranteed profit",
        "easy money",
    ];
    let has_fomo_language = fomo_terms.iter().any(|t| content_lower.contains(t));

    let words: Vec<&str> = content.split_whitespace().collect();

    Some(TwitterSignals {
        signal_score,
        author_tier,
        _author_name: author_name,
        is_coordinated,
        is_known_bot,
        is_known_scammer,
        has_cashtags: cashtag_count > 0,
        cashtag_count,
        narrative_count,
        has_rug_warning: narratives_str.contains("rug_warning")
            || content_lower.contains("rug")
            || content_lower.contains("rugged"),
        has_onchain_ref,
        has_address_or_tx,
        word_count: words.len(),
        has_fomo_language,
    })
}

/// Score twitter signals on all 6 axioms.
/// Asymmetric: warnings scored generously, hype scored skeptically.
pub(super) fn score(s: &TwitterSignals) -> AxiomScores {
    // ── Known-bad fast path: bots and scammers ──
    if s.is_known_bot || s.is_known_scammer || s.is_coordinated {
        return score_known_bad(s);
    }

    // ── FIDELITY: Does this contain a truthful, specific signal? ──
    let mut fidelity = PHI_BASE;
    if s.has_cashtags {
        fidelity += ADJUST_SMALL; // names specific tokens
    }
    if s.cashtag_count >= 2 {
        fidelity += ADJUST_SMALL; // cross-references tokens
    }
    if s.has_rug_warning {
        fidelity += ADJUST_MEDIUM; // warnings carry higher potential truth value
    }
    if s.has_address_or_tx {
        fidelity += ADJUST_MEDIUM; // cites verifiable data
    }
    if s.author_tier == TIER_CURATED {
        fidelity += ADJUST_SMALL; // human-vetted source
    }
    if s.signal_score >= 5 {
        fidelity += ADJUST_SMALL;
    }
    if s.has_fomo_language {
        fidelity -= ADJUST_MEDIUM; // manipulation = low fidelity
    }
    let fidelity = fidelity.clamp(ADJUST_SMALL, PHI_INV);
    let fidelity_reason = format!(
        "cashtags={}, rug_warning={}, address_ref={}, author={}.",
        s.cashtag_count, s.has_rug_warning, s.has_address_or_tx, s.author_tier,
    );

    // ── PHI: Structural coherence of the signal ──
    let mut phi = PHI_BASE;
    if s.narrative_count >= 2 {
        phi += ADJUST_MEDIUM; // cross-narrative = denser signal
    }
    if s.has_cashtags && s.has_rug_warning {
        phi += ADJUST_SMALL; // claim + subject = complete signal
    }
    if s.word_count > 10 && s.word_count < 100 {
        phi += ADJUST_SMALL; // proportional length
    } else if s.word_count <= 5 {
        phi -= ADJUST_MEDIUM; // too short to be coherent
    }
    let phi = phi.clamp(ADJUST_SMALL, PHI_INV);
    let phi_reason = format!(
        "narratives={}, word_count={}, cashtags+warning={}.",
        s.narrative_count,
        s.word_count,
        s.has_cashtags && s.has_rug_warning,
    );

    // ── VERIFY: Can claims be independently verified? ──
    let mut verify = NEUTRAL; // social signals are harder to verify than on-chain
    if s.has_address_or_tx {
        verify += ADJUST_LARGE; // anyone can check on-chain
    }
    if s.has_onchain_ref {
        verify += ADJUST_SMALL;
    }
    if s.signal_score >= 5 {
        verify += ADJUST_SMALL; // high-signal tweets tend to cite evidence
    }
    if s.has_fomo_language {
        verify -= ADJUST_MEDIUM; // unfalsifiable claims
    }
    let verify = verify.clamp(ADJUST_SMALL, PHI_INV);
    let verify_reason = format!(
        "address_ref={}, onchain_ref={}, signal={}.",
        s.has_address_or_tx, s.has_onchain_ref, s.signal_score,
    );

    // ── CULTURE: Does this follow legitimate analysis patterns? ──
    let mut culture = PHI_BASE;
    if s.author_tier == TIER_CURATED {
        culture += ADJUST_MEDIUM; // part of trusted community
    } else if s.author_tier == TIER_INFLUENCER {
        culture += ADJUST_SMALL;
    }
    if s.has_rug_warning {
        culture += ADJUST_SMALL; // watchdog culture
    }
    if s.has_fomo_language {
        culture -= ADJUST_LARGE; // manipulation violates norms
    }
    let culture = culture.clamp(ADJUST_SMALL, PHI_INV);
    let culture_reason = format!(
        "author_tier={}, fomo={}, rug_warning={}.",
        s.author_tier, s.has_fomo_language, s.has_rug_warning,
    );

    // ── BURN: Signal-to-noise ratio ──
    let mut burn = BURN_BASE;
    if s.has_cashtags && s.word_count < 80 {
        burn += ADJUST_SMALL; // concise + specific
    }
    if s.narrative_count >= 2 {
        burn += ADJUST_SMALL; // information-dense
    }
    if s.word_count > 150 {
        burn -= ADJUST_SMALL; // verbose
    }
    if s.has_fomo_language {
        burn -= ADJUST_MEDIUM; // noise
    }
    let burn = burn.clamp(ADJUST_SMALL, PHI_INV);
    let burn_reason = format!(
        "cashtags={}, concise={}, narratives={}.",
        s.has_cashtags,
        s.word_count < 80,
        s.narrative_count,
    );

    // ── SOVEREIGNTY: Does this preserve reader's independent judgment? ──
    let mut sovereignty = SOVEREIGNTY_BASE;
    if s.has_address_or_tx {
        sovereignty += ADJUST_SMALL; // empowers verification
    }
    if s.has_fomo_language {
        sovereignty -= ADJUST_LARGE; // short-circuits judgment
    }
    if s.has_rug_warning && s.has_cashtags {
        sovereignty += ADJUST_SMALL; // specific warning = informed decision
    }
    let sovereignty = sovereignty.clamp(ADJUST_SMALL, PHI_INV);
    let sovereignty_reason = format!(
        "address_ref={}, fomo={}, specific_warning={}.",
        s.has_address_or_tx,
        s.has_fomo_language,
        s.has_rug_warning && s.has_cashtags,
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
        abstentions: vec![], // All 6 axioms judged
    }
}

/// Fast path for known-bad actors: bots, scammers, coordinated campaigns.
fn score_known_bad(s: &TwitterSignals) -> AxiomScores {
    let reason = if s.is_known_scammer {
        "Known recovery scammer account."
    } else if s.is_known_bot {
        "Known bot ring member."
    } else {
        "Coordinated campaign detected."
    };

    AxiomScores {
        fidelity: ADJUST_SMALL,
        phi: ADJUST_SMALL,
        verify: ADJUST_SMALL,
        culture: ADJUST_SMALL,
        burn: ADJUST_SMALL,
        sovereignty: ADJUST_SMALL,
        prompt_tokens: 0,
        completion_tokens: 0,
        thinking_tokens: 0,
        reasoning: AxiomReasoning {
            fidelity: reason.into(),
            phi: reason.into(),
            verify: reason.into(),
            culture: reason.into(),
            burn: reason.into(),
            sovereignty: reason.into(),
        },
        abstentions: vec![],
    }
}

/// Extract a field value from context string.
/// Looks for "key: value" and returns the value until end-of-line or period.
fn extract_field(ctx: &str, key: &str) -> Option<String> {
    ctx.find(key).map(|start| {
        let rest = &ctx[start + key.len()..];
        let end = rest.find(['.', '\n']).unwrap_or(rest.len());
        rest[..end].trim().to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_signal(content: &str, context: &str) -> TwitterSignals {
        parse(content, Some(context)).expect("should parse as twitter signal")
    }

    #[test]
    fn parse_rug_warning() {
        let s = make_signal(
            "X social signal — @gcrtrd (signal 7): $BEDROCK slow rugged, every token that assigned fees is dead.",
            "Cashtags: $BEDROCK, $ASDFASDFA. Narratives: rug_warning, onchain. Author tier: curated.",
        );
        assert_eq!(s.signal_score, 7);
        assert_eq!(s.author_tier, "curated");
        assert!(s.has_rug_warning);
        assert!(s.has_cashtags);
        assert_eq!(s.narrative_count, 2);
        assert!(!s.is_known_bot);
    }

    #[test]
    fn parse_bot_spam() {
        let s = make_signal(
            "X social signal — @EP_PETER (signal 1): Top 5 Strongest Memecoins on Solana!",
            "Cashtags: $WIF, $POPCAT. Narratives: pump_hype. Author tier: bot_suspect. is_coordinated: true.",
        );
        assert!(s.is_known_bot);
        assert!(s.is_coordinated);
        assert_eq!(s.author_tier, "bot_suspect");
    }

    #[test]
    fn parse_rejects_non_twitter() {
        assert!(parse("[DOMAIN: token-analysis]\n[METRICS]", None).is_none());
        assert!(parse("Just a normal sentence.", None).is_none());
    }

    #[test]
    fn rug_warning_scores_higher_than_bot_spam() {
        let rug = make_signal(
            "X social signal — @gcrtrd (signal 7): $BEDROCK slow rugged.",
            "Cashtags: $BEDROCK. Narratives: rug_warning. Author tier: curated.",
        );
        let bot = make_signal(
            "X social signal — @EP_PETER (signal 1): Top 5 memecoins!",
            "Narratives: pump_hype. Author tier: bot_suspect. is_coordinated: true.",
        );
        let rug_scores = score(&rug);
        let bot_scores = score(&bot);

        // Rug warning should score significantly higher than bot spam
        let rug_q = (rug_scores.fidelity
            + rug_scores.phi
            + rug_scores.verify
            + rug_scores.culture
            + rug_scores.burn
            + rug_scores.sovereignty)
            / 6.0;
        let bot_q = (bot_scores.fidelity
            + bot_scores.phi
            + bot_scores.verify
            + bot_scores.culture
            + bot_scores.burn
            + bot_scores.sovereignty)
            / 6.0;

        assert!(
            rug_q > bot_q + 0.20,
            "rug warning Q ({rug_q:.3}) should be >0.20 above bot spam Q ({bot_q:.3})"
        );
    }

    #[test]
    fn known_bot_scores_floor() {
        let bot = make_signal(
            "X social signal — @EP_PETER (signal 1): Top memecoins on Solana!",
            "Author tier: bot_suspect. is_coordinated: true.",
        );
        let scores = score(&bot);
        assert!(
            scores.fidelity <= ADJUST_SMALL + 0.001,
            "known bot fidelity should be floor, got {}",
            scores.fidelity
        );
    }

    #[test]
    fn fomo_language_penalizes() {
        let fomo = make_signal(
            "X social signal — @shill (signal 3): Last chance to buy $TOKEN before it's too late! 100x guaranteed!",
            "Cashtags: $TOKEN. Narratives: pump_hype. Author tier: organic.",
        );
        let scores = score(&fomo);
        assert!(
            scores.sovereignty < SOVEREIGNTY_BASE,
            "FOMO should penalize sovereignty, got {}",
            scores.sovereignty
        );
        assert!(
            scores.fidelity < PHI_BASE,
            "FOMO should penalize fidelity, got {}",
            scores.fidelity
        );
    }

    #[test]
    fn address_reference_boosts_verify() {
        let with_addr = make_signal(
            "X social signal — @analyst (signal 5): Dev wallet 7xK3abcdefghijklmnopqrstuvwxyz12345 just drained LP.",
            "Cashtags: $TOKEN. Narratives: rug_warning. Author tier: organic.",
        );
        let scores = score(&with_addr);
        assert!(
            scores.verify > NEUTRAL + ADJUST_MEDIUM,
            "address reference should boost verify, got {}",
            scores.verify
        );
    }

    #[test]
    fn zero_abstentions() {
        let s = make_signal(
            "X social signal — @test (signal 3): Some content about $TOKEN.",
            "Cashtags: $TOKEN. Narratives: ecosystem. Author tier: organic.",
        );
        let scores = score(&s);
        assert!(
            scores.abstentions.is_empty(),
            "twitter scorer should have zero abstentions, got {:?}",
            scores.abstentions
        );
    }
}
