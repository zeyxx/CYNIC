//! CCM — Cognitive Crystallization Mechanism.
//! Ephemeral verdicts → persistent wisdom. Domain-pure logic.
//!
//! A pattern that scores >= φ⁻¹ (0.618) repeatedly across 21+ cycles
//! crystallizes into persistent wisdom. Below φ⁻² (0.382) it decays.
//! This is how CYNIC learns without training — through phi-bounded consensus.

use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::domain::dog::{PHI_INV, PHI_INV2};

/// Fibonacci F(8) = 21 — minimum observations before crystallization.
/// Production thresholds are in atomic SQL (storage/surreal.rs::observe_crystal).
#[cfg(test)]
pub const MIN_CRYSTALLIZATION_CYCLES: u32 = 21;
/// Fibonacci F(13) = 233 — canonical status (deeply crystallized).
#[cfg(test)]
pub const CANONICAL_CYCLES: u32 = 233;

// ── CRYSTAL ─────────────────────────────────────────────────
/// A crystallized truth — persistent wisdom extracted from ephemeral verdicts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crystal {
    /// Unique identifier
    pub id: String,
    /// The crystallized insight (domain-agnostic)
    pub content: String,
    /// Domain this was crystallized from (e.g. "chess", "code", "general")
    pub domain: String,
    /// Running mean of Q-Score totals that contributed
    pub confidence: f64,
    /// Number of concordant observations
    pub observations: u32,
    /// Current state in the crystallization lifecycle
    pub state: CrystalState,
    /// Timestamp of creation
    pub created_at: String,
    /// Timestamp of last update
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CrystalState {
    /// Accumulating observations, not yet crystallized (< 21 cycles)
    Forming,
    /// Reached crystallization threshold (>= 21 cycles, confidence >= φ⁻¹)
    Crystallized,
    /// Reached canonical status (>= 233 cycles, confidence >= φ⁻¹)
    Canonical,
    /// Confidence dropped below φ⁻² — decaying
    Decaying,
    /// Fully dissolved — no longer valid wisdom
    Dissolved,
}

// ── CRYSTALLIZATION ENGINE (pure domain logic) ──────────────

/// Observe a new Q-Score for a pattern. Returns the updated crystal state.
/// Pure domain logic — used in tests. Production uses atomic SQL.
#[cfg(test)]
pub fn observe(crystal: &Crystal, new_score: f64) -> CrystalState {
    let next_obs = crystal.observations + 1;
    let next_confidence = running_mean(crystal.confidence, crystal.observations, new_score);

    classify(next_confidence, next_obs)
}

/// Floating-point tolerance for phi-threshold comparisons.
#[cfg(test)]
const EPSILON: f64 = 1e-10;

/// Classify a crystal based on its confidence and observation count.
/// Test-only — production state classification is in atomic SQL (observe_crystal).
#[cfg(test)]
fn classify(confidence: f64, observations: u32) -> CrystalState {
    if confidence < PHI_INV2 - EPSILON {
        if observations > MIN_CRYSTALLIZATION_CYCLES {
            CrystalState::Decaying
        } else {
            // Not enough data to dissolve — keep forming until MIN_CRYSTALLIZATION_CYCLES
            CrystalState::Forming
        }
    } else if observations >= CANONICAL_CYCLES && confidence >= PHI_INV - EPSILON {
        CrystalState::Canonical
    } else if observations >= MIN_CRYSTALLIZATION_CYCLES && confidence >= PHI_INV - EPSILON {
        CrystalState::Crystallized
    } else {
        CrystalState::Forming
    }
}

/// Update a crystal with a new observation. Returns the new crystal.
/// Pure function — used in tests to verify crystallization logic.
/// Production path uses atomic SQL UPDATE (no Rust-side state computation).
#[cfg(test)]
pub fn update_crystal(crystal: &Crystal, new_score: f64, timestamp: &str) -> Crystal {
    let observations = crystal.observations + 1;
    let confidence = running_mean(crystal.confidence, crystal.observations, new_score);
    let state = classify(confidence, observations);

    Crystal {
        id: crystal.id.clone(),
        content: crystal.content.clone(),
        domain: crystal.domain.clone(),
        confidence,
        observations,
        state,
        created_at: crystal.created_at.clone(),
        updated_at: timestamp.to_string(),
    }
}

/// Create a new crystal from a first observation.
/// Test helper — production path creates crystals via atomic SQL UPSERT.
#[cfg(test)]
pub fn new_crystal(id: String, content: String, domain: String, initial_score: f64, timestamp: &str) -> Crystal {
    Crystal {
        id,
        content,
        domain,
        confidence: initial_score,
        observations: 1,
        state: CrystalState::Forming,
        created_at: timestamp.to_string(),
        updated_at: timestamp.to_string(),
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

// ── TEMPORAL DECAY ─────────────────────────────────────────
/// Decay constant: 90 days. At 90 days, factor = e⁻¹ ≈ 0.368.
const DECAY_DAYS: f64 = 90.0;

/// Compute temporal relevance: `confidence × e^(-age_days / DECAY_DAYS)`.
/// Pure function — caller provides `now` for testability.
/// Returns 0.0 on unparseable timestamps (defensive, not silent).
pub fn temporal_relevance(confidence: f64, updated_at: &str, now: &str) -> f64 {
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
/// Only includes Crystallized and Canonical crystals from the same domain.
/// Token-budget-aware: caps at max_chars to avoid overflowing small models.
pub fn format_crystal_context(crystals: &[Crystal], domain: &str, max_chars: usize) -> Option<String> {
    let mature: Vec<&Crystal> = crystals.iter()
        .filter(|c| c.domain == domain || c.domain == "general")
        .filter(|c| matches!(c.state, CrystalState::Crystallized | CrystalState::Canonical))
        .collect();

    if mature.is_empty() {
        return None;
    }

    // Sort by temporal relevance descending — recent high-confidence crystals first
    let now = chrono::Utc::now().to_rfc3339();
    let mut sorted = mature;
    sorted.sort_by(|a, b| {
        let ra = temporal_relevance(a.confidence, &a.updated_at, &now);
        let rb = temporal_relevance(b.confidence, &b.updated_at, &now);
        rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut lines = Vec::new();
    let mut total_chars = 0;
    let header = format!("[CYNIC Memory — {} crystallized patterns for domain '{}']", sorted.len(), domain);
    total_chars += header.len();
    lines.push(header);

    for c in sorted {
        let state_label = if c.state == CrystalState::Canonical { "CANONICAL" } else { "CRYSTALLIZED" };
        let line = format!("- [{}] (confidence: {:.2}, {} observations): {}", state_label, c.confidence, c.observations, c.content);
        if total_chars + line.len() > max_chars {
            break; // Token budget exhausted
        }
        total_chars += line.len();
        lines.push(line);
    }

    if lines.len() <= 1 {
        return None; // Only header, no crystals fit
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
    let header = format!("[CYNIC Session Memory — {} recent sessions]", summaries.len());
    let mut total_chars = header.len();
    lines.push(header);

    for s in summaries {
        let line = format!("- [{}] ({}obs): {}", s.session_id.chars().take(12).collect::<String>(), s.observations_count, s.summary);
        if total_chars + line.len() > max_chars {
            break;
        }
        total_chars += line.len();
        lines.push(line);
    }

    if lines.len() <= 1 {
        return None; // Only header, no summaries fit
    }

    Some(lines.join("\n"))
}

/// Format raw observations into a summarization prompt for the sovereign LLM.
/// Pure function — no I/O.
pub fn format_summarization_prompt(observations: &[serde_json::Value]) -> String {
    let mut items: Vec<String> = Vec::new();
    for obs in observations.iter().take(30) {
        let tool = obs["tool"].as_str().unwrap_or("?");
        let target = obs["target"].as_str().unwrap_or("?");
        let domain = obs["domain"].as_str().unwrap_or("?");
        let status = obs["status"].as_str().unwrap_or("?");
        items.push(format!("- {} {} (domain: {}, status: {})", tool, target, domain, status));
    }
    format!(
        "Summarize this development session in 2-3 sentences. Focus on WHAT was done and WHY. Be specific about files and outcomes.\n\nObservations:\n{}",
        items.join("\n")
    )
}

// ── CONTENT HASHING ─────────────────────────────────────────
/// Deterministic content hash for crystal IDs. FNV-1a — not cryptographic,
/// just stable content addressing for deduplication. Changing this algo
/// would orphan all existing crystals in the DB.
pub fn content_hash(input: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for byte in input.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3); // FNV-1a prime
    }
    h
}

// ── WORKFLOW AGGREGATOR ────────────────────────────────────
/// Aggregate raw observations into CCM crystals. Runs periodically.
/// Extracts frequency patterns and co-occurrences from the observation table,
/// then feeds them into observe_crystal as workflow domain crystals.
///
/// Pure logic: takes query results, returns (id, content, domain, score) tuples.
/// Caller is responsible for DB queries and observe_crystal calls.
pub fn extract_patterns(rows: &[serde_json::Value], total_observations: u64) -> Vec<(String, String, f64)> {
    if total_observations == 0 {
        return Vec::new();
    }

    let mut patterns: Vec<(String, String, f64)> = Vec::new();

    for row in rows {
        let target = row["target"].as_str().unwrap_or("").to_string();
        let tool = row["tool"].as_str().unwrap_or("").to_string();
        let freq = row["freq"].as_u64().unwrap_or(0);

        if target.is_empty() || freq < 3 {
            continue; // Skip noise — at least 3 occurrences to matter
        }

        let score = freq as f64 / total_observations as f64;
        // Crystal ID: deterministic hash of the pattern
        let id = format!("wf_{:x}", content_hash(&format!("{}:{}", tool, target)));
        // Human-readable content for injection into prompts
        let content = format!("{} {} — {}x observed", tool, target, freq);

        patterns.push((id, content, score));
    }

    patterns
}

/// Extract co-occurrence patterns from session-grouped observations.
/// Input: rows of {session_id, target} sorted by session_id.
/// Output: (crystal_id, content, score) for pairs that co-occur in 2+ sessions.
///
/// Pure function — all co-occurrence computation happens in Rust, not SQL.
pub fn extract_cooccurrences(rows: &[serde_json::Value]) -> Vec<(String, String, f64)> {
    use std::collections::{HashMap, HashSet};

    // Group targets by session
    let mut sessions: HashMap<String, HashSet<String>> = HashMap::new();
    for row in rows {
        let session = row["session_id"].as_str().unwrap_or("").to_string();
        let target = row["target"].as_str().unwrap_or("").to_string();
        if session.is_empty() || target.is_empty() {
            continue;
        }
        sessions.entry(session).or_default().insert(target);
    }

    // Only sessions with 2+ distinct targets can produce co-occurrences
    let multi_target_sessions: Vec<&HashSet<String>> = sessions.values()
        .filter(|targets| targets.len() >= 2)
        .collect();

    if multi_target_sessions.is_empty() {
        return Vec::new();
    }

    let total_sessions = multi_target_sessions.len() as f64;

    // Count pair co-occurrences
    let mut pair_counts: HashMap<(String, String), u32> = HashMap::new();
    for targets in &multi_target_sessions {
        let mut sorted: Vec<&String> = targets.iter().collect();
        sorted.sort();
        // Generate pairs (ordered to avoid duplicates)
        for i in 0..sorted.len() {
            for j in (i + 1)..sorted.len() {
                let key = (sorted[i].clone(), sorted[j].clone());
                *pair_counts.entry(key).or_insert(0) += 1;
            }
        }
    }

    // Filter: at least 2 co-occurrences to matter
    let mut patterns: Vec<(String, String, f64)> = pair_counts.into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|((a, b), count)| {
            let score = count as f64 / total_sessions;
            // Shorten paths for readability — use filename only
            let short_a = a.rsplit('/').next().unwrap_or(&a);
            let short_b = b.rsplit('/').next().unwrap_or(&b);
            let id = format!("co_{:x}", content_hash(&format!("{}:{}", a, b)));
            let content = format!("{} + {} — co-edited in {}% of sessions", short_a, short_b, (score * 100.0) as u32);
            (id, content, score)
        })
        .collect();

    // Sort by score descending — strongest co-occurrences first
    patterns.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    patterns.truncate(20); // Cap at 20 co-occurrence patterns
    patterns
}

/// Run a full aggregation cycle against storage.
/// Returns the number of patterns fed into CCM.
pub async fn aggregate_observations(
    storage: &dyn crate::domain::storage::StoragePort,
    project: &str,
) -> u32 {
    // Get top patterns (target+tool frequency) — single query, derive total from results
    let rows = match storage.query_observations(project, None, 50).await {
        Ok(r) => r,
        Err(_) => return 0,
    };

    // Derive total from the same result set — atomic, no race between two queries
    let total = rows.iter()
        .filter_map(|r| r["freq"].as_u64())
        .sum::<u64>()
        .max(1); // Avoid division by zero

    let patterns = extract_patterns(&rows, total);

    // Phase 2: Co-occurrence patterns (targets edited together in the same session)
    let session_rows = storage.query_session_targets(project, 500).await
        .unwrap_or_default();
    let cooccurrences = extract_cooccurrences(&session_rows);

    // Workflow patterns are ANALYTICS, not epistemic memory.
    // They are NOT fed into the crystal system because:
    // 1. Frequency ratios (freq/total ≈ 0.01–0.30) can NEVER reach crystallization threshold (0.618)
    // 2. Content is tool usage logs ("Read main.rs — 152x"), not knowledge
    // 3. Mixing operational telemetry with judgment crystals degrades prompt injection quality
    //    (confirmed by RAG contamination research — PoisonedRAG, USENIX Security 2025)
    //
    // Only the judge pipeline (pipeline.rs::side_effects) creates epistemic crystals,
    // using phi-bounded Q-Scores from consensus evaluation.
    let freq_count = patterns.len() as u32;
    let cooccur_count = cooccurrences.len() as u32;

    klog!("[CCM/aggregate] {} freq + {} co-occur patterns from {} observations (analytics only)",
        freq_count, cooccur_count, total);

    freq_count + cooccur_count
}


// ── TESTS ───────────────────────────────────────────────────
#[cfg(test)]
mod tests {
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
        let c = make_crystal(PHI_INV, MIN_CRYSTALLIZATION_CYCLES - 1, CrystalState::Forming);
        let state = observe(&c, PHI_INV);
        assert_eq!(state, CrystalState::Crystallized);
    }

    #[test]
    fn does_not_crystallize_below_threshold() {
        let c = make_crystal(0.5, MIN_CRYSTALLIZATION_CYCLES - 1, CrystalState::Forming);
        let state = observe(&c, 0.5);
        // 0.5 < PHI_INV (0.618), so stays Forming
        assert_eq!(state, CrystalState::Forming);
    }

    #[test]
    fn decays_when_confidence_drops() {
        let c = make_crystal(PHI_INV, 25, CrystalState::Crystallized);
        // Feed many low scores to drag confidence below PHI_INV2
        let mut crystal = c;
        for _ in 0..100 {
            crystal = update_crystal(&crystal, 0.1, "now");
        }
        assert!(crystal.confidence < PHI_INV2);
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
        // Mean of [0.5, 0.7] = 0.6
        let mean = running_mean(0.5, 1, 0.7);
        assert!((mean - 0.6).abs() < 1e-10);
    }

    #[test]
    fn running_mean_first_value() {
        let mean = running_mean(0.0, 0, 0.42);
        assert!((mean - 0.42).abs() < 1e-10);
    }

    #[test]
    fn geometric_mean_drag_prevents_false_crystallization() {
        // A pattern that oscillates between high and low should NOT crystallize
        let mut c = new_crystal("osc".into(), "oscillating".into(), "test".into(), 0.9, "now");
        for i in 0..30 {
            let score = if i % 2 == 0 { 0.9 } else { 0.2 };
            c = update_crystal(&c, score, "now");
        }
        // Mean of alternating 0.9/0.2 ≈ 0.55, below PHI_INV
        assert!(c.confidence < PHI_INV);
        assert_ne!(c.state, CrystalState::Crystallized);
    }

    #[test]
    fn crystal_context_includes_mature_only() {
        let crystals = vec![
            make_crystal(PHI_INV, 25, CrystalState::Crystallized),
            make_crystal(0.3, 5, CrystalState::Forming),
            make_crystal(PHI_INV, 250, CrystalState::Canonical),
        ];
        let ctx = format_crystal_context(&crystals, "test", 2000).unwrap();
        assert!(ctx.contains("CRYSTALLIZED"));
        assert!(ctx.contains("CANONICAL"));
        assert!(!ctx.contains("Forming")); // Forming excluded
    }

    #[test]
    fn crystal_context_respects_budget() {
        let crystals: Vec<Crystal> = (0..100).map(|i| {
            let mut c = make_crystal(PHI_INV, 25, CrystalState::Crystallized);
            c.content = format!("pattern number {} with some extra text to fill space", i);
            c
        }).collect();
        let ctx = format_crystal_context(&crystals, "test", 200).unwrap();
        assert!(ctx.len() <= 300); // Some slack for header
    }

    #[test]
    fn crystal_context_empty_when_no_mature() {
        let crystals = vec![
            make_crystal(0.3, 5, CrystalState::Forming),
        ];
        assert!(format_crystal_context(&crystals, "test", 2000).is_none());
    }

    #[test]
    fn extract_patterns_basic() {
        let rows = vec![
            serde_json::json!({"target": "storage.rs", "tool": "Edit", "freq": 10}),
            serde_json::json!({"target": "judge.rs", "tool": "Edit", "freq": 5}),
            serde_json::json!({"target": "ls", "tool": "Bash", "freq": 2}), // below threshold
        ];
        let patterns = extract_patterns(&rows, 20);
        assert_eq!(patterns.len(), 2); // "ls" filtered out (freq < 3)
        assert!((patterns[0].2 - 0.5).abs() < 1e-10); // 10/20
        assert!((patterns[1].2 - 0.25).abs() < 1e-10); // 5/20
        assert!(patterns[0].1.contains("storage.rs"));
    }

    #[test]
    fn extract_patterns_empty() {
        let patterns = extract_patterns(&[], 0);
        assert!(patterns.is_empty());
    }

    #[test]
    fn cooccurrence_basic() {
        let rows = vec![
            // Session 1: edited A and B
            serde_json::json!({"session_id": "s1", "target": "/src/a.rs"}),
            serde_json::json!({"session_id": "s1", "target": "/src/b.rs"}),
            // Session 2: edited A and B again
            serde_json::json!({"session_id": "s2", "target": "/src/a.rs"}),
            serde_json::json!({"session_id": "s2", "target": "/src/b.rs"}),
            // Session 3: edited A only (no pair)
            serde_json::json!({"session_id": "s3", "target": "/src/a.rs"}),
        ];
        let patterns = extract_cooccurrences(&rows);
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].1.contains("a.rs"));
        assert!(patterns[0].1.contains("b.rs"));
        assert!((patterns[0].2 - 1.0).abs() < 1e-10); // 2 co-occur / 2 multi-target sessions
    }

    #[test]
    fn cooccurrence_filters_single_occurrence() {
        let rows = vec![
            // Only 1 session with both files — below threshold of 2
            serde_json::json!({"session_id": "s1", "target": "/src/x.rs"}),
            serde_json::json!({"session_id": "s1", "target": "/src/y.rs"}),
        ];
        let patterns = extract_cooccurrences(&rows);
        assert!(patterns.is_empty()); // Need 2+ co-occurrences
    }

    #[test]
    fn cooccurrence_empty_sessions() {
        let patterns = extract_cooccurrences(&[]);
        assert!(patterns.is_empty());
    }

    #[test]
    fn cooccurrence_skips_empty_session_id() {
        let rows = vec![
            serde_json::json!({"session_id": "", "target": "/src/a.rs"}),
            serde_json::json!({"session_id": "", "target": "/src/b.rs"}),
        ];
        let patterns = extract_cooccurrences(&rows);
        assert!(patterns.is_empty());
    }

    #[test]
    fn extract_patterns_skips_empty_targets() {
        let rows = vec![
            serde_json::json!({"target": "", "tool": "Bash", "freq": 10}),
        ];
        let patterns = extract_patterns(&rows, 10);
        assert!(patterns.is_empty());
    }

    #[test]
    fn temporal_relevance_no_decay_for_today() {
        let now = "2026-03-21T12:00:00Z";
        let updated = "2026-03-21T12:00:00Z";
        let rel = temporal_relevance(0.7, updated, now);
        assert!((rel - 0.7).abs() < 0.01, "same-day crystal should have ~no decay, got {}", rel);
    }

    #[test]
    fn temporal_relevance_decays_old_crystals() {
        let now = "2026-03-21T12:00:00Z";
        let updated_90d_ago = "2025-12-21T12:00:00Z"; // ~90 days ago
        let rel = temporal_relevance(0.7, updated_90d_ago, now);
        // At 90 days: factor = e^(-1) ≈ 0.368, so relevance ≈ 0.7 * 0.368 ≈ 0.258
        assert!(rel < 0.3, "90-day-old crystal should decay significantly, got {}", rel);
        assert!(rel > 0.2, "decay shouldn't be too extreme at 90 days, got {}", rel);
    }

    #[test]
    fn temporal_relevance_recent_beats_old_high_confidence() {
        let now = "2026-03-21T12:00:00Z";
        let recent = temporal_relevance(0.65, "2026-03-20T12:00:00Z", now); // yesterday, conf 0.65
        let old = temporal_relevance(0.70, "2025-09-21T12:00:00Z", now);    // 6 months ago, conf 0.70
        assert!(recent > old, "recent crystal ({}) should outrank old one ({})", recent, old);
    }

    #[test]
    fn temporal_relevance_bad_timestamp_returns_zero() {
        let rel = temporal_relevance(0.7, "not-a-date", "2026-03-21T12:00:00Z");
        assert!((rel - 0.0).abs() < 1e-10, "bad timestamp should return 0, got {}", rel);
    }

    #[test]
    fn crystal_context_prefers_recent_over_old() {
        // Two crystallized crystals: one recent (lower confidence), one old (higher confidence)
        let mut recent = make_crystal(0.65, 25, CrystalState::Crystallized);
        recent.updated_at = "2026-03-20T12:00:00Z".into();
        recent.content = "RECENT_PATTERN".into();

        let mut old = make_crystal(0.70, 30, CrystalState::Crystallized);
        old.updated_at = "2025-06-01T00:00:00Z".into();
        old.content = "OLD_PATTERN".into();
        old.id = "test-2".into();

        let crystals = vec![old, recent]; // old first in input
        let ctx = format_crystal_context(&crystals, "test", 2000).unwrap();
        // Recent should appear before old after decay-weighted sorting
        let recent_pos = ctx.find("RECENT_PATTERN").expect("recent should be in output");
        let old_pos = ctx.find("OLD_PATTERN").expect("old should be in output");
        assert!(recent_pos < old_pos, "recent crystal should rank before old crystal");
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
        let summaries: Vec<SessionSummary> = (0..50).map(|i| {
            make_session_summary(&format!("sess-{:03}", i), 10, &format!("Did something important in session number {}", i))
        }).collect();
        let ctx = format_session_context(&summaries, 200).unwrap();
        assert!(ctx.len() <= 300); // Slack for header
    }

    #[test]
    fn summarization_prompt_formats_observations() {
        let obs = vec![
            serde_json::json!({"tool": "Edit", "target": "ccm.rs", "domain": "code", "status": "ok"}),
            serde_json::json!({"tool": "Bash", "target": "cargo test", "domain": "workflow", "status": "ok"}),
        ];
        let prompt = format_summarization_prompt(&obs);
        assert!(prompt.contains("Summarize this development session"));
        assert!(prompt.contains("Edit ccm.rs"));
        assert!(prompt.contains("Bash cargo test"));
    }

    #[test]
    fn summarization_prompt_caps_at_30_observations() {
        let obs: Vec<serde_json::Value> = (0..50).map(|i| {
            serde_json::json!({"tool": "Edit", "target": format!("file{}.rs", i), "domain": "code", "status": "ok"})
        }).collect();
        let prompt = format_summarization_prompt(&obs);
        assert!(prompt.contains("file29.rs")); // 30th (index 29) should be included
        assert!(!prompt.contains("file30.rs")); // 31st should be excluded
    }

    #[test]
    fn phi_bounds_are_consistent() {
        // Verify our thresholds match the phi constants
        assert!(PHI_INV > PHI_INV2);
        assert!((PHI_INV - 0.618).abs() < 0.001);
        assert!((PHI_INV2 - 0.382).abs() < 0.001);
    }
}
