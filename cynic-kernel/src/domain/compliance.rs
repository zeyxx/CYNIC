//! Session compliance scoring — deterministic heuristics for workflow quality.
//! Pure domain logic. No external dependencies. Analyzes observation sequences
//! to produce a structured compliance score per session.
//!
//! Three heuristics (φ-weighted):
//! - Read-before-Edit ratio: proxy for "diagnose before fixing" (Rule 6)
//! - Bash retry penalty: proxy for "2 fix attempts max" (Rule 6)
//! - Modification scope: tracks file count for /distill threshold awareness

use crate::domain::dog::PHI_INV;
use crate::domain::storage::RawObservation;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Structured compliance report for a single session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCompliance {
    pub session_id: String,
    pub agent_id: String,
    /// Composite score: 0.0 (worst) to 1.0 (best). φ-bounded max = 0.618.
    pub score: f64,
    /// Human-readable warnings for the session-stop display.
    pub warnings: Vec<String>,
    /// Ratio of Edit operations preceded by a Read of the same file.
    pub read_before_edit: f64,
    /// Count of consecutive similar Bash failures (>2 = Rule 6 violation).
    pub bash_retry_violations: u32,
    /// Number of unique files modified (Edit/Write).
    pub files_modified: u32,
    pub created_at: String,
}

/// Score a session's workflow compliance from its raw observations.
/// Pure function — deterministic, testable, no I/O.
pub fn score_session(
    session_id: &str,
    agent_id: &str,
    observations: &[RawObservation],
) -> SessionCompliance {
    let now = chrono::Utc::now().to_rfc3339();

    if observations.is_empty() {
        return SessionCompliance {
            session_id: session_id.to_string(),
            agent_id: agent_id.to_string(),
            score: PHI_INV, // No observations = no violations = max score
            warnings: vec![],
            read_before_edit: 1.0,
            bash_retry_violations: 0,
            files_modified: 0,
            created_at: now,
        };
    }

    let mut warnings = Vec::new();

    // ── Heuristic 1: Read-before-Edit ratio ──
    let read_before_edit = compute_read_before_edit(observations);
    if read_before_edit < 0.5 {
        warnings.push(format!(
            "Low Read-before-Edit ratio: {:.0}% — read files before editing (Rule 6)",
            read_before_edit * 100.0
        ));
    }

    // ── Heuristic 2: Bash retry penalty ──
    let bash_retry_violations = count_bash_retry_violations(observations);
    if bash_retry_violations > 0 {
        warnings.push(format!(
            "{bash_retry_violations} Bash retry loop(s) detected — 2 attempts max, then diagnose (Rule 6)"
        ));
    }

    // ── Heuristic 3: Files modified scope ──
    let files_modified = count_files_modified(observations);
    if files_modified > 5 {
        warnings.push(format!(
            "{files_modified} files modified — consider invoking /distill"
        ));
    }

    // ── Composite score (φ-weighted) ──
    // Weights: read_before_edit (φ=1.618), bash_retry (1.0), scope (φ⁻¹=0.618)
    let bash_score = if bash_retry_violations == 0 {
        1.0
    } else {
        (1.0 / (1.0 + f64::from(bash_retry_violations))).max(0.0)
    };
    let scope_score = if files_modified <= 5 { 1.0 } else { 0.7 };

    let phi = 1.618_034;
    let phi_inv = 0.618_034;
    let weighted_sum = read_before_edit * phi + bash_score * 1.0 + scope_score * phi_inv;
    let weight_total = phi + 1.0 + phi_inv;
    let raw_score = weighted_sum / weight_total;

    // Cap at φ⁻¹ (max confidence = 0.618)
    let score = raw_score.min(PHI_INV);

    SessionCompliance {
        session_id: session_id.to_string(),
        agent_id: agent_id.to_string(),
        score,
        warnings,
        read_before_edit,
        bash_retry_violations,
        files_modified,
        created_at: now,
    }
}

/// For each Edit, check if a Read of the same file preceded it in the session.
/// Returns ratio: 1.0 = all edits had prior reads, 0.0 = none did.
fn compute_read_before_edit(observations: &[RawObservation]) -> f64 {
    let mut read_files: HashSet<String> = HashSet::new();
    let mut edits_total = 0u32;
    let mut edits_with_read = 0u32;

    for obs in observations {
        match obs.tool.as_str() {
            "Read" => {
                if !obs.target.is_empty() {
                    read_files.insert(obs.target.clone());
                }
            }
            "Edit" | "Write" => {
                if !obs.target.is_empty() {
                    edits_total += 1;
                    if read_files.contains(&obs.target) {
                        edits_with_read += 1;
                    }
                }
            }
            _ => {}
        }
    }

    if edits_total == 0 {
        return 1.0; // No edits = no violations
    }
    f64::from(edits_with_read) / f64::from(edits_total)
}

/// Count occurrences of >2 consecutive similar Bash failures.
/// "Similar" = same first 40 chars of target (command prefix).
fn count_bash_retry_violations(observations: &[RawObservation]) -> u32 {
    let mut violations = 0u32;
    let mut consecutive_fails = 0u32;
    let mut last_bash_prefix = String::new();

    for obs in observations {
        if obs.tool != "Bash" {
            // Non-Bash breaks the streak
            consecutive_fails = 0;
            last_bash_prefix.clear();
            continue;
        }

        if obs.status == "error" {
            let prefix: String = obs.target.chars().take(40).collect();
            if prefix == last_bash_prefix {
                consecutive_fails += 1;
                if consecutive_fails > 2 {
                    violations += 1;
                }
            } else {
                consecutive_fails = 1;
                last_bash_prefix = prefix;
            }
        } else {
            consecutive_fails = 0;
            last_bash_prefix.clear();
        }
    }

    violations
}

/// Count unique files modified by Edit or Write operations.
fn count_files_modified(observations: &[RawObservation]) -> u32 {
    let modified: HashSet<&str> = observations
        .iter()
        .filter(|o| o.tool == "Edit" || o.tool == "Write")
        .filter(|o| !o.target.is_empty())
        .map(|o| o.target.as_str())
        .collect();
    modified.len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(tool: &str, target: &str, status: &str) -> RawObservation {
        RawObservation {
            id: String::new(),
            tool: tool.to_string(),
            target: target.to_string(),
            domain: String::new(),
            status: status.to_string(),
            context: String::new(),
            created_at: String::new(),
            project: String::new(),
            agent_id: String::new(),
            session_id: String::new(),
        }
    }

    #[test]
    fn empty_session_scores_max() {
        let c = score_session("s1", "a1", &[]);
        assert!((c.score - PHI_INV).abs() < 0.001);
        assert!(c.warnings.is_empty());
    }

    #[test]
    fn read_before_edit_perfect() {
        let observations = vec![
            obs("Read", "foo.rs", "success"),
            obs("Edit", "foo.rs", "success"),
            obs("Read", "bar.rs", "success"),
            obs("Edit", "bar.rs", "success"),
        ];
        let c = score_session("s1", "a1", &observations);
        assert!((c.read_before_edit - 1.0).abs() < 0.001);
        assert!(c.warnings.is_empty());
    }

    #[test]
    fn edit_without_read_warns() {
        let observations = vec![
            obs("Edit", "foo.rs", "success"),
            obs("Edit", "bar.rs", "success"),
        ];
        let c = score_session("s1", "a1", &observations);
        assert!((c.read_before_edit - 0.0).abs() < 0.001);
        assert!(c.warnings.iter().any(|w| w.contains("Read-before-Edit")));
    }

    #[test]
    fn bash_retry_loop_detected() {
        let observations = vec![
            obs("Bash", "cargo test --release", "error"),
            obs("Bash", "cargo test --release", "error"),
            obs("Bash", "cargo test --release", "error"),
        ];
        let c = score_session("s1", "a1", &observations);
        assert!(c.bash_retry_violations > 0);
        assert!(c.warnings.iter().any(|w| w.contains("retry loop")));
    }

    #[test]
    fn bash_different_commands_no_violation() {
        let observations = vec![
            obs("Bash", "cargo test --release", "error"),
            obs("Bash", "cargo build --release", "error"),
            obs("Bash", "cargo clippy --release", "error"),
        ];
        let c = score_session("s1", "a1", &observations);
        assert_eq!(c.bash_retry_violations, 0);
    }

    #[test]
    fn files_modified_threshold() {
        let observations: Vec<_> = (0..8)
            .map(|i| obs("Edit", &format!("file{i}.rs"), "success"))
            .collect();
        let c = score_session("s1", "a1", &observations);
        assert_eq!(c.files_modified, 8);
        assert!(c.warnings.iter().any(|w| w.contains("distill")));
    }

    #[test]
    fn score_bounded_by_phi_inv() {
        let observations = vec![
            obs("Read", "foo.rs", "success"),
            obs("Edit", "foo.rs", "success"),
        ];
        let c = score_session("s1", "a1", &observations);
        assert!(c.score <= PHI_INV + 0.001);
    }

    #[test]
    fn mixed_session_realistic() {
        let observations = vec![
            obs("Read", "pipeline.rs", "success"),
            obs("Edit", "pipeline.rs", "success"),
            obs("Edit", "judge.rs", "success"), // no read
            obs("Bash", "cargo test", "error"),
            obs("Bash", "cargo test", "error"),
            obs("Bash", "cargo test", "error"), // 3 retries
            obs("Read", "main.rs", "success"),
            obs("Edit", "main.rs", "success"),
        ];
        let c = score_session("s1", "a1", &observations);
        // 2/3 edits had reads = 0.667
        assert!(c.read_before_edit > 0.6 && c.read_before_edit < 0.7);
        assert!(c.bash_retry_violations > 0);
        assert_eq!(c.files_modified, 3);
        // Degraded by bash retries but φ-capped. score < PHI_INV because
        // bash_score=0.5 pulls weighted avg down, but scope=1.0 and read=0.667
        // keep it high. Composite ~0.618 (φ-cap applies).
        assert!(c.score > 0.5 && c.score <= PHI_INV + 0.001);
    }
}
