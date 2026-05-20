//! Phone-number domain scorer for DeterministicDog.
//!
//! Parses structured [METRICS] from build_phone_stimulus() output.
//! Scores SUBSTANCE (community signal quality) not FORM (text structure).
//! All 6 axioms scored — zero abstentions for this domain.

use crate::domain::dog::*;

use super::{ADJUST_MEDIUM, ADJUST_SMALL, PHI_BASE, SOVEREIGNTY_BASE};

/// Parsed metrics from a phone-number stimulus.
#[derive(Debug)]
pub(super) struct PhoneMetrics {
    total_events: u64,
    reporter_count: u32,
    mean_reporter_trust: f64,
    age_days: u32,
    spam_score: f64,
    label_legitimate: u32,
    label_nuisance: u32,
    label_scam: u32,
    label_unknown: u32,
    contestation_count: u32,
    owner_verified: bool,
    challenge_pass_rate: Option<f64>,
}

/// Extract phone metrics from a formatted stimulus string.
/// Returns None if content is not a phone-number stimulus.
pub(super) fn parse(content: &str) -> Option<PhoneMetrics> {
    if !content.contains("[DOMAIN: phone-number]") {
        return None;
    }

    // Find [METRICS] section
    let metrics_start = content.find("[METRICS]")?;
    let section_rest = &content[metrics_start..];
    let metrics_end = section_rest[9..] // skip "[METRICS]"
        .find("\n[")
        .map(|i| metrics_start + 9 + i)
        .unwrap_or(content.len());
    let section = &content[metrics_start..metrics_end];

    let mut m = PhoneMetrics {
        total_events: 0,
        reporter_count: 0,
        mean_reporter_trust: 0.5,
        age_days: 0,
        spam_score: 0.5,
        label_legitimate: 0,
        label_nuisance: 0,
        label_scam: 0,
        label_unknown: 0,
        contestation_count: 0,
        owner_verified: false,
        challenge_pass_rate: None,
    };

    for line in section.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("total_events: ") {
            m.total_events = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("reporter_count: ") {
            m.reporter_count = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("mean_reporter_trust: ") {
            m.mean_reporter_trust = v.trim().parse().unwrap_or(0.5);
        } else if let Some(v) = line.strip_prefix("age_days: ") {
            m.age_days = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("spam_score: ") {
            m.spam_score = v.trim().parse().unwrap_or(0.5);
        } else if let Some(v) = line.strip_prefix("labels: ") {
            // Parse: "legitimate=3 nuisance=30 scam=12 unknown=2"
            for part in v.split_whitespace() {
                if let Some(n) = part.strip_prefix("legitimate=") {
                    m.label_legitimate = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix("nuisance=") {
                    m.label_nuisance = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix("scam=") {
                    m.label_scam = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix("unknown=") {
                    m.label_unknown = n.parse().unwrap_or(0);
                }
            }
        } else if let Some(v) = line.strip_prefix("contestation_count: ") {
            m.contestation_count = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("owner_verified: ") {
            m.owner_verified = v.trim() == "true";
        } else if let Some(v) = line.strip_prefix("challenge_pass_rate: ") {
            let v = v.trim();
            if v != "N/A" {
                // Parse "15.0%" → 0.15
                if let Ok(pct) = v.trim_end_matches('%').trim().parse::<f64>() {
                    m.challenge_pass_rate = Some(pct / 100.0);
                }
            }
        }
    }

    Some(m)
}

/// Score a phone-number stimulus on all 6 axioms using community report metrics.
/// No abstentions — every axiom has deterministic signals from PhoneData.
///
/// Falsification notes per axiom:
/// - FIDELITY fails for coordinated report bombing (fake trust signals).
/// - PHI fails for numbers with automated dialing (skewed reporter/event ratio).
/// - VERIFY fails when owner verification is gamed through fraudulent contestation.
/// - CULTURE fails for numbers that recently switched from legitimate to spam use.
/// - BURN fails for numbers with a mix of nuisance and legitimate labels (ambiguous signal).
/// - SOVEREIGNTY fails for verified-owner numbers that still run scam campaigns.
pub(super) fn score(m: &PhoneMetrics) -> AxiomScores {
    // ── FIDELITY: Is the spam consensus trustworthy? ──
    // Falsify: coordinated fake reports inflate reporter_count with low trust.
    let fidelity: f64 = if m.reporter_count >= 10 && m.mean_reporter_trust > 0.7 {
        0.85 // strong high-trust consensus
    } else if m.reporter_count >= 5 && m.mean_reporter_trust > 0.5 {
        0.60 // moderate consensus — some trust
    } else {
        0.20 // insufficient reporters or low trust
    };
    let fidelity = fidelity.clamp(ADJUST_SMALL, PHI_INV);
    let fidelity_reason = format!(
        "reporter_count={}, mean_reporter_trust={:.3}.",
        m.reporter_count, m.mean_reporter_trust
    );

    // ── PHI: Reporter engagement proportional to event volume? ──
    // Good ratio = many reporters per event = engaged human reporters, not bots.
    // Falsify: automated dialing system generates thousands of events with few reporters.
    let phi: f64 = if m.total_events == 0 {
        PHI_BASE
    } else {
        let ratio = m.reporter_count as f64 / m.total_events as f64;
        if ratio >= 0.5 {
            PHI_BASE + ADJUST_MEDIUM + ADJUST_SMALL // very high engagement
        } else if ratio >= 0.2 {
            PHI_BASE + ADJUST_MEDIUM // healthy engagement
        } else if ratio >= 0.05 {
            PHI_BASE // moderate engagement — neutral
        } else {
            PHI_BASE - ADJUST_MEDIUM // very low ratio — spam bot pattern
        }
    };
    let phi = phi.clamp(ADJUST_SMALL, PHI_INV);
    let phi_reason = if m.total_events > 0 {
        let ratio = m.reporter_count as f64 / m.total_events as f64;
        format!(
            "reporter_count={}, total_events={}, ratio={:.3}.",
            m.reporter_count, m.total_events, ratio
        )
    } else {
        "total_events=0 — no engagement data.".into()
    };

    // ── VERIFY: Can reports be independently verified? ──
    // Base above neutral: community reports have inherent verifiability (real callers).
    // Falsify: fake owner verification through fraudulent identity documents.
    let mut verify: f64 = 0.50; // base — community-sourced data is inherently somewhat verifiable
    if m.owner_verified {
        verify += ADJUST_MEDIUM; // owner confirmed = auditable
    }
    if m.contestation_count > 0 {
        verify += ADJUST_SMALL; // active dispute = real engagement, not phantom data
    }
    if m.challenge_pass_rate.is_some() {
        verify += ADJUST_SMALL; // challenge data exists = mechanically testable
    }
    let verify = verify.clamp(ADJUST_SMALL, PHI_INV);
    let verify_reason = format!(
        "owner_verified={}, contestation_count={}, challenge_data={}.",
        m.owner_verified,
        m.contestation_count,
        m.challenge_pass_rate.is_some()
    );

    // ── CULTURE: Follows expected calling norms? ──
    // Falsify: new numbers switching from spam to legitimate after reputation reset.
    let culture: f64 = if m.spam_score < 0.3 && m.age_days > 30 {
        0.80 // long-lived legitimate = follows cultural norms
    } else if m.spam_score > 0.7 && m.age_days < 7 {
        0.70 // fresh + high spam = classic throwaway spam culture (inverse: confirms scam pattern)
    } else if m.spam_score < 0.2 {
        0.75 // very clean number
    } else if m.spam_score > 0.8 {
        0.15 // confirmed heavy spam
    } else {
        PHI_BASE // ambiguous — neutral
    };
    let culture = culture.clamp(ADJUST_SMALL, PHI_INV);
    let total_labels = m.label_legitimate + m.label_nuisance + m.label_scam + m.label_unknown;
    let culture_reason = format!(
        "spam_score={:.3}, age_days={}, labels(legit={}/nuisance={}/scam={}/unknown={}/total={}).",
        m.spam_score,
        m.age_days,
        m.label_legitimate,
        m.label_nuisance,
        m.label_scam,
        m.label_unknown,
        total_labels,
    );

    // ── BURN: Is the signal clear and efficient? ──
    // Clear signal = easy decision = high burn (efficient use of judgment).
    // Noisy signal (low reporter count or ambiguous labels) = low burn.
    // Reporter count check takes priority — insufficient data = noisy regardless of score.
    // Falsify: highly-reported ambiguous number where spam_score ~0.5 — unclear despite data.
    let burn: f64 = if m.reporter_count < 5 {
        0.30 // too few reporters — signal unreliable regardless of score
    } else if m.spam_score > 0.8 || m.spam_score < 0.2 {
        0.90 // extremely clear signal — efficient judgment
    } else if m.spam_score > 0.65 || m.spam_score < 0.35 {
        0.70 // fairly clear
    } else {
        0.45 // ambiguous middle — some confidence but not clear
    };
    let burn = burn.clamp(ADJUST_SMALL, PHI_INV);
    let burn_reason = format!(
        "spam_score={:.3} ({}), reporter_count={}.",
        m.spam_score,
        if m.reporter_count < 5 {
            "insufficient-data"
        } else if m.spam_score > 0.65 {
            "clear-spam"
        } else if m.spam_score < 0.35 {
            "clear-legit"
        } else {
            "ambiguous"
        },
        m.reporter_count
    );

    // ── SOVEREIGNTY: Does this number preserve caller autonomy? ──
    // High sovereignty = legitimate number that doesn't coerce recipients.
    // Low sovereignty = spam/scam that undermines recipient's agency.
    // Falsify: robocall service for emergency alerts (coercive form, legitimate purpose).
    let mut sovereignty: f64 = SOVEREIGNTY_BASE;
    // Spam score directly maps to coercion: high spam = low sovereignty
    sovereignty += (0.5 - m.spam_score) * 0.5; // [-0.25, +0.25] range from spam score
    if m.owner_verified && m.contestation_count > 0 {
        sovereignty = 0.90; // verified owner actively contesting = strong autonomy signal
    }
    // Scam labels are strongest sovereignty violation (not just nuisance)
    if total_labels > 0 {
        let scam_ratio = m.label_scam as f64 / total_labels as f64;
        if scam_ratio > 0.5 {
            sovereignty -= ADJUST_MEDIUM; // majority scam labels = severe sovereignty breach
        }
    }
    let sovereignty = sovereignty.clamp(ADJUST_SMALL, PHI_INV);
    let sovereignty_reason = format!(
        "spam_score={:.3}, scam_labels={}/{}, owner_verified={}, contestation_count={}.",
        m.spam_score, m.label_scam, total_labels, m.owner_verified, m.contestation_count,
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
        abstentions: vec![], // All 6 axioms judged from community report data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::phone_number::{LabelDistribution, PhoneData};
    use crate::domain::stimulus::build_phone_stimulus;

    /// Build a test PhoneData with confirmed-spam profile.
    fn confirmed_spam_data() -> PhoneData {
        PhoneData {
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
            mean_reporter_trust: 0.75,
            age_days: 14,
            days_since_last_report: 1,
            challenge_pass_rate: Some(0.15),
            contestation_count: 0,
            owner_verified: false,
        }
    }

    /// Build a test PhoneData with unknown number profile (few reports).
    fn unknown_number_data() -> PhoneData {
        PhoneData {
            number: "+14155552671".to_string(),
            country_code: "US".to_string(),
            total_events: 2,
            label_distribution: LabelDistribution {
                legitimate: 1,
                nuisance: 0,
                scam: 0,
                unknown: 1,
            },
            reporter_count: 2,
            mean_reporter_trust: 0.50,
            age_days: 3,
            days_since_last_report: 2,
            challenge_pass_rate: None,
            contestation_count: 0,
            owner_verified: false,
        }
    }

    #[test]
    fn parse_phone_metrics_from_stimulus() {
        let data = confirmed_spam_data();
        let stimulus = build_phone_stimulus(&data);
        let m = parse(&stimulus).expect("should parse phone stimulus");

        assert_eq!(m.total_events, 47);
        assert_eq!(m.reporter_count, 35);
        assert!((m.mean_reporter_trust - 0.75).abs() < 0.001);
        assert_eq!(m.age_days, 14);
        assert_eq!(m.label_legitimate, 3);
        assert_eq!(m.label_nuisance, 30);
        assert_eq!(m.label_scam, 12);
        assert_eq!(m.label_unknown, 2);
        assert_eq!(m.contestation_count, 0);
        assert!(!m.owner_verified);
        // challenge_pass_rate: Some(0.15) → "15.0%" in stimulus → 0.15 parsed back
        assert!(
            m.challenge_pass_rate
                .is_some_and(|r| (r - 0.15).abs() < 0.001),
            "challenge_pass_rate should be ~0.15, got {:?}",
            m.challenge_pass_rate
        );
    }

    #[test]
    fn parse_returns_none_for_non_phone_content() {
        assert!(parse("Just a normal sentence.").is_none());
        assert!(parse("[DOMAIN: token-analysis]\n[METRICS]\n").is_none());
        assert!(parse("[DOMAIN: chess]\n[METRICS]\n").is_none());
    }

    #[test]
    fn parse_phone_unknown_number() {
        let data = unknown_number_data();
        let stimulus = build_phone_stimulus(&data);
        let m = parse(&stimulus).expect("should parse unknown number stimulus");

        assert_eq!(m.total_events, 2);
        assert_eq!(m.reporter_count, 2);
        assert_eq!(m.label_legitimate, 1);
        assert_eq!(m.label_unknown, 1);
        assert!(m.challenge_pass_rate.is_none());
    }

    #[tokio::test]
    async fn score_confirmed_spam_high_fidelity() {
        let data = confirmed_spam_data();
        let stimulus = build_phone_stimulus(&data);
        let m = parse(&stimulus).unwrap();
        let scores = score(&m);

        // Confirmed spam with 35 high-trust reporters: FIDELITY should be high
        assert!(
            scores.fidelity > 0.60,
            "confirmed spam with high-trust reporters should have high fidelity, got {}",
            scores.fidelity
        );
        // BURN should be high: spam_score ~0.77 is a clear signal
        assert!(
            scores.burn > 0.60,
            "clear spam signal should have high burn, got {}",
            scores.burn
        );
        // SOVEREIGNTY should be low: high spam_score = coercion
        assert!(
            scores.sovereignty < 0.50,
            "spam number should have low sovereignty, got {}",
            scores.sovereignty
        );
        // No abstentions
        assert!(
            scores.abstentions.is_empty(),
            "phone-number domain should have zero abstentions, got {:?}",
            scores.abstentions
        );
    }

    #[tokio::test]
    async fn score_unknown_number_low_fidelity_low_burn() {
        let data = unknown_number_data();
        let stimulus = build_phone_stimulus(&data);
        let m = parse(&stimulus).unwrap();
        let scores = score(&m);

        // Unknown number with only 2 reporters: FIDELITY should be low
        assert!(
            scores.fidelity < 0.40,
            "unknown number with few reporters should have low fidelity, got {}",
            scores.fidelity
        );
        // Low reporter count → low BURN (noisy signal)
        assert!(
            scores.burn < 0.50,
            "unknown number should have low burn (noisy signal), got {}",
            scores.burn
        );
        // No abstentions
        assert!(
            scores.abstentions.is_empty(),
            "phone-number domain should have zero abstentions, got {:?}",
            scores.abstentions
        );
    }

    #[tokio::test]
    async fn score_differs_between_spam_and_unknown() {
        let spam_data = confirmed_spam_data();
        let spam_stim = build_phone_stimulus(&spam_data);
        let m_spam = parse(&spam_stim).unwrap();
        let scores_spam = score(&m_spam);

        let unknown_data = unknown_number_data();
        let unknown_stim = build_phone_stimulus(&unknown_data);
        let m_unknown = parse(&unknown_stim).unwrap();
        let scores_unknown = score(&m_unknown);

        // Spam should score higher fidelity (more reporters) but lower sovereignty
        assert!(
            scores_spam.fidelity > scores_unknown.fidelity,
            "spam (more reporters) should have higher fidelity: spam={}, unknown={}",
            scores_spam.fidelity,
            scores_unknown.fidelity
        );
        assert!(
            scores_spam.sovereignty < scores_unknown.sovereignty,
            "spam should have lower sovereignty: spam={}, unknown={}",
            scores_spam.sovereignty,
            scores_unknown.sovereignty
        );
    }

    #[tokio::test]
    async fn score_verified_owner_with_contestation_boosts_sovereignty() {
        let data = PhoneData {
            number: "+14155550001".to_string(),
            country_code: "US".to_string(),
            total_events: 10,
            label_distribution: LabelDistribution {
                legitimate: 8,
                nuisance: 1,
                scam: 0,
                unknown: 1,
            },
            reporter_count: 10,
            mean_reporter_trust: 0.80,
            age_days: 60,
            days_since_last_report: 5,
            challenge_pass_rate: Some(0.95),
            contestation_count: 3,
            owner_verified: true,
        };
        let stimulus = build_phone_stimulus(&data);
        let m = parse(&stimulus).unwrap();
        let scores = score(&m);

        // Owner verified + contestation = high sovereignty (capped at PHI_INV = 0.618)
        assert!(
            scores.sovereignty > 0.55,
            "verified owner with contestation should have high sovereignty, got {}",
            scores.sovereignty
        );
        // Verify should be boosted
        assert!(
            scores.verify > 0.60,
            "owner_verified + challenge data should boost verify, got {}",
            scores.verify
        );
    }
}
