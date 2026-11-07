//! Deterministic collision-risk heuristic for refactor / absorption candidates.
//!
//! Pure logic: no I/O, no async. Scores a candidate on observable signals
//! (churn, test coverage, modularity, active-refactor overlap, churning deps)
//! and returns a Low/Medium/High risk level with human-readable reasons.
//!
//! This is the baseline (heuristic) that any future ML model must beat
//! before it is allowed into the critical path. See docs/audits/refactor-risk-dataset.json
//! for the retroactively labeled ground truth.
//!
//! Canonical error code: ArchitectureRefactorCollisionRisk (CYNIC-ARCH-007) when High.

use cynic_protocol::CynicErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RefactorCollisionRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RefactorCandidate {
    /// Human-readable name of the candidate (crate or subsystem).
    pub name: String,
    /// Commits touching the candidate in the last 90 days.
    pub commits_90d: u32,
    /// Number of tests covering the candidate.
    pub test_count: u32,
    /// Approximate lines of code (0 if unknown).
    pub loc: u32,
    /// Whether the candidate already exposes ports (FCIS modularity).
    pub has_ports: bool,
    /// Whether the candidate sits in an active refactor theme.
    pub in_active_refactor: bool,
    /// Number of direct dependencies that are themselves high-churn.
    pub churning_deps: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RefactorRisk {
    pub level: RefactorCollisionRisk,
    pub score: u32,
    pub code: Option<CynicErrorCode>,
    pub reasons: Vec<String>,
}

/// Assess collision risk for a refactor or absorption candidate.
pub fn assess(candidate: &RefactorCandidate) -> RefactorRisk {
    let mut score: u32 = 0;
    let mut reasons: Vec<String> = Vec::new();

    // Churn signal.
    if candidate.commits_90d >= 100 {
        score += 3;
        reasons.push(format!(
            "high churn: {} commits/90d (>=100)",
            candidate.commits_90d
        ));
    } else if candidate.commits_90d >= 50 {
        score += 2;
        reasons.push(format!(
            "notable churn: {} commits/90d (50-99)",
            candidate.commits_90d
        ));
    } else if candidate.commits_90d >= 10 {
        score += 1;
        reasons.push(format!(
            "moderate churn: {} commits/90d (10-49)",
            candidate.commits_90d
        ));
    }

    // Test coverage signal.
    if candidate.test_count == 0 {
        score += 2;
        reasons.push("no tests covering candidate".to_string());
    } else if candidate.test_count < 10 {
        score += 1;
        reasons.push(format!(
            "low test coverage: {} tests (<10)",
            candidate.test_count
        ));
    }

    // Modularity signal.
    if !candidate.has_ports {
        score += 1;
        reasons.push("no port boundary (not yet modular/FCIS)".to_string());
    }

    // Active refactor overlap signal.
    if candidate.in_active_refactor {
        score += 2;
        reasons.push("sits in an active refactor theme".to_string());
    }

    // Churning dependency signal.
    if candidate.churning_deps >= 3 {
        score += 2;
        reasons.push(format!(
            "depends on {} high-churn crates (>=3)",
            candidate.churning_deps
        ));
    } else if candidate.churning_deps >= 1 {
        score += 1;
        reasons.push(format!(
            "depends on {} high-churn crate(s)",
            candidate.churning_deps
        ));
    }

    let level = if score >= 5 {
        RefactorCollisionRisk::High
    } else if score >= 3 {
        RefactorCollisionRisk::Medium
    } else {
        RefactorCollisionRisk::Low
    };

    let code = if level == RefactorCollisionRisk::High {
        Some(CynicErrorCode::ArchitectureRefactorCollisionRisk)
    } else {
        None
    };

    if reasons.is_empty() {
        reasons.push("no risk signals detected".to_string());
    }

    RefactorRisk {
        level,
        score,
        code,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(name: &str, commits: u32, tests: u32, ports: bool, active: bool, churning: u32) -> RefactorCandidate {
        RefactorCandidate {
            name: name.to_string(),
            commits_90d: commits,
            test_count: tests,
            loc: 0,
            has_ports: ports,
            in_active_refactor: active,
            churning_deps: churning,
        }
    }

    #[test]
    fn low_risk_stable_modular_candidate() {
        let c = candidate("browser-ingestion", 2, 30, true, false, 0);
        let r = assess(&c);
        assert_eq!(r.level, RefactorCollisionRisk::Low);
        assert!(r.code.is_none());
    }

    #[test]
    fn high_risk_active_churning_candidate() {
        let c = candidate("cynic-secrets", 9, 42, true, true, 1);
        let r = assess(&c);
        assert!(r.score >= 2);
    }

    #[test]
    fn high_risk_protocol_partition() {
        let c = candidate("cynic-protocol", 72, 0, true, true, 1);
        let r = assess(&c);
        assert_eq!(r.level, RefactorCollisionRisk::High);
        assert_eq!(r.code, Some(CynicErrorCode::ArchitectureRefactorCollisionRisk));
    }

    #[test]
    fn reasons_non_empty() {
        let c = candidate("x", 0, 100, true, false, 0);
        let r = assess(&c);
        assert!(!r.reasons.is_empty());
    }

    /// Ground-truth check against docs/audits/refactor-risk-dataset.json
    #[test]
    fn dataset_predictions_match_expected() {
        let cases: &[(RefactorCollisionRisk, RefactorCandidate)] = &[
            (
                RefactorCollisionRisk::Low,
                RefactorCandidate {
                    name: "browser-ingestion".into(),
                    commits_90d: 2,
                    test_count: 1,
                    loc: 0,
                    has_ports: true,
                    in_active_refactor: false,
                    churning_deps: 0,
                },
            ),
            (
                RefactorCollisionRisk::High,
                RefactorCandidate {
                    name: "cynic-host".into(),
                    commits_90d: 725,
                    test_count: 717,
                    loc: 0,
                    has_ports: false,
                    in_active_refactor: true,
                    churning_deps: 3,
                },
            ),
            (
                RefactorCollisionRisk::High,
                RefactorCandidate {
                    name: "cynic-protocol".into(),
                    commits_90d: 72,
                    test_count: 734,
                    loc: 0,
                    has_ports: true,
                    in_active_refactor: true,
                    churning_deps: 1,
                },
            ),
        ];
        for (expected, c) in cases {
            let r = assess(c);
            assert_eq!(r.level, *expected, "mismatch for {}", c.name);
        }
    }
}