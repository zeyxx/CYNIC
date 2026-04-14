//! CCM Intake — content hashing, pattern extraction, observation building.
//!
//! Ingestion side of the CCM pipeline: normalize content, extract patterns
//! from observations, infer domains, build observation records.

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
            continue;
        }

        let score = row.freq as f64 / total_observations as f64;
        let id = format!(
            "wf_{:x}",
            content_hash(&format!("{}:{}", row.tool, row.target))
        );
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

    let multi_target_sessions: Vec<&HashSet<String>> = sessions
        .values()
        .filter(|targets| targets.len() >= 2)
        .collect();

    if multi_target_sessions.is_empty() {
        return Vec::new();
    }

    let total_sessions = multi_target_sessions.len() as f64;

    let mut pair_counts: HashMap<(String, String), u32> = HashMap::new();
    for targets in &multi_target_sessions {
        let mut sorted: Vec<&String> = targets.iter().collect();
        sorted.sort();
        for i in 0..sorted.len() {
            for j in (i + 1)..sorted.len() {
                let key = (sorted[i].clone(), sorted[j].clone());
                *pair_counts.entry(key).or_insert(0) += 1;
            }
        }
    }

    let mut patterns: Vec<(String, String, f64)> = pair_counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|((a, b), count)| {
            let score = count as f64 / total_sessions;
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

    patterns.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    patterns.truncate(20);
    patterns
}

// ── DOMAIN INFERENCE ────────────────────────────────────────
/// Infer domain from file extension or target pattern.
/// Moved from api/rest/observe.rs — this is domain logic (K5 compliance).
pub fn infer_domain(target: Option<&str>, tool: Option<&str>) -> String {
    if let Some(t) = tool
        && t == "self-probe"
    {
        return "infra".to_string();
    }

    let target = match target {
        Some(t) if !t.is_empty() => t,
        _ => return "general".to_string(),
    };

    if target.starts_with("cynic-")
        || target.contains("llama-server")
        || target.contains("surrealdb")
    {
        return "infra".to_string();
    }

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
    tags: Option<Vec<String>>,
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
        tags: tags.unwrap_or_default(),
    }
}

// ── TESTS ───────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

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
            },
        ];
        let patterns = extract_patterns(&rows, 20);
        assert_eq!(patterns.len(), 2);
        assert!((patterns[0].2 - 0.5).abs() < 1e-10);
        assert!((patterns[1].2 - 0.25).abs() < 1e-10);
        assert!(patterns[0].1.contains("storage.rs"));
    }

    #[test]
    fn extract_patterns_empty() {
        let patterns = extract_patterns(&[], 0);
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
        assert!((patterns[0].2 - 1.0).abs() < 1e-10);
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
        assert!(patterns.is_empty());
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
