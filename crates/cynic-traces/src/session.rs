//! The NormalizedSession aggregate.
//!
//! Strict port of `apps/cynic-data/src/cynic_data/canonical/domain/session.py`.
//! One harness session projected into canonical entities. Invariants are
//! enforced here, not in the store and not in a script.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use super::domain::{
    CorrectionKind, Edge, EdgeProvenance, HumanSignal, ReasoningTrace, StateSnapshot, ToolCall,
    Turn,
};

/// Canonical invariant breach. Fail-closed: the session is rejected, never repaired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractViolation(pub String);

impl fmt::Display for ContractViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "contract violation: {}", self.0)
    }
}

impl std::error::Error for ContractViolation {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedSession {
    pub session_uid: String,
    pub harness: String,
    pub source_ref: String,
    pub turns: Vec<Turn>,
    pub tools: Vec<ToolCall>,
    pub reasoning: Vec<ReasoningTrace>,
    pub edges: Vec<Edge>,
    pub states: Vec<StateSnapshot>,
    pub human: Vec<HumanSignal>,
    pub native_meta: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStats {
    pub turns: usize,
    pub tools: usize,
    pub reasoning: usize,
    pub edges: usize,
    pub native_edges: usize,
    pub states: usize,
    pub human: usize,
    pub corrections: usize,
}

impl NormalizedSession {
    /// Enforce the canonical invariants. Returns the breach, never repairs it.
    pub fn check(&self) -> Result<(), ContractViolation> {
        let turn_ids: HashSet<&str> = self.turns.iter().map(|t| t.turn_uid.as_str()).collect();
        if turn_ids.len() != self.turns.len() {
            return Err(ContractViolation(format!(
                "{}: duplicate turn_uid",
                self.session_uid
            )));
        }
        for t in &self.turns {
            if t.session_uid != self.session_uid {
                return Err(ContractViolation(format!(
                    "{}: turn {} carries foreign session_uid",
                    self.session_uid, t.turn_uid
                )));
            }
            if t.role.is_empty() {
                return Err(ContractViolation(format!(
                    "{}: turn {} has empty role",
                    self.session_uid, t.turn_uid
                )));
            }
        }
        for e in &self.edges {
            if e.session_uid != self.session_uid {
                return Err(ContractViolation(format!(
                    "{}: edge {} carries foreign session_uid",
                    self.session_uid, e.edge_uid
                )));
            }
            if e.source_uids.is_empty() {
                return Err(ContractViolation(format!(
                    "{}: edge {} has no source",
                    self.session_uid, e.edge_uid
                )));
            }
            if let Some(target) = &e.target_uid {
                if !turn_ids.contains(target.as_str()) {
                    return Err(ContractViolation(format!(
                        "{}: edge {} targets unknown turn {target}",
                        self.session_uid, e.edge_uid
                    )));
                }
            }
        }
        for h in &self.human {
            if !turn_ids.contains(h.turn_uid.as_str()) {
                return Err(ContractViolation(format!(
                    "{}: human signal {} references unknown turn",
                    self.session_uid, h.human_uid
                )));
            }
        }
        for s in &self.states {
            if let Some(turn) = &s.turn_uid {
                if !turn_ids.contains(turn.as_str()) {
                    return Err(ContractViolation(format!(
                        "{}: state {} references unknown turn",
                        self.session_uid, s.snapshot_uid
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn stats(&self) -> SessionStats {
        SessionStats {
            turns: self.turns.len(),
            tools: self.tools.len(),
            reasoning: self.reasoning.len(),
            edges: self.edges.len(),
            native_edges: self
                .edges
                .iter()
                .filter(|e| e.provenance == EdgeProvenance::Native)
                .count(),
            states: self.states.len(),
            human: self.human.len(),
            corrections: self
                .human
                .iter()
                .filter(|h| h.kind != CorrectionKind::Unknown)
                .count(),
        }
    }
}
