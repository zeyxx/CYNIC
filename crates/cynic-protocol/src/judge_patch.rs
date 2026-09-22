//! Judge patch — pure decision from extracted diff vs current codebase.
//!
//! Complements `judgment::CynicJudgment` with a domain-specific producer:
//! given an extracted patch (diff hash, size, ahead/behind), produce a
//! calibrated judgment. All I/O is outside — this module is pure.

use serde::{Deserialize, Serialize};

use crate::judgment::{CynefinDomain, CynicJudgment, DijkwLayer, SagePhase};

pub const JUDGE_PATCH_SCHEMA: &str = "cynic.judge_patch.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchObservation {
    pub schema: String,
    pub branch: String,
    pub base_commit: String,
    #[serde(default)]
    pub diff_hash: String,
    pub diff_bytes_len: usize,
    #[serde(default)]
    pub ahead: u32,
    #[serde(default)]
    pub behind: u32,
    #[serde(default)]
    pub staged_files: Vec<String>,
    pub observed_at_unix_s: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchJudgmentInput {
    pub observation: PatchObservation,
    pub judged_at_unix_s: u64,
}

/// Pure patch judge — no I/O, no LLM. Calibration is explicit and contestable.
pub fn judge_patch(input: &PatchJudgmentInput) -> CynicJudgment {
    let obs = &input.observation;
    let mut unknowns = Vec::new();
    let mut confidence: f64 = 0.7;
    let mut domain = CynefinDomain::Complicated;
    let mut layer = DijkwLayer::Knowledge;
    let mut phase = SagePhase::Evaluate;

    // Empty diff is disordered — nothing to judge.
    if obs.diff_bytes_len == 0 || obs.diff_hash.is_empty() {
        unknowns.push("empty_diff".to_string());
        confidence = 0.0;
        domain = CynefinDomain::Disordered;
        layer = DijkwLayer::Data;
        phase = SagePhase::Sense;
    } else {
        // Large ahead => split hygiene risk
        if obs.ahead >= 20 {
            unknowns.push(format!("large_ahead:{}", obs.ahead));
            confidence *= 0.6;
            domain = CynefinDomain::Complex;
        }
        // Far behind => stale base risk
        if obs.behind >= 50 {
            unknowns.push(format!("stale_base:{}", obs.behind));
            confidence *= 0.7;
            if domain != CynefinDomain::Complex {
                domain = CynefinDomain::Complicated;
            }
        }
        // Large diff bytes => review cost
        if obs.diff_bytes_len > 50_000 {
            unknowns.push(format!("large_diff:{}", obs.diff_bytes_len));
            confidence *= 0.7;
            domain = CynefinDomain::Complex;
        }
        // Staged leftover on source branch suggests dirty extract
        if !obs.staged_files.is_empty() {
            unknowns.push("staged_on_source".to_string());
            confidence *= 0.8;
        }
    }

    // Wisdom never 1.0; actionable only if not disordered and confidence enough
    let actionable = !matches!(domain, CynefinDomain::Disordered | CynefinDomain::Unknown)
        && confidence >= 0.5
        && unknowns.len() <= 2;

    if confidence > 0.99 {
        confidence = 0.99;
    }

    let mut j = CynicJudgment::new(
        format!(
            "judge-patch:{}:{}",
            obs.branch,
            obs.diff_hash.chars().take(8).collect::<String>()
        ),
        actionable,
        layer,
        phase,
        domain,
        input.judged_at_unix_s,
    );
    j.confidence = confidence;
    j.residual_unknowns = unknowns;
    // observation_refs would carry content-addressed diff hash in production; keep minimal for now
    j
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(diff_len: usize, ahead: u32, behind: u32) -> PatchJudgmentInput {
        PatchJudgmentInput {
            observation: PatchObservation {
                schema: JUDGE_PATCH_SCHEMA.to_string(),
                branch: "agent/hermes/x".to_string(),
                base_commit: "abc".to_string(),
                diff_hash: if diff_len == 0 {
                    "".to_string()
                } else {
                    "deadbeefcafe".to_string()
                },
                diff_bytes_len: diff_len,
                ahead,
                behind,
                staged_files: vec![],
                observed_at_unix_s: 1,
            },
            judged_at_unix_s: 2,
        }
    }

    #[test]
    fn empty_diff_is_disordered_not_actionable() {
        let j = judge_patch(&obs(0, 1, 0));
        assert_eq!(j.cynefin_domain, CynefinDomain::Disordered);
        assert!(!j.is_actionable());
    }

    #[test]
    fn small_patch_is_actionable() {
        let j = judge_patch(&obs(1000, 2, 5));
        assert!(j.is_actionable());
        assert!(j.confidence >= 0.5);
    }

    #[test]
    fn large_ahead_reduces_confidence_and_becomes_complex() {
        let j = judge_patch(&obs(1000, 42, 10));
        assert_eq!(j.cynefin_domain, CynefinDomain::Complex);
        assert!(j.confidence < 0.5 || !j.is_actionable());
    }

    #[test]
    fn stale_behind_adds_unknown() {
        let j = judge_patch(&obs(1000, 2, 60));
        assert!(j.residual_unknowns.iter().any(|u| u.contains("stale_base")));
    }
}
