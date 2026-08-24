//! CYNIC Autoresearcher — sovereign skill-extraction research loop.
//!
//! Fidèle au pattern karpathy/autoresearch (master): a closed experiment loop
//! with (1) a single editable surface (`program.md`-equivalent spec), (2) a
//! single comparable metric (`skill_reuse_gain`), (3) a fixed per-iteration
//! budget, (4) keep/discard decided by the metric, (5) machine-readable
//! read-back proof. This module is the PURE protocol layer only: types, the
//! metric, and the dedup/keep decisions. No IO, no LLM call — adapters live in
//! `cynic-host::autoresearch` and the reflection LLM is behind a swappable
//! `ReflectionAdapter` (see design doc). This is R&D step 1 before recursive agents.
//!
//! `program.md` equivalent = `AutoresearchProgram` (the editable spec the
//! operator iterates). `prepare.py` equivalent = the neutral trace miner.
//! `evaluate_bpb` equivalent = `skill_reuse_gain`.

use serde::{Deserialize, Serialize};

pub const AUTORESEARCH_PROGRAM_SCHEMA: &str = "cynic.autoresearch.program.v1";
pub const AUTORESEARCH_HYPOTHESIS_SCHEMA: &str = "cynic.autoresearch.hypothesis.v1";
pub const AUTORESEARCH_CANDIDATE_SCHEMA: &str = "cynic.autoresearch.candidate.v1";
pub const AUTORESEARCH_REPORT_SCHEMA: &str = "cynic.autoresearch.report.v1";

/// The editable spec — equivalent of `train.py` in autoresearch. The operator
/// (human) iterates this; the loop reads it to know HOW to generate skills.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoresearchProgram {
    pub schema: String,
    /// Min coverage (fraction of corpus sessions with a real failure/block the
    /// skill addresses) for a candidate to be considered.
    pub min_coverage: f64,
    /// Keep threshold on the final gain metric.
    pub keep_gain_threshold: f64,
    /// Penalty subtracted when an existing skill already covers the domain
    /// (dedup). 1.0 = full duplicate, must not exist.
    pub dedup_penalty: f64,
    /// Penalty subtracted when the candidate is a markdown stub with no
    /// machine-actionable trigger (port/CLI/adapter). 0.5 default per design.
    pub stub_penalty: f64,
    /// Domains already owned by a skill — used by `dedup_against` (canonical
    /// decision source, structural-invariant #6). Declared, not hardcoded.
    pub owned_domains: Vec<String>,
}

impl Default for AutoresearchProgram {
    fn default() -> Self {
        Self {
            schema: AUTORESEARCH_PROGRAM_SCHEMA.to_string(),
            min_coverage: 0.02,
            keep_gain_threshold: 0.1,
            dedup_penalty: 1.0,
            stub_penalty: 0.5,
            owned_domains: vec![
                "toolchain_ice".into(),
                "cargo_build".into(),
                "rust".into(),
                "permission".into(),
                "oom".into(),
                "network".into(),
                "tailscale".into(),
                "render".into(),
                "cynic_brain".into(),
                "design_doc_plan_tests".into(),
            ],
        }
    }
}

/// One observed unit from the trace corpus (produced by the neutral miner).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceUnit {
    pub session_id: String,
    /// Domain this unit belongs to (matches an `owned_domains` entry or a new one).
    pub domain: String,
    /// True if the unit is a *real* failure/block, not a passing mention.
    pub is_block: bool,
}

/// A hypothesis produced by the reflection LLM adapter (or a fake in tests).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillHypothesis {
    pub schema: String,
    /// Candidate skill id, e.g. `cynic-something`.
    pub skill_id: String,
    /// Domain the skill claims to own (used for dedup).
    pub domain: String,
    /// Short description of the experiment (logged like results.tsv).
    pub description: String,
    /// Does this skill trigger a machine-actionable capability (port/CLI/adapter)?
    pub actionability: bool,
    /// How many corpus units this hypothesis claims to cover.
    pub claimed_coverage: f64,
}

impl SkillHypothesis {
    pub fn new(
        skill_id: impl Into<String>,
        domain: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            schema: AUTORESEARCH_HYPOTHESIS_SCHEMA.to_string(),
            skill_id: skill_id.into(),
            domain: domain.into(),
            description: description.into(),
            actionability: false,
            claimed_coverage: 0.0,
        }
    }
}

/// The comparable metric — equivalent of `val_bpb` (lower-is-better inverted:
/// higher gain = better). Pure function of measured coverage + penalties.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkillReuseGain {
    pub coverage: f64,
    pub dedup_penalty: f64,
    pub stub_penalty: f64,
    pub gain: f64,
}

/// Keep/discard decision (equivalent of commit/revert in autoresearch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeepDiscard {
    Keep,
    Discard,
    Unknown,
}

/// A candidate skill with its proof, ready to be sunk or reverted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillCandidate {
    pub schema: String,
    pub skill_id: String,
    pub domain: String,
    pub description: String,
    pub actionability: bool,
    pub gain: SkillReuseGain,
    pub decision: KeepDiscard,
    /// Content-addressed evidence refs (CausalEvidenceRefs-compatible digest).
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl SkillCandidate {
    pub fn digest(&self) -> String {
        crate::digest_json(self).unwrap_or_else(|_| "sha256:invalid".to_string())
    }
}

/// Machine-readable read-back proof — equivalent of `results.tsv`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoresearchReport {
    pub schema: String,
    pub program_digest: String,
    pub corpus_sessions: usize,
    pub hypotheses_evaluated: usize,
    pub kept: Vec<String>,
    pub discarded: Vec<String>,
    pub entries: Vec<AutoresearchReportEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoresearchReportEntry {
    pub skill_id: String,
    pub domain: String,
    pub gain: f64,
    pub status: KeepDiscard,
    pub description: String,
}

// ---------------------------------------------------------------------------
// PURE POLICY — no IO, no LLM. Tested below without any external dependency.
// ---------------------------------------------------------------------------

/// Measure coverage of a domain from observed units: fraction of sessions where
/// the domain appears as a *real block* (not a passing mention).
pub fn measure_coverage(units: &[TraceUnit], domain: &str) -> f64 {
    if units.is_empty() {
        return 0.0;
    }
    let total_sessions = units
        .iter()
        .map(|u| u.session_id.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    if total_sessions == 0 {
        return 0.0;
    }
    let blocked_sessions = units
        .iter()
        .filter(|u| u.domain == domain && u.is_block)
        .map(|u| u.session_id.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    blocked_sessions as f64 / total_sessions as f64
}

/// Canonical dedup decision (structural-invariant #6): one source of truth.
/// Returns the penalty (0.0 if the domain is not already owned) and whether the
/// domain collides with an existing skill.
pub fn dedup_against(program: &AutoresearchProgram, domain: &str) -> (f64, bool) {
    let collides = program
        .owned_domains
        .iter()
        .any(|d| d.eq_ignore_ascii_case(domain));
    if collides {
        (program.dedup_penalty, true)
    } else {
        (0.0, false)
    }
}

/// Compute the comparable gain metric (equivalent of val_bpb, higher = better).
pub fn skill_reuse_gain(
    program: &AutoresearchProgram,
    hypothesis: &SkillHypothesis,
    coverage: f64,
) -> SkillReuseGain {
    let (dedup_pen, _) = dedup_against(program, &hypothesis.domain);
    let stub_pen = if hypothesis.actionability {
        0.0
    } else {
        program.stub_penalty
    };
    let gain = coverage - dedup_pen - stub_pen;
    SkillReuseGain {
        coverage,
        dedup_penalty: dedup_pen,
        stub_penalty: stub_pen,
        gain,
    }
}

/// Decide keep/discard from the gain + program thresholds. Honest Unknown when
/// coverage is below the program minimum (unknown is data, not failure).
pub fn decide(program: &AutoresearchProgram, gain: &SkillReuseGain, coverage: f64) -> KeepDiscard {
    if coverage < program.min_coverage {
        return KeepDiscard::Unknown;
    }
    if gain.gain >= program.keep_gain_threshold {
        KeepDiscard::Keep
    } else {
        KeepDiscard::Discard
    }
}

/// Build a candidate from a hypothesis + observed units (pure assembly).
pub fn evaluate_candidate(
    program: &AutoresearchProgram,
    hypothesis: &SkillHypothesis,
    units: &[TraceUnit],
) -> SkillCandidate {
    let coverage = measure_coverage(units, &hypothesis.domain);
    let gain = skill_reuse_gain(program, hypothesis, coverage);
    let decision = decide(program, &gain, coverage);
    SkillCandidate {
        schema: AUTORESEARCH_CANDIDATE_SCHEMA.to_string(),
        skill_id: hypothesis.skill_id.clone(),
        domain: hypothesis.domain.clone(),
        description: hypothesis.description.clone(),
        actionability: hypothesis.actionability,
        gain,
        decision,
        evidence_refs: vec![],
    }
}

/// Assemble the machine-readable report from evaluated candidates.
pub fn build_report(
    program: &AutoresearchProgram,
    corpus_sessions: usize,
    candidates: &[SkillCandidate],
) -> AutoresearchReport {
    let program_digest =
        crate::digest_json(program).unwrap_or_else(|_| "sha256:invalid".to_string());
    let mut kept = Vec::new();
    let mut discarded = Vec::new();
    let mut entries = Vec::new();
    for c in candidates {
        entries.push(AutoresearchReportEntry {
            skill_id: c.skill_id.clone(),
            domain: c.domain.clone(),
            gain: c.gain.gain,
            status: c.decision,
            description: c.description.clone(),
        });
        match c.decision {
            KeepDiscard::Keep => kept.push(c.skill_id.clone()),
            KeepDiscard::Discard => discarded.push(c.skill_id.clone()),
            KeepDiscard::Unknown => {}
        }
    }
    AutoresearchReport {
        schema: AUTORESEARCH_REPORT_SCHEMA.to_string(),
        program_digest,
        corpus_sessions,
        hypotheses_evaluated: candidates.len(),
        kept,
        discarded,
        entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units() -> Vec<TraceUnit> {
        // 5 sessions; `toolchain_ice` blocks in 2 (coverage 0.4);
        // `novel_domain` blocks in 1 (coverage 0.2, >= min_coverage).
        vec![
            TraceUnit {
                session_id: "s1".into(),
                domain: "toolchain_ice".into(),
                is_block: true,
            },
            TraceUnit {
                session_id: "s2".into(),
                domain: "toolchain_ice".into(),
                is_block: true,
            },
            TraceUnit {
                session_id: "s3".into(),
                domain: "tailscale".into(),
                is_block: true,
            },
            TraceUnit {
                session_id: "s4".into(),
                domain: "design_doc".into(),
                is_block: false,
            },
            TraceUnit {
                session_id: "s5".into(),
                domain: "novel_domain".into(),
                is_block: true,
            },
        ]
    }

    #[test]
    fn measure_coverage_counts_real_blocks_only() {
        let u = units();
        // 2 of 5 sessions blocked by toolchain_ice -> 0.4
        assert!((measure_coverage(&u, "toolchain_ice") - 0.4).abs() < 1e-9);
        // passing mention (is_block=false) must NOT count
        assert!((measure_coverage(&u, "design_doc") - 0.0).abs() < 1e-9);
        // empty corpus -> 0.0, no panic
        assert_eq!(measure_coverage(&[], "x"), 0.0);
    }

    #[test]
    fn dedup_detects_owned_domain() {
        let p = AutoresearchProgram::default();
        let (pen, collides) = dedup_against(&p, "toolchain_ice");
        assert!(collides);
        assert!((pen - p.dedup_penalty).abs() < 1e-9);
        let (pen2, collides2) = dedup_against(&p, "novel_domain");
        assert!(!collides2);
        assert_eq!(pen2, 0.0);
    }

    #[test]
    fn gain_rewards_actionability_and_penalizes_dup() {
        let p = AutoresearchProgram::default();
        // duplicate + stub: coverage 0.5 - 1.0 - 0.5 = -1.0
        let h_dup = SkillHypothesis::new("cynic-x", "toolchain_ice", "dup stub");
        let g_dup = skill_reuse_gain(&p, &h_dup, 0.5);
        assert!((g_dup.gain + 1.0).abs() < 1e-9);
        // novel + actionable: 0.5 - 0 - 0 = 0.5
        let mut h_new = SkillHypothesis::new("cynic-y", "novel_domain", "new actionable");
        h_new.actionability = true;
        let g_new = skill_reuse_gain(&p, &h_new, 0.5);
        assert!((g_new.gain - 0.5).abs() < 1e-9);
    }

    #[test]
    fn decide_keeps_actionable_novel_above_threshold() {
        let p = AutoresearchProgram::default();
        let mut h = SkillHypothesis::new("cynic-y", "novel_domain", "new");
        h.actionability = true;
        let g = skill_reuse_gain(&p, &h, 0.5); // gain 0.5 >= 0.1
        assert_eq!(decide(&p, &g, 0.5), KeepDiscard::Keep);
        // below min coverage -> Unknown (honest, not Discard)
        let g_low = skill_reuse_gain(&p, &h, 0.01);
        assert_eq!(decide(&p, &g_low, 0.01), KeepDiscard::Unknown);
    }

    #[test]
    fn evaluate_candidate_assembles_decision() {
        let p = AutoresearchProgram::default();
        // toolchain_ice is owned -> dedup penalty -> Discard even with coverage
        let h = SkillHypothesis::new("cynic-x", "toolchain_ice", "dup");
        let c = evaluate_candidate(&p, &h, &units());
        assert_eq!(c.decision, KeepDiscard::Discard);
        assert!((c.gain.coverage - 0.4).abs() < 1e-9);
    }

    #[test]
    fn build_report_machine_readable_and_deterministic() {
        let p = AutoresearchProgram::default();
        let mut h_new = SkillHypothesis::new("cynic-y", "novel_domain", "new actionable");
        h_new.actionability = true;
        let cands = vec![
            evaluate_candidate(
                &p,
                &SkillHypothesis::new("cynic-x", "toolchain_ice", "dup"),
                &units(),
            ),
            evaluate_candidate(&p, &h_new, &units()),
        ];
        let r = build_report(&p, 5, &cands);
        assert_eq!(r.corpus_sessions, 5);
        assert_eq!(r.hypotheses_evaluated, 2);
        assert_eq!(r.kept, vec!["cynic-y".to_string()]);
        assert_eq!(r.discarded, vec!["cynic-x".to_string()]);
        assert!(r.program_digest.starts_with("sha256:"));
        // determinism: same input -> same digest
        let r2 = build_report(&p, 4, &cands);
        assert_eq!(r.program_digest, r2.program_digest);
    }

    #[test]
    fn schema_names_are_stable_contracts() {
        assert_eq!(AUTORESEARCH_PROGRAM_SCHEMA, "cynic.autoresearch.program.v1");
        assert_eq!(
            AUTORESEARCH_HYPOTHESIS_SCHEMA,
            "cynic.autoresearch.hypothesis.v1"
        );
        assert_eq!(
            AUTORESEARCH_CANDIDATE_SCHEMA,
            "cynic.autoresearch.candidate.v1"
        );
        assert_eq!(AUTORESEARCH_REPORT_SCHEMA, "cynic.autoresearch.report.v1");
    }
}
