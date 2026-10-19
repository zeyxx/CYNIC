//! HITL grade proposals: how the fixture grows without rotting.
//!
//! Anyone (human or LLM) may propose a labeled query, but only proposals
//! passing [`judge_grade_proposal`] enter the fixture — the same
//! data-centric gate pattern as `judge patch`, applied to grades. The gate
//! is purely structural (ids, grade values, trap discipline); semantic
//! truth (does the gold really answer the query) stays human — that is
//! what the minimal review UI (phase 2) presents, and what the epistemic
//! engine calibrates on in phase 3. Proposals carry their proposer so the
//! engine can later weight `llm:*` grades below `human` ones.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::retrieval_gain::{Fixture, LabeledQuery};

pub const GRADE_PROPOSAL_SCHEMA_V1: &str = "cynic.grade_proposal.v1";

/// A candidate labeled query. `proposer` is `human` or `llm:<model>` —
/// the epistemic seam: phase 3 weights them differently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeProposal {
    pub schema: String,
    pub id: String,
    pub query: String,
    #[serde(default)]
    pub trap: bool,
    #[serde(default)]
    pub grades: HashMap<String, u32>,
    pub proposer: String,
    #[serde(default)]
    pub rationale: String,
}

/// Deterministic gate verdict. `accept` false always comes with reasons.
#[derive(Debug, Clone, Serialize)]
pub struct GradeVerdict {
    pub id: String,
    pub accept: bool,
    pub reasons: Vec<String>,
}

fn valid_id(id: &str) -> bool {
    id.starts_with("RQ")
        && id.contains('_')
        && id.len() <= 48
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn valid_query(query: &str) -> bool {
    query.trim().len() >= 10 && query.split_whitespace().count() >= 2
}

/// Structural gate over one proposal. Reads the existing fixture only
/// for duplicate-id detection — no corpus, no network, no model.
pub fn judge_grade_proposal(proposal: &GradeProposal, fixture: &Fixture) -> GradeVerdict {
    let mut reasons = Vec::new();
    if proposal.schema != GRADE_PROPOSAL_SCHEMA_V1 {
        reasons.push(format!("unknown schema {}", proposal.schema));
    }
    if !valid_id(&proposal.id) {
        reasons.push("id must look like RQ9_TEMPORAL (RQ-prefix, underscore, ascii)".into());
    }
    if fixture.queries.iter().any(|q| q.id == proposal.id) {
        reasons.push("duplicate id already in fixture".into());
    }
    if !valid_query(&proposal.query) {
        reasons.push("query needs >= 10 chars and >= 2 tokens".into());
    }
    if proposal.proposer.trim().is_empty() {
        reasons.push("proposer must be human or llm:<model>".into());
    }
    if proposal.proposer.starts_with("llm:") && proposal.rationale.trim().is_empty() {
        reasons.push("llm proposals must carry a rationale for later calibration".into());
    }
    if proposal.trap {
        if !proposal.grades.is_empty() {
            reasons.push("trap queries must carry no grades".into());
        }
    } else {
        if proposal.grades.is_empty() {
            reasons.push("non-trap queries need at least one grade".into());
        }
        for (path, grade) in &proposal.grades {
            if !matches!(grade, 1 | 2) {
                reasons.push(format!("grade for {path} must be 1 or 2, got {grade}"));
            }
            if !path.contains('/') && !path.contains('.') {
                reasons.push(format!("grade path {path} does not look like a repo path"));
            }
        }
    }
    GradeVerdict {
        id: proposal.id.clone(),
        accept: reasons.is_empty(),
        reasons,
    }
}

impl Fixture {
    /// Inserts an accepted proposal's query. The caller must have run
    /// [`judge_grade_proposal`] first; the duplicate check runs again
    /// here so direct inserts stay safe.
    pub fn insert_labeled(&mut self, proposal: &GradeProposal) -> Result<(), String> {
        if self.queries.iter().any(|q| q.id == proposal.id) {
            return Err(format!("duplicate id {}", proposal.id));
        }
        self.queries.push(LabeledQuery {
            id: proposal.id.clone(),
            query: proposal.query.clone(),
            trap: proposal.trap,
            grades: proposal.grades.clone(),
        });
        Ok(())
    }

    /// Ids already taken — lets callers pre-check batches.
    pub fn known_ids(&self) -> HashSet<String> {
        self.queries.iter().map(|q| q.id.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        Fixture {
            schema: "cynic.retrieval_gain_labels.v1".into(),
            queries: vec![LabeledQuery {
                id: "RQ9_TEMPORAL".into(),
                query: "TemporalContract temporal reconciliation".into(),
                trap: false,
                grades: HashMap::from([("temporal.rs".into(), 2)]),
            }],
        }
    }

    fn good() -> GradeProposal {
        GradeProposal {
            schema: GRADE_PROPOSAL_SCHEMA_V1.into(),
            id: "RQ14_SMOKE".into(),
            query: "how are smoke runs gated".into(),
            trap: false,
            grades: HashMap::from([("crates/cynic-eval/src/lib.rs".into(), 2)]),
            proposer: "human".into(),
            rationale: String::new(),
        }
    }

    #[test]
    fn good_human_proposal_passes_and_inserts() {
        let mut fixture = fixture();
        let proposal = good();
        let verdict = judge_grade_proposal(&proposal, &fixture);
        assert!(verdict.accept, "{:?}", verdict.reasons);
        fixture.insert_labeled(&proposal).unwrap();
        assert!(fixture.known_ids().contains("RQ14_SMOKE"));
    }

    #[test]
    fn gate_rejects_every_structural_fault() {
        let fixture = fixture();
        let mut bad = good();
        bad.id = "RQ9_TEMPORAL".into();
        assert!(!judge_grade_proposal(&bad, &fixture).accept);

        let mut bad = good();
        bad.id = "nope".into();
        assert!(!judge_grade_proposal(&bad, &fixture).accept);

        let mut bad = good();
        bad.query = "x".into();
        assert!(!judge_grade_proposal(&bad, &fixture).accept);

        let mut bad = good();
        bad.grades.clear();
        assert!(!judge_grade_proposal(&bad, &fixture).accept);

        let mut bad = good();
        bad.grades = HashMap::from([("x/y.md".into(), 5)]);
        let verdict = judge_grade_proposal(&bad, &fixture);
        assert!(!verdict.accept);

        let mut bad = good();
        bad.trap = true;
        assert!(!judge_grade_proposal(&bad, &fixture).accept);

        let mut bad = good();
        bad.proposer = "llm:gemini-3.8-flash".into();
        let verdict = judge_grade_proposal(&bad, &fixture);
        assert!(!verdict.accept, "llm without rationale must fail");
        bad.rationale = "gold mentions the gate twice".into();
        assert!(judge_grade_proposal(&bad, &fixture).accept);
    }

    #[test]
    fn trap_with_no_grades_passes() {
        let fixture = fixture();
        let proposal = GradeProposal {
            schema: GRADE_PROPOSAL_SCHEMA_V1.into(),
            id: "RQ15_TRAP_PHONES".into(),
            query: "deploy android apk phones".into(),
            trap: true,
            grades: HashMap::new(),
            proposer: "human".into(),
            rationale: String::new(),
        };
        assert!(judge_grade_proposal(&proposal, &fixture).accept);
    }
}
