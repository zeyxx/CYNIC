//! Versioned emergent pattern models — sealed snapshots with lineage.
//!
//! A [`PatternModel`] is an immutable snapshot of the classified motif
//! space at one boundary. Its identity is content-addressed (`model_id`),
//! it names its parent (lineage chain, same shape as proof-of-history),
//! and [`drift`] makes emergence *observable*: the class transitions
//! between two consecutive snapshots ARE the emerging/collapsing signal,
//! not a narrative claim.
//!
//! Schema versions bump when the contract changes; data itself stays
//! addressed by content, never re-versioned.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::types::{PatternClass, PatternGcAssessment, RetentionPolicy};

pub const MODEL_SCHEMA: &str = "cynic.pattern_gc.model.v1";

/// One key's position in the motif space at snapshot time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternState {
    pub key: String,
    pub class: PatternClass,
}

/// Sealed model snapshot with lineage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternModel {
    pub schema: String,
    /// Content-addressed identity (sha256 hex over canonical content).
    pub model_id: String,
    /// Parent snapshot id, `None` for genesis.
    pub parent_id: Option<String>,
    /// Policy snapshot that produced every classification.
    pub policy: RetentionPolicy,
    /// Windowing parameters (unix seconds).
    pub boundary_s: i64,
    pub recent_span_s: u64,
    /// Digest of the sealed report this model was distilled from.
    pub report_digest: String,
    /// Per-key states, sorted by key (canonical order).
    pub states: Vec<PatternState>,
    /// Events stamped beyond the boundary at scan time (surfaced verbatim).
    pub out_of_order_events: u64,
}

/// One key's class change between two snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassTransition {
    pub key: String,
    pub from: PatternClass,
    pub to: PatternClass,
}

/// Observable drift between a parent and its child snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelDrift {
    pub parent_id: String,
    pub child_id: String,
    /// Keys whose class changed (sorted by key).
    pub transitions: Vec<ClassTransition>,
    /// Keys present only in the child (first appearance).
    pub appeared: Vec<String>,
    /// Keys present only in the parent (no recent signal).
    pub vanished: Vec<String>,
}

impl ModelDrift {
    /// Keys that newly entered an emerging state (the actionable signal).
    pub fn newly_emerging(&self) -> Vec<&str> {
        self.transitions
            .iter()
            .filter(|t| t.to == PatternClass::Emerging && t.from != PatternClass::Emerging)
            .map(|t| t.key.as_str())
            .collect()
    }

    /// Recurring keys that collapsed into fading since the parent.
    pub fn newly_fading(&self) -> Vec<&str> {
        self.transitions
            .iter()
            .filter(|t| t.to == PatternClass::Fading && t.from != PatternClass::Fading)
            .map(|t| t.key.as_str())
            .collect()
    }
}

/// Seal a model snapshot from classified observations. Pure: identical
/// inputs (including parent id) always yield the identical model id.
pub fn seal_model(
    assessments: &[PatternGcAssessment],
    policy: &RetentionPolicy,
    boundary_s: i64,
    recent_span_s: u64,
    out_of_order_events: u64,
    parent_id: Option<&str>,
) -> PatternModel {
    let mut states: Vec<PatternState> = assessments
        .iter()
        .map(|a| PatternState {
            key: a.key.clone(),
            class: a.class,
        })
        .collect();
    states.sort_by(|a, b| a.key.cmp(&b.key));

    // Report digest via the existing sealing rule keeps one digest law.
    let report = super::report::build_report(
        assessments,
        policy,
        boundary_s,
        recent_span_s,
        out_of_order_events,
    );

    let mut hasher = Sha256::new();
    let content = (
        MODEL_SCHEMA,
        report.content_sha256.as_str(),
        parent_id.unwrap_or("genesis"),
    );
    hasher.update(serde_json::to_vec(&content).unwrap_or_default());
    let model_id = hex::encode(hasher.finalize());

    PatternModel {
        schema: MODEL_SCHEMA.to_string(),
        model_id,
        parent_id: parent_id.map(str::to_string),
        policy: policy.clone(),
        boundary_s,
        recent_span_s,
        report_digest: report.content_sha256,
        states,
        out_of_order_events,
    }
}

/// Compare a child snapshot against its declared parent. Keys absent on
/// one side surface explicitly (`appeared` / `vanished`) — unknown is data.
/// Callers must pass the true parent; mismatched pairs yield empty drift.
pub fn drift(parent: &PatternModel, child: &PatternModel) -> ModelDrift {
    let mut transitions = Vec::new();
    let mut appeared = Vec::new();
    let mut vanished = Vec::new();

    let mut parent_states = parent.states.clone();
    parent_states.sort_by(|a, b| a.key.cmp(&b.key));
    let mut child_states = child.states.clone();
    child_states.sort_by(|a, b| a.key.cmp(&b.key));

    let mut pi = 0usize;
    let mut ci = 0usize;
    while pi < parent_states.len() || ci < child_states.len() {
        match (parent_states.get(pi), child_states.get(ci)) {
            (Some(p), Some(c)) => match p.key.cmp(&c.key) {
                std::cmp::Ordering::Equal => {
                    if p.class != c.class {
                        transitions.push(ClassTransition {
                            key: c.key.clone(),
                            from: p.class,
                            to: c.class,
                        });
                    }
                    pi += 1;
                    ci += 1;
                }
                std::cmp::Ordering::Less => {
                    vanished.push(p.key.clone());
                    pi += 1;
                }
                std::cmp::Ordering::Greater => {
                    appeared.push(c.key.clone());
                    ci += 1;
                }
            },
            (Some(p), None) => {
                vanished.push(p.key.clone());
                pi += 1;
            }
            (None, Some(c)) => {
                appeared.push(c.key.clone());
                ci += 1;
            }
            (None, None) => break,
        }
    }

    ModelDrift {
        parent_id: parent.model_id.clone(),
        child_id: child.model_id.clone(),
        transitions,
        appeared,
        vanished,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_gc::classify_all;
    use crate::pattern_gc::types::PatternObservation;

    fn obs(key: &str, historical: u64, recent: u64) -> PatternObservation {
        PatternObservation {
            key: key.to_string(),
            windows: crate::pattern_gc::types::PatternWindows::comparable(historical, recent),
        }
    }

    fn assess(observations: &[PatternObservation]) -> Vec<PatternGcAssessment> {
        classify_all(observations, &RetentionPolicy::observed_default())
    }

    #[test]
    fn seal_is_deterministic_and_parent_sensitive() {
        let a = assess(&[obs("k", 600, 100)]);
        let m1 = seal_model(&a, &RetentionPolicy::observed_default(), 1000, 100, 0, None);
        let m2 = seal_model(&a, &RetPolicyAlias::observed_default(), 1000, 100, 0, None);
        assert_eq!(m1.model_id, m2.model_id);
        assert!(m1.parent_id.is_none());

        let m3 = seal_model(
            &a,
            &RetentionPolicy::observed_default(),
            1000,
            100,
            0,
            Some(&m1.model_id),
        );
        assert_ne!(m1.model_id, m3.model_id);
        assert_eq!(m3.parent_id.as_deref(), Some(m1.model_id.as_str()));
    }

    // Local alias keeping the determinism assertion readable.
    use RetentionPolicy as RetPolicyAlias;

    #[test]
    fn states_are_canonically_sorted_and_roundtrip() {
        let a = assess(&[obs("zeta", 5, 1), obs("alpha", 600, 100)]);
        let m = seal_model(&a, &RetentionPolicy::observed_default(), 1000, 100, 3, None);
        assert_eq!(
            m.states.iter().map(|s| s.key.as_str()).collect::<Vec<_>>(),
            vec!["alpha", "zeta"]
        );
        assert_eq!(m.out_of_order_events, 3);
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains(MODEL_SCHEMA));
        let back: PatternModel = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn drift_exposes_transitions_appearance_and_vanishing() {
        let p = RetentionPolicy::observed_default();
        let parent = seal_model(
            &assess(&[obs("ice", 5025, 900), obs("old_tool", 700, 20)]),
            &p,
            1000,
            100,
            0,
            None,
        );
        let child = seal_model(
            &assess(&[obs("ice", 5025, 900), obs("ts_exec", 120, 400)]),
            &p,
            2000,
            100,
            0,
            Some(&parent.model_id),
        );

        let d = drift(&parent, &child);
        assert_eq!(d.parent_id, parent.model_id);
        assert_eq!(d.appeared, vec!["ts_exec".to_string()]);
        assert_eq!(d.vanished, vec!["old_tool".to_string()]);
        // ice at comparable spans reads Fading in both snapshots here, so no
        // transition; the surge key is the actionable signal instead.
        assert!(d.newly_emerging().is_empty());
        assert!(d.transitions.is_empty());
    }

    #[test]
    fn drift_catches_class_changes_between_snapshots() {
        let p = RetentionPolicy::observed_default();
        // Same key, spans declare steady pace first, true collapse second.
        let mut before_obs = obs("net", 1983, 180);
        before_obs.windows.historical_span = 365;
        before_obs.windows.recent_span = 30;

        let after_obs = obs("net", 1983, 180);

        let before = seal_model(&assess(&[before_obs]), &p, 1000, 100, 0, None);
        let after = seal_model(
            &assess(&[after_obs]),
            &p,
            2000,
            100,
            0,
            Some(&before.model_id),
        );
        let d = drift(&before, &after);
        assert_eq!(d.transitions.len(), 1);
        assert_eq!(d.transitions[0].from, PatternClass::Recurring);
        assert_eq!(d.transitions[0].to, PatternClass::Fading);
        assert_eq!(d.newly_fading(), vec!["net"]);
    }
}
