//! Crystallization engine — state machine, decay, context formatting.
//!
//! Pure domain logic: compute certainty, classify states, format context for injection.

use serde::{Deserialize, Serialize};

use crate::domain::dog::{PHI_INV, PHI_INV2, PHI_INV3};

#[cfg(test)]
use super::crystal::Crystal;
use super::crystal::{CANONICAL_CYCLES, CrystalState, MIN_CRYSTALLIZATION_CYCLES, MatureCrystal};

// ── CRYSTALLIZATION ENGINE (pure domain logic) ──────────────

/// Compute certainty from Welford stats + observation count.
/// Pure domain logic — used by storage adapters and tests.
///
/// Formula: `concordance * volume` where:
/// - `concordance = 1 / (1 + (stddev / phi-cubed-inverse)^2)` — quadratic decay, phi-calibrated
/// - `volume = min(obs / 21, 1.0)` — linear ramp to crystallization threshold
///
/// Crystallization (certainty >= phi-inverse) requires stddev <= 0.186 at 21+ obs.
/// Decay (certainty < phi-inverse-squared) requires stddev > 0.300 — maximally disagreeing.
/// All thresholds self-derive from phi constants.
pub fn compute_certainty(variance_m2: f64, observations: u32) -> f64 {
    let stddev = if observations > 1 {
        (variance_m2 / (observations as f64 - 1.0)).sqrt()
    } else {
        0.0
    };
    let ratio = stddev / PHI_INV3;
    let concordance = 1.0 / (1.0 + ratio * ratio);
    let volume = (observations as f64 / MIN_CRYSTALLIZATION_CYCLES as f64).min(1.0);
    concordance * volume
}

/// Classify a crystal based on its certainty and observation count.
/// Used by InMemoryStorage and tests. SurrealDB does this in SQL.
pub fn classify(certainty: f64, observations: u32) -> CrystalState {
    if observations >= CANONICAL_CYCLES && certainty >= PHI_INV {
        CrystalState::Canonical
    } else if observations >= MIN_CRYSTALLIZATION_CYCLES && certainty >= PHI_INV {
        CrystalState::Crystallized
    } else if observations >= MIN_CRYSTALLIZATION_CYCLES && certainty < PHI_INV2 {
        CrystalState::Decaying
    } else {
        CrystalState::Forming
    }
}

/// Observe a new Q-Score for a pattern. Returns the updated crystal state.
/// Pure domain logic — used in tests. Production uses atomic SQL.
#[cfg(test)]
pub fn observe(crystal: &Crystal, new_score: f64) -> CrystalState {
    let next_obs = crystal.observations + 1;
    let old_mean = crystal.confidence;
    let new_mean = running_mean(old_mean, crystal.observations, new_score);
    let delta = new_score - old_mean;
    let delta2 = new_score - new_mean;
    let new_m2 = crystal.variance_m2 + delta * delta2;
    let certainty = compute_certainty(new_m2, next_obs);
    classify(certainty, next_obs)
}

/// Update a crystal with a new observation. Returns the new crystal.
/// Pure function — used in tests to verify crystallization logic.
/// Production path uses atomic SQL UPDATE (no Rust-side state computation).
#[cfg(test)]
pub fn update_crystal(crystal: &Crystal, new_score: f64, timestamp: &str) -> Crystal {
    let observations = crystal.observations + 1;
    let confidence = running_mean(crystal.confidence, crystal.observations, new_score);
    let delta = new_score - crystal.confidence;
    let delta2 = new_score - confidence;
    let variance_m2 = crystal.variance_m2 + delta * delta2;
    let certainty = compute_certainty(variance_m2, observations);
    let state = classify(certainty, observations);

    Crystal {
        id: crystal.id.clone(),
        content: crystal.content.clone(),
        domain: crystal.domain.clone(),
        confidence,
        observations,
        state,
        created_at: crystal.created_at.clone(),
        updated_at: timestamp.to_string(),
        contributing_verdicts: crystal.contributing_verdicts.clone(),
        certainty,
        variance_m2,
        mean_quorum: crystal.mean_quorum,
        howl_count: crystal.howl_count,
        wag_count: crystal.wag_count,
        growl_count: crystal.growl_count,
        bark_count: crystal.bark_count,
    }
}

/// Create a new crystal from a first observation.
/// Test helper — production path creates crystals via atomic SQL UPSERT.
#[cfg(test)]
pub fn new_crystal(
    id: String,
    content: String,
    domain: String,
    initial_score: f64,
    timestamp: &str,
) -> Crystal {
    Crystal {
        id,
        content,
        domain,
        confidence: initial_score,
        observations: 1,
        state: CrystalState::Forming,
        created_at: timestamp.to_string(),
        updated_at: timestamp.to_string(),
        contributing_verdicts: vec![],
        certainty: 0.0,
        variance_m2: 0.0,
        mean_quorum: 0.0,
        howl_count: 0,
        wag_count: 0,
        growl_count: 0,
        bark_count: 0,
    }
}

/// Incremental running mean: avoids storing all historical values.
#[cfg(test)]
fn running_mean(current_mean: f64, count: u32, new_value: f64) -> f64 {
    if count == 0 {
        return new_value;
    }
    let n = count as f64;
    (current_mean * n + new_value) / (n + 1.0)
}

// ── DECAY RELEVANCE ───────────────────────────────────────
/// Decay constant: 90 days. At 90 days, factor = e^(-1) ~ 0.368.
const DECAY_DAYS: f64 = 90.0;

/// Compute decay relevance: `confidence * e^(-age_days / DECAY_DAYS)`.
/// Pure function — caller provides `now` for testability.
/// Returns 0.0 on unparseable timestamps (defensive, not silent).
pub fn decay_relevance(confidence: f64, updated_at: &str, now: &str) -> f64 {
    let Ok(updated) = chrono::DateTime::parse_from_rfc3339(updated_at) else {
        return 0.0;
    };
    let Ok(now_dt) = chrono::DateTime::parse_from_rfc3339(now) else {
        return 0.0;
    };
    let age_days = (now_dt - updated).num_seconds().max(0) as f64 / 86400.0;
    confidence * (-age_days / DECAY_DAYS).exp()
}

// ── CCM FEEDBACK — inject crystallized wisdom into stimulus context ──

/// Format mature crystals as context for Dog prompts.
/// Accepts `&[MatureCrystal]` — type system guarantees only Crystallized/Canonical.
/// Filters by exact domain match only. No "general" cross-domain fallback —
/// it was a poison vector (hermes heartbeats, curl commands contaminating all domains).
/// Cross-domain discovery is handled by embedding similarity (pipeline primary path).
/// Token-budget-aware: caps at max_chars to avoid overflowing small models.
/// Content wrapped in delimiters (T7 defense-in-depth).
pub fn format_crystal_context(
    crystals: &[MatureCrystal],
    domain: &str,
    max_chars: usize,
) -> Option<String> {
    let domain_filtered: Vec<&MatureCrystal> =
        crystals.iter().filter(|c| c.domain() == domain).collect();

    if domain_filtered.is_empty() {
        return None;
    }

    let now = chrono::Utc::now().to_rfc3339();
    let mut sorted = domain_filtered;
    sorted.sort_by(|a, b| {
        let ra = decay_relevance(a.confidence(), a.updated_at(), &now);
        let rb = decay_relevance(b.confidence(), b.updated_at(), &now);
        rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut lines = Vec::new();
    let mut total_chars = 0;
    let header = format!(
        "[CYNIC Memory — {} crystallized patterns for domain '{}']",
        sorted.len(),
        domain
    );
    total_chars += header.len();
    lines.push(header);

    for c in sorted {
        let state_label = if *c.state() == CrystalState::Canonical {
            "CANONICAL"
        } else {
            "CRYSTALLIZED"
        };
        let polarity = c.dominant_polarity();
        let delimited = crate::domain::sanitize::delimit_crystal_content(c.content());
        let line = format!(
            "- [{} {}] (certainty: {:.2}, quality: {:.2}, {} obs): {}",
            state_label,
            polarity,
            c.certainty(),
            c.confidence(),
            c.observations(),
            delimited
        );
        if total_chars + line.len() > max_chars {
            break;
        }
        total_chars += line.len();
        lines.push(line);
    }

    if lines.len() <= 1 {
        return None;
    }

    Some(lines.join("\n"))
}

// ── SESSION SUMMARIES ──────────────────────────────────────
/// A compressed narrative of what happened in a development session.
/// Created by sovereign inference at session end (coord_release) or
/// by the background summarization task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub agent_id: String,
    pub summary: String,
    pub observations_count: u32,
    pub created_at: String,
}

/// Format recent session summaries as context for Dog prompts.
/// Separate from crystal context — own token budget.
pub fn format_session_context(summaries: &[SessionSummary], max_chars: usize) -> Option<String> {
    if summaries.is_empty() {
        return None;
    }

    let mut lines = Vec::new();
    let header = format!(
        "[CYNIC Session Memory — {} recent sessions]",
        summaries.len()
    );
    let mut total_chars = header.len();
    lines.push(header);

    for s in summaries {
        let line = format!(
            "- [{}] ({}obs): {}",
            s.session_id.chars().take(12).collect::<String>(),
            s.observations_count,
            s.summary
        );
        if total_chars + line.len() > max_chars {
            break;
        }
        total_chars += line.len();
        lines.push(line);
    }

    if lines.len() <= 1 {
        return None;
    }

    Some(lines.join("\n"))
}

/// Format raw observations into a summarization prompt for the sovereign LLM.
/// Pure function — no I/O.
pub fn format_summarization_prompt(
    observations: &[crate::domain::storage::RawObservation],
) -> String {
    let mut items: Vec<String> = Vec::new();
    for obs in observations.iter().take(30) {
        items.push(format!(
            "- {} {} (domain: {}, status: {})",
            obs.tool, obs.target, obs.domain, obs.status
        ));
    }
    format!(
        "Summarize this development session in 2-3 sentences. Focus on WHAT was done and WHY. Be specific about files and outcomes.\n\nObservations:\n{}",
        items.join("\n")
    )
}

// ── TESTS ───────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::super::crystal::filter_mature;
    use super::*;

    fn make_crystal(confidence: f64, observations: u32, state: CrystalState) -> Crystal {
        Crystal {
            id: "test-1".into(),
            content: "test pattern".into(),
            domain: "test".into(),
            confidence,
            observations,
            state,
            created_at: "2026-03-13T00:00:00Z".into(),
            updated_at: "2026-03-13T00:00:00Z".into(),
            contributing_verdicts: vec![],
            certainty: 0.0,
            variance_m2: 0.0,
            mean_quorum: 0.0,
            howl_count: 0,
            wag_count: 0,
            growl_count: 0,
            bark_count: 0,
        }
    }

    #[test]
    fn new_crystal_starts_forming() {
        let c = new_crystal("x".into(), "test".into(), "chess".into(), 0.5, "now");
        assert_eq!(c.state, CrystalState::Forming);
        assert_eq!(c.observations, 1);
    }

    #[test]
    fn crystallizes_after_21_cycles_above_phi() {
        let c = make_crystal(
            PHI_INV,
            MIN_CRYSTALLIZATION_CYCLES - 1,
            CrystalState::Forming,
        );
        let state = observe(&c, PHI_INV);
        assert_eq!(state, CrystalState::Crystallized);
    }

    #[test]
    fn concordant_low_quality_crystallizes_under_4d() {
        let c = make_crystal(0.5, MIN_CRYSTALLIZATION_CYCLES - 1, CrystalState::Forming);
        let state = observe(&c, 0.5);
        assert_eq!(state, CrystalState::Crystallized);
    }

    #[test]
    fn decays_when_certainty_drops_from_high_variance() {
        let c = make_crystal(PHI_INV, 25, CrystalState::Crystallized);
        let mut crystal = c;
        for i in 0..100 {
            let score = if i % 2 == 0 { 1.0 } else { 0.0 };
            crystal = update_crystal(&crystal, score, "now");
        }
        assert!(
            crystal.certainty < PHI_INV2,
            "certainty should be low: {}",
            crystal.certainty
        );
        assert_eq!(crystal.state, CrystalState::Decaying);
    }

    #[test]
    fn canonical_after_233_cycles() {
        let c = make_crystal(PHI_INV, CANONICAL_CYCLES - 1, CrystalState::Crystallized);
        let state = observe(&c, PHI_INV);
        assert_eq!(state, CrystalState::Canonical);
    }

    #[test]
    fn running_mean_correct() {
        let mean = running_mean(0.5, 1, 0.7);
        assert!((mean - 0.6).abs() < 1e-10);
    }

    #[test]
    fn running_mean_first_value() {
        let mean = running_mean(0.0, 0, 0.42);
        assert!((mean - 0.42).abs() < 1e-10);
    }

    #[test]
    fn high_variance_prevents_false_crystallization() {
        let mut c = new_crystal(
            "osc".into(),
            "oscillating".into(),
            "test".into(),
            0.9,
            "now",
        );
        for i in 0..30 {
            let score = if i % 2 == 0 { 0.9 } else { 0.2 };
            c = update_crystal(&c, score, "now");
        }
        assert!(
            c.certainty < PHI_INV,
            "certainty should be low: {}",
            c.certainty
        );
        assert_ne!(c.state, CrystalState::Crystallized);
    }

    #[test]
    fn crystal_context_includes_mature_only() {
        let crystals = vec![
            make_crystal(PHI_INV, 25, CrystalState::Crystallized),
            make_crystal(0.3, 5, CrystalState::Forming),
            make_crystal(PHI_INV, 250, CrystalState::Canonical),
        ];
        let mature = filter_mature(crystals);
        assert_eq!(mature.len(), 2);
        let ctx = format_crystal_context(&mature, "test", 2000).unwrap();
        assert!(ctx.contains("CRYSTALLIZED"));
        assert!(ctx.contains("CANONICAL"));
    }

    #[test]
    fn crystal_context_respects_budget() {
        let crystals: Vec<Crystal> = (0..100)
            .map(|i| {
                let mut c = make_crystal(PHI_INV, 25, CrystalState::Crystallized);
                c.content = format!("pattern number {i} with some extra text to fill space");
                c
            })
            .collect();
        let mature = filter_mature(crystals);
        let ctx = format_crystal_context(&mature, "test", 400).unwrap();
        assert!(ctx.len() <= 600);
    }

    #[test]
    fn crystal_context_empty_when_no_mature() {
        let crystals = vec![make_crystal(0.3, 5, CrystalState::Forming)];
        let mature = filter_mature(crystals);
        assert!(mature.is_empty());
        assert!(format_crystal_context(&mature, "test", 2000).is_none());
    }

    #[test]
    fn decay_relevance_no_decay_for_today() {
        let now = "2026-03-21T12:00:00Z";
        let updated = "2026-03-21T12:00:00Z";
        let rel = decay_relevance(0.7, updated, now);
        assert!(
            (rel - 0.7).abs() < 0.01,
            "same-day crystal should have ~no decay, got {rel}"
        );
    }

    #[test]
    fn decay_relevance_decays_old_crystals() {
        let now = "2026-03-21T12:00:00Z";
        let updated_90d_ago = "2025-12-21T12:00:00Z";
        let rel = decay_relevance(0.7, updated_90d_ago, now);
        assert!(
            rel < 0.3,
            "90-day-old crystal should decay significantly, got {rel}"
        );
        assert!(
            rel > 0.2,
            "decay shouldn't be too extreme at 90 days, got {rel}"
        );
    }

    #[test]
    fn decay_relevance_recent_beats_old_high_confidence() {
        let now = "2026-03-21T12:00:00Z";
        let recent = decay_relevance(0.65, "2026-03-20T12:00:00Z", now);
        let old = decay_relevance(0.70, "2025-09-21T12:00:00Z", now);
        assert!(
            recent > old,
            "recent crystal ({recent}) should outrank old one ({old})"
        );
    }

    #[test]
    fn decay_relevance_bad_timestamp_returns_zero() {
        let rel = decay_relevance(0.7, "not-a-date", "2026-03-21T12:00:00Z");
        assert!(
            (rel - 0.0).abs() < 1e-10,
            "bad timestamp should return 0, got {rel}"
        );
    }

    #[test]
    fn crystal_context_prefers_recent_over_old() {
        let mut recent = make_crystal(0.65, 25, CrystalState::Crystallized);
        recent.updated_at = "2026-03-20T12:00:00Z".into();
        recent.content = "RECENT_PATTERN".into();

        let mut old = make_crystal(0.70, 30, CrystalState::Crystallized);
        old.updated_at = "2025-06-01T00:00:00Z".into();
        old.content = "OLD_PATTERN".into();
        old.id = "test-2".into();

        let mature = filter_mature(vec![old, recent]);
        let ctx = format_crystal_context(&mature, "test", 2000).unwrap();
        let recent_pos = ctx
            .find("RECENT_PATTERN")
            .expect("recent should be in output");
        let old_pos = ctx.find("OLD_PATTERN").expect("old should be in output");
        assert!(
            recent_pos < old_pos,
            "recent crystal should rank before old crystal"
        );
    }

    fn make_session_summary(session_id: &str, obs_count: u32, summary: &str) -> SessionSummary {
        SessionSummary {
            session_id: session_id.into(),
            agent_id: "test-agent".into(),
            summary: summary.into(),
            observations_count: obs_count,
            created_at: "2026-03-21T12:00:00Z".into(),
        }
    }

    #[test]
    fn session_context_formats_summaries() {
        let summaries = vec![
            make_session_summary("sess-abc123", 15, "Fixed temporal decay in CCM crystals"),
            make_session_summary("sess-def456", 8, "Added vector search to SurrealDB"),
        ];
        let ctx = format_session_context(&summaries, 2000).unwrap();
        assert!(ctx.contains("Session Memory"));
        assert!(ctx.contains("Fixed temporal decay"));
        assert!(ctx.contains("vector search"));
    }

    #[test]
    fn session_context_empty_when_no_summaries() {
        assert!(format_session_context(&[], 2000).is_none());
    }

    #[test]
    fn session_context_respects_budget() {
        let summaries: Vec<SessionSummary> = (0..50)
            .map(|i| {
                make_session_summary(
                    &format!("sess-{i:03}"),
                    10,
                    &format!("Did something important in session number {i}"),
                )
            })
            .collect();
        let ctx = format_session_context(&summaries, 200).unwrap();
        assert!(ctx.len() <= 300);
    }

    #[test]
    fn summarization_prompt_formats_observations() {
        use crate::domain::storage::RawObservation;
        let obs = vec![
            RawObservation {
                id: String::new(),
                tool: "Edit".into(),
                target: "ccm.rs".into(),
                domain: "code".into(),
                status: "ok".into(),
                context: String::new(),
                created_at: String::new(),
                project: String::new(),
                agent_id: String::new(),
                session_id: String::new(),
                tags: vec![],
            },
            RawObservation {
                id: String::new(),
                tool: "Bash".into(),
                target: "cargo test".into(),
                domain: "workflow".into(),
                status: "ok".into(),
                context: String::new(),
                created_at: String::new(),
                project: String::new(),
                agent_id: String::new(),
                session_id: String::new(),
                tags: vec![],
            },
        ];
        let prompt = format_summarization_prompt(&obs);
        assert!(prompt.contains("Summarize this development session"));
        assert!(prompt.contains("Edit ccm.rs"));
        assert!(prompt.contains("Bash cargo test"));
    }

    #[test]
    fn summarization_prompt_caps_at_30_observations() {
        use crate::domain::storage::RawObservation;
        let obs: Vec<RawObservation> = (0..50)
            .map(|i| RawObservation {
                id: String::new(),
                tool: "Edit".into(),
                target: format!("file{i}.rs"),
                domain: "code".into(),
                status: "ok".into(),
                context: String::new(),
                created_at: String::new(),
                project: String::new(),
                agent_id: String::new(),
                session_id: String::new(),
                tags: vec![],
            })
            .collect();
        let prompt = format_summarization_prompt(&obs);
        assert!(prompt.contains("file29.rs"));
        assert!(!prompt.contains("file30.rs"));
    }

    #[test]
    fn normalized_qscore_enables_crystallization() {
        let typical_qscore = 0.55;
        let normalized = (typical_qscore / PHI_INV).min(1.0);

        let c = make_crystal(
            normalized,
            MIN_CRYSTALLIZATION_CYCLES - 1,
            CrystalState::Forming,
        );
        let state = observe(&c, normalized);
        assert_eq!(
            state,
            CrystalState::Crystallized,
            "Normalized Q-Score {normalized:.3} (raw {typical_qscore:.3}) should crystallize after 21 cycles"
        );
    }

    #[test]
    fn negative_truth_crystallizes_under_4d() {
        let bad_qscore = 0.08;
        let normalized = (bad_qscore / PHI_INV).min(1.0);

        let c = make_crystal(
            normalized,
            MIN_CRYSTALLIZATION_CYCLES - 1,
            CrystalState::Forming,
        );
        let state = observe(&c, normalized);
        assert_eq!(
            state,
            CrystalState::Crystallized,
            "4D: concordant negative truth (normalized {normalized:.3}) MUST crystallize"
        );
    }

    #[test]
    fn raw_qscore_crystallizes_under_4d() {
        let good_qscore = 0.57;

        let c = make_crystal(
            good_qscore,
            MIN_CRYSTALLIZATION_CYCLES - 1,
            CrystalState::Forming,
        );
        let state = observe(&c, good_qscore);
        assert_eq!(
            state,
            CrystalState::Crystallized,
            "4D: concordant raw Q-Score {good_qscore:.3} MUST crystallize"
        );
    }
}
