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
/// Single source of truth — used by both SurrealDB SQL and InMemory adapter.
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
    /// Verdict IDs that contributed observations to this crystal (provenance trail).
    /// Enables auditing: "why did this crystal crystallize?" → trace back to verdicts.
    #[serde(default)]
    pub contributing_verdicts: Vec<String>,
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

impl std::fmt::Display for CrystalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forming => write!(f, "forming"),
            Self::Crystallized => write!(f, "crystallized"),
            Self::Canonical => write!(f, "canonical"),
            Self::Decaying => write!(f, "decaying"),
            Self::Dissolved => write!(f, "dissolved"),
        }
    }
}

impl std::str::FromStr for CrystalState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "forming" => Ok(Self::Forming),
            "crystallized" => Ok(Self::Crystallized),
            "canonical" => Ok(Self::Canonical),
            "decaying" => Ok(Self::Decaying),
            "dissolved" => Ok(Self::Dissolved),
            other => Err(format!("unknown crystal state: {other}")),
        }
    }
}

// ── MATURE CRYSTAL NEWTYPE (T4) ─────────────────────────────
/// A crystal that has reached Crystallized or Canonical state.
/// Private inner field — can only be constructed via `TryFrom<Crystal>`,
/// which validates the state at compile-time boundary. This prevents
/// Forming/Decaying/Dissolved crystals from reaching Dog prompts.
#[derive(Debug, Clone)]
pub struct MatureCrystal {
    inner: Crystal,
}

/// Error when attempting to create a MatureCrystal from a non-mature Crystal.
#[derive(Debug)]
pub struct NotMatureError {
    pub id: String,
    pub state: CrystalState,
}

impl std::fmt::Display for NotMatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "crystal '{}' is {:?}, not mature", self.id, self.state)
    }
}

impl std::error::Error for NotMatureError {}

impl TryFrom<Crystal> for MatureCrystal {
    type Error = NotMatureError;

    fn try_from(crystal: Crystal) -> Result<Self, Self::Error> {
        match crystal.state {
            CrystalState::Crystallized | CrystalState::Canonical => {
                Ok(MatureCrystal { inner: crystal })
            }
            _ => Err(NotMatureError {
                id: crystal.id.clone(),
                state: crystal.state,
            }),
        }
    }
}

impl MatureCrystal {
    /// Access the underlying Crystal (read-only).
    pub fn crystal(&self) -> &Crystal {
        &self.inner
    }

    pub fn id(&self) -> &str {
        &self.inner.id
    }

    pub fn content(&self) -> &str {
        &self.inner.content
    }

    pub fn domain(&self) -> &str {
        &self.inner.domain
    }

    pub fn confidence(&self) -> f64 {
        self.inner.confidence
    }

    pub fn observations(&self) -> u32 {
        self.inner.observations
    }

    pub fn state(&self) -> &CrystalState {
        &self.inner.state
    }

    pub fn updated_at(&self) -> &str {
        &self.inner.updated_at
    }
}

/// Filter a list of Crystals into only mature ones.
/// Convenience function — equivalent to try_from + filter_map.
pub fn filter_mature(crystals: Vec<Crystal>) -> Vec<MatureCrystal> {
    crystals
        .into_iter()
        .filter_map(|c| MatureCrystal::try_from(c).ok())
        .collect()
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
        contributing_verdicts: crystal.contributing_verdicts.clone(),
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
/// Decay constant: 90 days. At 90 days, factor = e⁻¹ ≈ 0.368.
const DECAY_DAYS: f64 = 90.0;

/// Compute decay relevance: `confidence × e^(-age_days / DECAY_DAYS)`.
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
/// Filters by domain (including "general" cross-domain).
/// Token-budget-aware: caps at max_chars to avoid overflowing small models.
/// Content wrapped in delimiters (T7 defense-in-depth).
pub fn format_crystal_context(
    crystals: &[MatureCrystal],
    domain: &str,
    max_chars: usize,
) -> Option<String> {
    let domain_filtered: Vec<&MatureCrystal> = crystals
        .iter()
        .filter(|c| c.domain() == domain || c.domain() == "general")
        .collect();

    if domain_filtered.is_empty() {
        return None;
    }

    // Sort by temporal relevance descending — recent high-confidence crystals first
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
        // T7 defense-in-depth: wrap content in delimiters
        let delimited = crate::domain::sanitize::delimit_crystal_content(c.content());
        let line = format!(
            "- [{}] (confidence: {:.2}, {} observations): {}",
            state_label,
            c.confidence(),
            c.observations(),
            delimited
        );
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
        return None; // Only header, no summaries fit
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

/// Normalize content before FNV hashing to reduce fragmentation.
/// Lowercase + collapse whitespace + trim. Two stimuli that differ only
/// in casing or spacing will hash to the same crystal ID.
/// Used as fallback when embedding-based KNN merge is unavailable.
pub fn normalize_for_hash(input: &str) -> String {
    input
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// ── WORKFLOW AGGREGATOR ────────────────────────────────────
/// Aggregate raw observations into CCM crystals. Runs periodically.
/// Extracts frequency patterns and co-occurrences from the observation table,
/// then feeds them into observe_crystal as workflow domain crystals.
///
/// Pure logic: takes query results, returns (id, content, domain, score) tuples.
/// Caller is responsible for DB queries and observe_crystal calls.
pub fn extract_patterns(
    rows: &[crate::domain::storage::ObservationFrequency],
    total_observations: u64,
) -> Vec<(String, String, f64)> {
    if total_observations == 0 {
        return Vec::new();
    }

    let mut patterns: Vec<(String, String, f64)> = Vec::new();

    for row in rows {
        if row.target.is_empty() || row.freq < 3 {
            continue; // Skip noise — at least 3 occurrences to matter
        }

        let score = row.freq as f64 / total_observations as f64;
        // Crystal ID: deterministic hash of the pattern
        let id = format!(
            "wf_{:x}",
            content_hash(&format!("{}:{}", row.tool, row.target))
        );
        // Human-readable content for injection into prompts
        let content = format!("{} {} — {}x observed", row.tool, row.target, row.freq);

        patterns.push((id, content, score));
    }

    patterns
}

/// Extract co-occurrence patterns from session-grouped observations.
/// Input: rows of {session_id, target} sorted by session_id.
/// Output: (crystal_id, content, score) for pairs that co-occur in 2+ sessions.
///
/// Pure function — all co-occurrence computation happens in Rust, not SQL.
pub fn extract_cooccurrences(
    rows: &[crate::domain::storage::SessionTarget],
) -> Vec<(String, String, f64)> {
    use std::collections::{HashMap, HashSet};

    // Group targets by session
    let mut sessions: HashMap<String, HashSet<String>> = HashMap::new();
    for row in rows {
        if row.session_id.is_empty() || row.target.is_empty() {
            continue;
        }
        sessions
            .entry(row.session_id.clone())
            .or_default()
            .insert(row.target.clone());
    }

    // Only sessions with 2+ distinct targets can produce co-occurrences
    let multi_target_sessions: Vec<&HashSet<String>> = sessions
        .values()
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
    let mut patterns: Vec<(String, String, f64)> = pair_counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|((a, b), count)| {
            let score = count as f64 / total_sessions;
            // Shorten paths for readability — use filename only
            let short_a = a.rsplit('/').next().unwrap_or(&a);
            let short_b = b.rsplit('/').next().unwrap_or(&b);
            let id = format!("co_{:x}", content_hash(&format!("{a}:{b}")));
            let content = format!(
                "{} + {} — co-edited in {}% of sessions",
                short_a,
                short_b,
                (score * 100.0) as u32
            );
            (id, content, score)
        })
        .collect();

    // Sort by score descending, then by id for deterministic tie-breaking
    patterns.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
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
    let Ok(rows) = storage.query_observations(project, None, 50).await else {
        return 0;
    };

    // Derive total from the same result set — atomic, no race between two queries
    let total = rows.iter().map(|r| r.freq).sum::<u64>().max(1); // Avoid division by zero

    let patterns = extract_patterns(&rows, total);

    // Phase 2: Co-occurrence patterns (targets edited together in the same session)
    let session_rows = storage
        .query_session_targets(project, 500)
        .await
        .inspect_err(|e| klog!("[CCM/aggregate] session_targets query failed: {}", e))
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

    klog!(
        "[CCM/aggregate] {} freq + {} co-occur patterns from {} observations (analytics only)",
        freq_count,
        cooccur_count,
        total
    );

    freq_count + cooccur_count
}

// ── DOMAIN INFERENCE ────────────────────────────────────────
/// Infer domain from file extension or target pattern.
/// Moved from api/rest/observe.rs — this is domain logic (K5 compliance).
pub fn infer_domain(target: Option<&str>, tool: Option<&str>) -> String {
    // Tool-based inference (highest priority)
    if let Some(t) = tool
        && t == "self-probe"
    {
        return "infra".to_string();
    }

    let target = match target {
        Some(t) if !t.is_empty() => t,
        _ => return "general".to_string(),
    };

    // Infra target patterns
    if target.starts_with("cynic-")
        || target.contains("llama-server")
        || target.contains("surrealdb")
    {
        return "infra".to_string();
    }

    // Extension-based inference
    target
        .rsplit('.')
        .next()
        .map(|ext| match ext {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "py" => "python",
            "md" => "docs",
            "toml" | "json" | "yaml" | "yml" => "config",
            "service" | "timer" => "infra",
            _ => "general",
        })
        .unwrap_or("general")
        .to_string()
}

// ── OBSERVATION BUILDER ─────────────────────────────────────
/// Build an `Observation` from raw API parameters.
///
/// Single source of truth for the REST and MCP observe surfaces (K3/K13).
/// Both `POST /observe` and `cynic_observe` call this — zero duplication.
#[allow(clippy::too_many_arguments)]
// WHY: K13 — single shared function for REST+MCP observe surfaces. All 8 fields
// map directly to Observation struct fields; grouping into a sub-struct would
// add indirection without reducing complexity.
pub fn build_observation(
    tool: String,
    target: Option<String>,
    domain: Option<String>,
    status: Option<String>,
    context: Option<String>,
    project: Option<String>,
    agent_id: Option<String>,
    session_id: Option<String>,
) -> crate::domain::storage::Observation {
    let resolved_domain = domain.unwrap_or_else(|| infer_domain(target.as_deref(), Some(&tool)));

    crate::domain::storage::Observation {
        project: project.unwrap_or_else(|| "CYNIC".into()),
        agent_id: agent_id.unwrap_or_else(|| "unknown".into()),
        tool,
        target: crate::domain::sanitize::sanitize_observation_target(&target.unwrap_or_default()),
        domain: resolved_domain,
        status: status.unwrap_or_else(|| "success".into()),
        context: context
            .map(|c| c.chars().take(200).collect())
            .unwrap_or_default(),
        session_id: session_id.unwrap_or_default(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
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
            contributing_verdicts: vec![],
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
        // T4: filter_mature drops Forming, format_crystal_context gets only mature
        let mature = filter_mature(crystals);
        assert_eq!(mature.len(), 2); // Forming filtered out by type system
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
        let ctx = format_crystal_context(&mature, "test", 200).unwrap();
        assert!(ctx.len() <= 500); // Slack for header + delimiters
    }

    #[test]
    fn crystal_context_empty_when_no_mature() {
        let crystals = vec![make_crystal(0.3, 5, CrystalState::Forming)];
        let mature = filter_mature(crystals);
        assert!(mature.is_empty()); // T4: Forming rejected by type system
        assert!(format_crystal_context(&mature, "test", 2000).is_none());
    }

    #[test]
    fn mature_crystal_rejects_forming() {
        let forming = make_crystal(0.5, 5, CrystalState::Forming);
        assert!(MatureCrystal::try_from(forming).is_err());
    }

    #[test]
    fn mature_crystal_accepts_crystallized() {
        let crystallized = make_crystal(PHI_INV, 25, CrystalState::Crystallized);
        assert!(MatureCrystal::try_from(crystallized).is_ok());
    }

    #[test]
    fn mature_crystal_accepts_canonical() {
        let canonical = make_crystal(PHI_INV, 250, CrystalState::Canonical);
        assert!(MatureCrystal::try_from(canonical).is_ok());
    }

    #[test]
    fn mature_crystal_rejects_decaying() {
        let decaying = make_crystal(0.2, 30, CrystalState::Decaying);
        assert!(MatureCrystal::try_from(decaying).is_err());
    }

    #[test]
    fn extract_patterns_basic() {
        use crate::domain::storage::ObservationFrequency;
        let rows = vec![
            ObservationFrequency {
                target: "storage.rs".into(),
                tool: "Edit".into(),
                freq: 10,
            },
            ObservationFrequency {
                target: "judge.rs".into(),
                tool: "Edit".into(),
                freq: 5,
            },
            ObservationFrequency {
                target: "ls".into(),
                tool: "Bash".into(),
                freq: 2,
            }, // below threshold
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
        use crate::domain::storage::SessionTarget;
        let rows = vec![
            SessionTarget {
                session_id: "s1".into(),
                target: "/src/a.rs".into(),
            },
            SessionTarget {
                session_id: "s1".into(),
                target: "/src/b.rs".into(),
            },
            SessionTarget {
                session_id: "s2".into(),
                target: "/src/a.rs".into(),
            },
            SessionTarget {
                session_id: "s2".into(),
                target: "/src/b.rs".into(),
            },
            SessionTarget {
                session_id: "s3".into(),
                target: "/src/a.rs".into(),
            },
        ];
        let patterns = extract_cooccurrences(&rows);
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].1.contains("a.rs"));
        assert!(patterns[0].1.contains("b.rs"));
        assert!((patterns[0].2 - 1.0).abs() < 1e-10); // 2 co-occur / 2 multi-target sessions
    }

    #[test]
    fn cooccurrence_filters_single_occurrence() {
        use crate::domain::storage::SessionTarget;
        let rows = vec![
            SessionTarget {
                session_id: "s1".into(),
                target: "/src/x.rs".into(),
            },
            SessionTarget {
                session_id: "s1".into(),
                target: "/src/y.rs".into(),
            },
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
        use crate::domain::storage::SessionTarget;
        let rows = vec![
            SessionTarget {
                session_id: "".into(),
                target: "/src/a.rs".into(),
            },
            SessionTarget {
                session_id: "".into(),
                target: "/src/b.rs".into(),
            },
        ];
        let patterns = extract_cooccurrences(&rows);
        assert!(patterns.is_empty());
    }

    #[test]
    fn extract_patterns_skips_empty_targets() {
        use crate::domain::storage::ObservationFrequency;
        let rows = vec![ObservationFrequency {
            target: "".into(),
            tool: "Bash".into(),
            freq: 10,
        }];
        let patterns = extract_patterns(&rows, 10);
        assert!(patterns.is_empty());
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
        let updated_90d_ago = "2025-12-21T12:00:00Z"; // ~90 days ago
        let rel = decay_relevance(0.7, updated_90d_ago, now);
        // At 90 days: factor = e^(-1) ≈ 0.368, so relevance ≈ 0.7 * 0.368 ≈ 0.258
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
        let recent = decay_relevance(0.65, "2026-03-20T12:00:00Z", now); // yesterday, conf 0.65
        let old = decay_relevance(0.70, "2025-09-21T12:00:00Z", now); // 6 months ago, conf 0.70
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
        // Two crystallized crystals: one recent (lower confidence), one old (higher confidence)
        let mut recent = make_crystal(0.65, 25, CrystalState::Crystallized);
        recent.updated_at = "2026-03-20T12:00:00Z".into();
        recent.content = "RECENT_PATTERN".into();

        let mut old = make_crystal(0.70, 30, CrystalState::Crystallized);
        old.updated_at = "2025-06-01T00:00:00Z".into();
        old.content = "OLD_PATTERN".into();
        old.id = "test-2".into();

        let mature = filter_mature(vec![old, recent]); // old first in input
        let ctx = format_crystal_context(&mature, "test", 2000).unwrap();
        // Recent should appear before old after decay-weighted sorting
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
        assert!(ctx.len() <= 300); // Slack for header
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
            })
            .collect();
        let prompt = format_summarization_prompt(&obs);
        assert!(prompt.contains("file29.rs")); // 30th (index 29) should be included
        assert!(!prompt.contains("file30.rs")); // 31st should be excluded
    }

    #[test]
    fn phi_bounds_are_consistent() {
        // Verify our thresholds match the phi constants (runtime check via let binding)
        let phi_inv = PHI_INV;
        let phi_inv2 = PHI_INV2;
        assert!(
            (phi_inv - 0.618).abs() < 0.001,
            "PHI_INV should be ~0.618, got {phi_inv}"
        );
        assert!(
            (phi_inv2 - 0.382).abs() < 0.001,
            "PHI_INV2 should be ~0.382, got {phi_inv2}"
        );
        assert!(
            phi_inv > phi_inv2,
            "PHI_INV ({phi_inv}) must be > PHI_INV2 ({phi_inv2})"
        );
    }

    #[test]
    fn normalized_qscore_enables_crystallization() {
        // Simulates the pipeline normalization: Q-Score / PHI_INV → confidence.
        // Without normalization, realistic Q-Scores (~0.55) never reach 0.618.
        // With normalization, they map to ~0.89 and crystallize correctly.
        let typical_qscore = 0.55; // realistic Sicilian Defense Q-Score
        let normalized = (typical_qscore / PHI_INV).min(1.0);

        // Build crystal with 20 observations of normalized score, then observe 21st
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
    fn normalized_bad_qscore_does_not_crystallize() {
        // Fool's Mate Q-Score ~0.08 → normalized ~0.13 → never crystallizes.
        let bad_qscore = 0.08;
        let normalized = (bad_qscore / PHI_INV).min(1.0);

        let c = make_crystal(
            normalized,
            MIN_CRYSTALLIZATION_CYCLES - 1,
            CrystalState::Forming,
        );
        let state = observe(&c, normalized);
        assert_ne!(
            state,
            CrystalState::Crystallized,
            "Normalized bad Q-Score {normalized:.3} (raw {bad_qscore:.3}) must NOT crystallize"
        );
    }

    #[test]
    fn raw_qscore_cannot_crystallize() {
        // Proves the bug: raw Q-Scores can never reach crystallization threshold.
        let good_qscore = 0.57; // best chess score observed

        let c = make_crystal(
            good_qscore,
            MIN_CRYSTALLIZATION_CYCLES - 1,
            CrystalState::Forming,
        );
        let state = observe(&c, good_qscore);
        assert_ne!(
            state,
            CrystalState::Crystallized,
            "Raw Q-Score {good_qscore:.3} must NOT crystallize (proves the bug existed)"
        );
    }

    // ── infer_domain (moved from api/rest/observe.rs) ──

    #[test]
    fn infer_domain_rust() {
        assert_eq!(infer_domain(Some("src/judge.rs"), None), "rust");
    }

    #[test]
    fn infer_domain_typescript() {
        assert_eq!(infer_domain(Some("App.tsx"), None), "typescript");
        assert_eq!(infer_domain(Some("utils.ts"), None), "typescript");
    }

    #[test]
    fn infer_domain_javascript() {
        assert_eq!(infer_domain(Some("index.js"), None), "javascript");
    }

    #[test]
    fn infer_domain_python() {
        assert_eq!(infer_domain(Some("train.py"), None), "python");
    }

    #[test]
    fn infer_domain_config() {
        assert_eq!(infer_domain(Some("Cargo.toml"), None), "config");
        assert_eq!(infer_domain(Some("package.json"), None), "config");
        assert_eq!(infer_domain(Some("config.yaml"), None), "config");
    }

    #[test]
    fn infer_domain_docs() {
        assert_eq!(infer_domain(Some("README.md"), None), "docs");
    }

    #[test]
    fn infer_domain_general_fallback() {
        assert_eq!(infer_domain(Some("binary.wasm"), None), "general");
        assert_eq!(infer_domain(None, None), "general");
        assert_eq!(infer_domain(Some("Makefile"), None), "general");
    }

    #[test]
    fn infer_domain_infra_service() {
        assert_eq!(infer_domain(Some("cynic-kernel.service"), None), "infra");
        assert_eq!(infer_domain(Some("backup.timer"), None), "infra");
    }

    #[test]
    fn infer_domain_infra_targets() {
        assert_eq!(infer_domain(Some("cynic-core"), None), "infra");
        assert_eq!(infer_domain(Some("cynic-gpu"), None), "infra");
        assert_eq!(infer_domain(Some("llama-server"), None), "infra");
        assert_eq!(infer_domain(Some("surrealdb"), None), "infra");
    }

    #[test]
    fn infer_domain_self_probe_tool() {
        assert_eq!(infer_domain(Some("localhost"), Some("self-probe")), "infra");
        assert_eq!(infer_domain(None, Some("self-probe")), "infra");
    }

    // ── CrystalState Display/FromStr contract (regression: session-init.sh jq) ──

    #[test]
    fn crystal_state_display_is_lowercase() {
        assert_eq!(CrystalState::Forming.to_string(), "forming");
        assert_eq!(CrystalState::Crystallized.to_string(), "crystallized");
        assert_eq!(CrystalState::Canonical.to_string(), "canonical");
        assert_eq!(CrystalState::Decaying.to_string(), "decaying");
        assert_eq!(CrystalState::Dissolved.to_string(), "dissolved");
    }

    #[test]
    fn crystal_state_round_trip() {
        for state in [
            CrystalState::Forming,
            CrystalState::Crystallized,
            CrystalState::Canonical,
            CrystalState::Decaying,
            CrystalState::Dissolved,
        ] {
            let s = state.to_string();
            let parsed: CrystalState = s.parse().unwrap();
            assert_eq!(parsed, state);
        }
    }

    // ── normalize_for_hash ──────────────────────────────────

    #[test]
    fn normalize_collapses_whitespace_and_lowercases() {
        assert_eq!(
            normalize_for_hash("  Chess:  1. E4  c5  "),
            "chess: 1. e4 c5"
        );
    }

    #[test]
    fn normalize_identical_content_hashes_same() {
        let a = content_hash(&normalize_for_hash("chess:1. e4 c5 — The Sicilian Defense"));
        let b = content_hash(&normalize_for_hash(
            "chess:1. e4 c5 —  The  Sicilian  Defense",
        ));
        let c = content_hash(&normalize_for_hash("Chess:1. E4 C5 — The Sicilian Defense"));
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn normalize_different_content_hashes_differ() {
        let a = content_hash(&normalize_for_hash("chess:1. e4 c5"));
        let b = content_hash(&normalize_for_hash("chess:1. d4 d5"));
        assert_ne!(a, b);
    }
}
