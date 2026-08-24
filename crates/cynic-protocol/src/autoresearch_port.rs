//! CYNIC Autoresearcher — port + reflection adapter (Phase 1).
//!
//! `AutoresearchPort` is the PURE contract for the experiment loop. Observation
//! and hypothesis generation are behind this trait so the loop, the policy, and
//! the keep/discard decision (all in `autoresearch.rs`) stay testable without
//! any LLM, filesystem, or network. The reflection LLM is an adapter: a FAKE in
//! tests (deterministic hypotheses), the real local-first backend in
//! `cynic-host::autoresearch` (Phase 3). This honors the autoresearch pattern:
//! the editable spec (`AutoresearchProgram`) + a single metric (`skill_reuse_gain`)
//! + keep/discard, with the "researcher" (reflection) fully swappable.

use serde::{Deserialize, Serialize};

use crate::autoresearch::{
    build_report, evaluate_candidate, gate_hypothesis, AutoresearchProgram, AutoresearchReport,
    EmergenceDirective, GateVerdict, SkillCandidate, SkillHypothesis, TraceUnit,
};

pub const AUTORESEARCH_PORT_ERROR_SCHEMA: &str = "cynic.autoresearch.port_error.v1";

/// Stable error for the loop boundary. Contains no secrets, paths, or provider
/// command output — only a code and contextual evidence refs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoresearchPortError {
    pub schema: String,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl AutoresearchPortError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            schema: AUTORESEARCH_PORT_ERROR_SCHEMA.to_string(),
            code: code.into(),
            message: message.into(),
            evidence_refs: Vec::new(),
        }
    }
}

/// The pure research-loop contract. No IO, no LLM. Implementors: `FakeReflection`
/// (tests), `LocalReflectionAdapter` (`cynic-host`, Phase 3).
pub trait AutoresearchPort {
    /// Observe the corpus: produce typed units (domain + is_block). Streaming in
    /// the real adapter; the fake returns a fixed set.
    fn observe_traces(&self) -> Result<Vec<TraceUnit>, AutoresearchPortError>;

    /// Reflect on the units to form skill hypotheses (the "researcher"). The fake
    /// returns pre-set hypotheses so decisions are deterministic and testable.
    fn hypothesize(
        &self,
        units: &[TraceUnit],
    ) -> Result<Vec<SkillHypothesis>, AutoresearchPortError>;
}

pub type ReasoningError = AutoresearchPortError;

#[async_trait::async_trait]
pub trait ReasoningAdapter: Send + Sync {
    async fn hypothesize(&self, evidence: &str) -> Result<Vec<SkillHypothesis>, ReasoningError>;
}

/// Run one full experiment loop over the corpus and return a machine-readable
/// report. PURE with respect to `port`/`program`: it only calls the trait and
/// the policy in `autoresearch.rs`. This is the equivalent of autoresearch's
/// "advance the branch if val_bpb improved" — but here the keep/discard is the
/// metric decision, and no git mutation happens (CYNIC requires --yes upstream).
pub fn run_loop(
    program: &AutoresearchProgram,
    port: &dyn AutoresearchPort,
) -> Result<AutoresearchReport, AutoresearchPortError> {
    let units = port.observe_traces()?;
    let hypotheses = port.hypothesize(&units)?;
    let corpus_sessions = units
        .iter()
        .map(|u| u.session_id.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let candidates: Vec<SkillCandidate> = hypotheses
        .iter()
        .map(|h| evaluate_candidate(program, h, &units))
        .collect();
    Ok(build_report(program, corpus_sessions, &candidates))
}

/// Run one full experiment loop with pre-computed units and hypotheses.
/// Used when the caller has already observed traces and generated hypotheses
/// (for example via separate adapters, or with offline reflection).
pub fn run_loop_with_hypotheses(
    program: &AutoresearchProgram,
    units: &[TraceUnit],
    hypotheses: &[SkillHypothesis],
) -> AutoresearchReport {
    let corpus_sessions = units
        .iter()
        .map(|u| u.session_id.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let candidates: Vec<SkillCandidate> = hypotheses
        .iter()
        .map(|h| evaluate_candidate(program, h, units))
        .collect();
    build_report(program, corpus_sessions, &candidates)
}

/// Gated loop: emergence directives decide BEFORE any metric work.
///
/// `directives` maps a domain to the temporal directive derived from the
/// pattern GC classes (see `autoresearch::emergence_directive`). Hypotheses
/// whose domain carries `Consolidate` or `NoSignal` are skipped with an
/// honest machine-readable reason and never reach evaluation — a collapsing
/// failure class cannot spawn new skills. Domains absent from the map are
/// treated as `NoSignal` (no temporal claim is invented).
pub fn run_loop_gated(
    program: &AutoresearchProgram,
    port: &dyn AutoresearchPort,
    directives: &std::collections::BTreeMap<String, EmergenceDirective>,
) -> Result<GatedLoopOutcome, AutoresearchPortError> {
    let units = port.observe_traces()?;
    let hypotheses = port.hypothesize(&units)?;
    let corpus_sessions = units
        .iter()
        .map(|u| u.session_id.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();

    let mut candidates = Vec::new();
    let mut gated_out = Vec::new();
    for hypothesis in &hypotheses {
        let directive = directives
            .get(&hypothesis.domain)
            .copied()
            .unwrap_or(EmergenceDirective::NoSignal);
        match gate_hypothesis(directive) {
            GateVerdict::Evaluate => {
                candidates.push(evaluate_candidate(program, hypothesis, &units));
            }
            GateVerdict::Skip { reason } => gated_out.push(GatedOutEntry {
                skill_id: hypothesis.skill_id.clone(),
                domain: hypothesis.domain.clone(),
                reason,
            }),
        }
    }

    Ok(GatedLoopOutcome {
        report: build_report(program, corpus_sessions, &candidates),
        gated_out,
    })
}

/// One hypothesis blocked by the emergence gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatedOutEntry {
    pub skill_id: String,
    pub domain: String,
    pub reason: String,
}

/// Report plus gate trace (machine-readable read-back of what was excluded).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatedLoopOutcome {
    /// Classic v1 report over the hypotheses that passed the gate.
    pub report: AutoresearchReport,
    /// Hypotheses excluded before evaluation, with reasons.
    pub gated_out: Vec<GatedOutEntry>,
}

/// Deterministic fake reflection adapter for tests. Holds fixed observations and
/// hypotheses so the loop + policy can be proven without any LLM or corpus file.
pub struct FakeReflection {
    pub units: Vec<TraceUnit>,
    pub hypotheses: Vec<SkillHypothesis>,
}

impl FakeReflection {
    pub fn new(units: Vec<TraceUnit>, hypotheses: Vec<SkillHypothesis>) -> Self {
        Self { units, hypotheses }
    }
}

impl AutoresearchPort for FakeReflection {
    fn observe_traces(&self) -> Result<Vec<TraceUnit>, AutoresearchPortError> {
        Ok(self.units.clone())
    }

    fn hypothesize(
        &self,
        _units: &[TraceUnit],
    ) -> Result<Vec<SkillHypothesis>, AutoresearchPortError> {
        Ok(self.hypotheses.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autoresearch::{AutoresearchProgram, KeepDiscard, SkillHypothesis};

    fn fake_port() -> FakeReflection {
        // 5 sessions; toolchain_ice blocks in 2 (owned domain -> discard),
        // novel_domain blocks in 1 (actionable -> keep).
        let units = vec![
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
        ];
        let mut h_dup = SkillHypothesis::new("cynic-x", "toolchain_ice", "duplicate stub");
        let mut h_new = SkillHypothesis::new("cynic-y", "novel_domain", "new actionable skill");
        h_new.actionability = true;
        FakeReflection::new(units, vec![h_dup, h_new])
    }

    #[test]
    fn run_loop_keeps_actionable_novel_discards_owned() {
        let p = AutoresearchProgram::default();
        let report = run_loop(&p, &fake_port()).expect("loop must run");
        assert_eq!(report.corpus_sessions, 5);
        assert_eq!(report.hypotheses_evaluated, 2);
        assert_eq!(report.kept, vec!["cynic-y".to_string()]);
        assert_eq!(report.discarded, vec!["cynic-x".to_string()]);
        // entry for the kept skill carries Keep
        let kept = report
            .entries
            .iter()
            .find(|e| e.skill_id == "cynic-y")
            .unwrap();
        assert_eq!(kept.status, KeepDiscard::Keep);
        let dropped = report
            .entries
            .iter()
            .find(|e| e.skill_id == "cynic-x")
            .unwrap();
        assert_eq!(dropped.status, KeepDiscard::Discard);
    }

    #[test]
    fn run_loop_is_deterministic_and_digestable() {
        let p = AutoresearchProgram::default();
        let r1 = run_loop(&p, &fake_port()).unwrap();
        let r2 = run_loop(&p, &fake_port()).unwrap();
        assert_eq!(r1.program_digest, r2.program_digest);
        assert_eq!(r1.kept, r2.kept);
        assert!(r1.program_digest.starts_with("sha256:"));
    }

    #[test]
    fn port_error_is_structured_not_panic() {
        struct Broken;
        impl AutoresearchPort for Broken {
            fn observe_traces(&self) -> Result<Vec<TraceUnit>, AutoresearchPortError> {
                Err(AutoresearchPortError::new("corpus_unreadable", "disk gone"))
            }
            fn hypothesize(
                &self,
                _u: &[TraceUnit],
            ) -> Result<Vec<SkillHypothesis>, AutoresearchPortError> {
                Ok(vec![])
            }
        }
        let p = AutoresearchProgram::default();
        let err = run_loop(&p, &Broken {}).unwrap_err();
        assert_eq!(err.code, "corpus_unreadable");
        assert_eq!(err.schema, AUTORESEARCH_PORT_ERROR_SCHEMA);
    }

    #[test]
    fn schema_name_is_stable_contract() {
        assert_eq!(
            AUTORESEARCH_PORT_ERROR_SCHEMA,
            "cynic.autoresearch.port_error.v1"
        );
    }

    #[test]
    fn gated_loop_skips_fading_and_unknown_domains_before_evaluation() {
        use crate::autoresearch::{emergence_directive, EmergenceDirective, FadingPolicy};
        use crate::pattern_gc::PatternClass;
        use std::collections::BTreeMap;

        let p = AutoresearchProgram::default();
        // Directives: toolchain_ice is Fading (consolidate), tailscale is
        // Emerging (prioritize), novel_domain has NO temporal entry.
        let mut directives = BTreeMap::new();
        directives.insert(
            "toolchain_ice".to_string(),
            emergence_directive(PatternClass::Fading, &FadingPolicy::default()),
        );
        directives.insert(
            "tailscale".to_string(),
            emergence_directive(PatternClass::Emerging, &FadingPolicy::default()),
        );

        let outcome = run_loop_gated(&p, &fake_port(), &directives).unwrap();

        // cynic-x (toolchain_ice) must be gated out with a reason.
        assert_eq!(outcome.gated_out.len(), 2);
        let x = outcome
            .gated_out
            .iter()
            .find(|g| g.skill_id == "cynic-x")
            .expect("fading domain hypothesis must be gated");
        assert!(x.reason.contains("consolidate"));
        // novel_domain absent from the map -> NoSignal -> gated too (honest:
        // no temporal claim invented).
        assert!(outcome.gated_out.iter().any(|g| g.domain == "novel_domain"));
        // The report only carries evaluated candidates: none passed here
        // because tailscale has no hypothesis in the fixture.
        assert_eq!(outcome.report.hypotheses_evaluated, 0);
    }
}
