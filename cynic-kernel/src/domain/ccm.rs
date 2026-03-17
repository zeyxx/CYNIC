//! CCM — Cognitive Crystallization Mechanism.
//! Ephemeral verdicts → persistent wisdom. Domain-pure logic.
//!
//! A pattern that scores >= φ⁻¹ (0.618) repeatedly across 21+ cycles
//! crystallizes into persistent wisdom. Below φ⁻² (0.382) it decays.
//! This is how CYNIC learns without training — through phi-bounded consensus.

use serde::{Deserialize, Serialize};
use crate::domain::dog::{PHI_INV, PHI_INV2};

/// Fibonacci F(8) = 21 — minimum observations before crystallization.
pub const MIN_CRYSTALLIZATION_CYCLES: u32 = 21;
/// Fibonacci F(13) = 233 — canonical status (deeply crystallized).
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
/// This is the core CCM algorithm — stateless, pure function.
pub fn observe(crystal: &Crystal, new_score: f64) -> CrystalState {
    let next_obs = crystal.observations + 1;
    let next_confidence = running_mean(crystal.confidence, crystal.observations, new_score);

    classify(next_confidence, next_obs)
}

/// Floating-point tolerance for phi-threshold comparisons.
const EPSILON: f64 = 1e-10;

/// Classify a crystal based on its confidence and observation count.
fn classify(confidence: f64, observations: u32) -> CrystalState {
    if confidence < PHI_INV2 - EPSILON {
        if observations > MIN_CRYSTALLIZATION_CYCLES {
            CrystalState::Decaying
        } else {
            CrystalState::Dissolved
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
/// Pure function — caller is responsible for persistence.
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
fn running_mean(current_mean: f64, count: u32, new_value: f64) -> f64 {
    if count == 0 {
        return new_value;
    }
    let n = count as f64;
    (current_mean * n + new_value) / (n + 1.0)
}

// ── CCM FEEDBACK — inject crystallized wisdom into stimulus context ──

/// Format mature crystals as context for Dog prompts.
/// Only includes Crystallized and Canonical crystals from the same domain.
/// Token-budget-aware: caps at max_chars to avoid overflowing small models.
pub fn format_crystal_context(crystals: &[Crystal], domain: &str, max_chars: usize) -> Option<String> {
    let mature: Vec<&Crystal> = crystals.iter()
        .filter(|c| c.domain == domain || domain == "general")
        .filter(|c| matches!(c.state, CrystalState::Crystallized | CrystalState::Canonical))
        .collect();

    if mature.is_empty() {
        return None;
    }

    // Sort by confidence descending — highest-value crystals first (agentkeeper pattern)
    let mut sorted = mature;
    sorted.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

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

/// Full BLAKE3 hash of a verdict's content — for integrity chain.
/// Hashes: id + q_score.total + all 6 axiom scores + stimulus + timestamp + prev_hash.
/// Returns hex-encoded 256-bit hash.
pub fn verdict_hash(
    id: &str,
    q_total: f64,
    scores: [f64; 6],
    stimulus: &str,
    timestamp: &str,
    prev_hash: Option<&str>,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(id.as_bytes());
    hasher.update(&q_total.to_le_bytes());
    for s in &scores {
        hasher.update(&s.to_le_bytes());
    }
    hasher.update(stimulus.as_bytes());
    hasher.update(timestamp.as_bytes());
    if let Some(ph) = prev_hash {
        hasher.update(ph.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
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
    // Get total observation count for score normalization
    let total = match storage.query_observations(project, None, 1).await {
        Ok(rows) => {
            // The query returns grouped rows — sum their frequencies for total
            rows.iter()
                .filter_map(|r| r["freq"].as_u64())
                .sum::<u64>()
                .max(1) // Avoid division by zero
        }
        Err(_) => return 0,
    };

    // Get top patterns (target+tool frequency)
    let rows = match storage.query_observations(project, None, 50).await {
        Ok(r) => r,
        Err(_) => return 0,
    };

    let patterns = extract_patterns(&rows, total);
    let now = chrono::Utc::now().to_rfc3339();
    let mut count = 0u32;

    for (id, content, score) in &patterns {
        if let Err(e) = storage.observe_crystal(id, content, "workflow", *score, &now).await {
            eprintln!("[CCM/aggregate] Warning: failed to observe crystal {}: {}", id, e);
        } else {
            count += 1;
        }
    }

    // Phase 2: Co-occurrence patterns (targets edited together in the same session)
    let session_rows = storage.query_session_targets(project, 500).await
        .unwrap_or_default();
    let cooccurrences = extract_cooccurrences(&session_rows);
    for (id, content, score) in &cooccurrences {
        if let Err(e) = storage.observe_crystal(id, content, "workflow", *score, &now).await {
            eprintln!("[CCM/aggregate] Warning: failed to observe co-occurrence crystal {}: {}", id, e);
        } else {
            count += 1;
        }
    }

    if count > 0 {
        klog!("[CCM/aggregate] {} patterns crystallized ({} freq + {} co-occur) from {} observations",
            count, count - cooccurrences.len() as u32, cooccurrences.len(), total);
    }

    count
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
    fn phi_bounds_are_consistent() {
        // Verify our thresholds match the phi constants
        assert!(PHI_INV > PHI_INV2);
        assert!((PHI_INV - 0.618).abs() < 0.001);
        assert!((PHI_INV2 - 0.382).abs() < 0.001);
    }
}
