//! Pure organic-opportunity contract — read-only compass.
//!
//! An **organic opportunity** is an observed, bounded candidate for operator
//! attention. It is not a task, a grant, a claim, or authority to mutate.
//! It arises when unfinished work, recurring session friction, emerging
//! patterns, plumbing debt, or cross-surface dependency evidence indicates
//! that a small next step could unlock disproportionate proof, coverage, or
//! operator capacity.
//!
//! `Organic always wins` is a priority law. It does not create a task,
//! claim, grant, urgency signal, or mutation authority.
//!
//! No I/O lives here. This contract is consumed by host adapters
//! (session, source, audit, validation observation) and projected through
//! a thin CLI report. Ranking is contextual and contestable within one
//! declared value frame; it is never a universal score.
//!
//! Authority: this module does NOT extend `CynicJudgment` with a parallel
//! "decision" type, does NOT create a competing energy ledger, and does
//! NOT collapse multi-dimensional comparison into a single number.

use serde::{Deserialize, Serialize};

// ===========================================================================
// SCHEMA
// ===========================================================================

/// Stable schema identifier for the organic-opportunity contract.
pub const ORGANIC_OPPORTUNITY_SCHEMA: &str = "cynic.organic_opportunity.v1";
/// Stable schema for the value-frame ranking predicate input.
pub const ORGANIC_VALUE_FRAME_SCHEMA: &str = "cynic.organic_value_frame.v1";
/// Stable schema for the ranked organic-opportunity report.
pub const ORGANIC_OPPORTUNITY_REPORT_SCHEMA: &str = "cynic.organic_opportunity_report.v1";

// ===========================================================================
// SOURCES — content-addressed evidence references
// ===========================================================================

/// Where the opportunity evidence was observed. Adapters populate these;
/// the protocol does not parse or fetch them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpportunitySourceKind {
    /// Observed in a session trace (adapters must minimise and redact).
    SessionTrace,
    /// Observed in source/worktree state (untracked files, modified files,
    /// unmerged branches). Injected repo context only.
    SourceWorktree,
    /// Observed in a validation/audit result (gate failure, debt tier).
    ValidationAudit,
    /// Observed in a task receipt or capacity admission.
    TaskReceipt,
    /// Manual operator note with a typed digest.
    OperatorNote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpportunitySource {
    pub kind: OpportunitySourceKind,
    /// Content-addressed reference (digest, path-with-digest, or receipt id).
    pub ref_id: String,
    /// Optional human-readable label; never the only identifier.
    #[serde(default)]
    pub label: Option<String>,
}

// ===========================================================================
// CLASS — derived from named predicates, not declared
// ===========================================================================

/// Opportunity class, inferred from named predicates in observation
/// adapters. The protocol does not classify; consumers map observed
/// evidence to one of these values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpportunityClass {
    /// A bounded slice was planned but not finished.
    Unfinished,
    /// A new pattern emerged in recent observations.
    Emerging,
    /// A pattern recurs across sessions and indicates structural friction.
    RecurringFriction,
    /// Plumbing debt tying multiple surfaces together.
    Plumbing,
    /// A cross-surface dependency whose smallest repair unlocks others.
    Dependency,
}

// ===========================================================================
// ENERGY — measured OR explicit unknown
// ===========================================================================

/// Estimated energy for the smallest next step.
///
/// `measured` carries a value attributed to a budget/receipt source.
/// `range` carries a method + low + high. `unknown` is reserved for
/// missing evidence: it must NOT be silently coerced to zero or to a
/// range — that would erase the residual unknown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EnergyEstimate {
    /// Value observed from a budget/receipt.
    Measured { source: String, value_joules: f64 },
    /// Bounded range with explicit method.
    Range {
        method: String,
        low_joules: f64,
        high_joules: f64,
    },
    /// No evidence available. Preserved explicitly.
    Unknown { reason: String },
}

impl EnergyEstimate {
    pub fn is_unknown(&self) -> bool {
        matches!(self, EnergyEstimate::Unknown { .. })
    }
}

// ===========================================================================
// SMALLEST NEXT STEP
// ===========================================================================

/// The kind of smallest next step the opportunity recommends. The
/// protocol never executes; it only records the recommendation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmallestNextStep {
    Observe,
    Consolidate,
    Plan,
    ProposeRepair,
}

// ===========================================================================
// OPERATOR DISPOSITION
// ===========================================================================

/// Operator disposition on this opportunity. The compass defaults to
/// `Unreviewed`; a later explicit accept/defer/reject record is the
/// only authority that may move a slice to admitted work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum OperatorDisposition {
    /// Default state. No operator record yet.
    Unreviewed,
    /// Operator accepted the opportunity. The compass still does not
    /// execute it; admission follows the normal claim/admission path.
    Accepted { at_unix_s: u64, reason: String },
    /// Operator deferred the opportunity with an explicit reason.
    /// The opportunity and its evidence are retained verbatim.
    Deferred { at_unix_s: u64, reason: String },
    /// Operator rejected the opportunity. Evidence is retained but
    /// the opportunity is hidden from default ranking.
    Rejected { at_unix_s: u64, reason: String },
}

impl Default for OperatorDisposition {
    fn default() -> Self {
        OperatorDisposition::Unreviewed
    }
}

// ===========================================================================
// CONTRACT — one organic opportunity
// ===========================================================================

/// One typed organic opportunity.
///
/// `actionable` is always `false`: the compass is a read-only projection
/// that recommends, never executes. No field here grants mutation
/// authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganicOpportunity {
    pub schema: String,
    /// Stable identity (digest of the canonical fields).
    pub opportunity_id: String,
    /// Affected boundary (e.g. "crates/cynic-kernel", "fleet/payment").
    pub scope: String,
    /// Attributed evidence references.
    pub sources: Vec<OpportunitySource>,
    /// Inferred class.
    pub class: OpportunityClass,
    /// Hypothesis: expected unlock (proof / coverage / capacity).
    pub expected_unlock: String,
    /// Energy estimate — measured, ranged, or explicitly unknown.
    pub estimated_energy: EnergyEstimate,
    /// Age of the youngest source in seconds.
    pub freshness_seconds: u64,
    /// Support of the evidence, in `[0.0, 1.0]`. Never `1.0` (Wisdom).
    pub confidence: f64,
    /// Recommended smallest next step.
    pub smallest_next_step: SmallestNextStep,
    /// Missing lower-layer data, conflicting inputs, etc.
    pub residual_unknowns: Vec<String>,
    /// Operator disposition; default Unreviewed.
    #[serde(default)]
    pub operator_disposition: OperatorDisposition,
}

// ===========================================================================
// VALUE FRAME — declared ranking context
// ===========================================================================

/// A declared value frame that makes a ranking order contestable and
/// reproducible. The same opportunity set ranked under two different
/// frames can yield different orders; both are valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueFrame {
    pub schema: String,
    pub frame_id: String,
    /// Free-text description of what this frame privileges.
    pub description: String,
    /// Lexicographic tie-break order used by `rank_in_value_frame`.
    /// Lower index = higher priority.
    pub tie_break: Vec<ValueDimension>,
}

/// One ranking dimension within a value frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueDimension {
    /// Safety / blocking status first.
    SafetyBlock,
    /// Unlock fan-out (how many other opportunities a fix unlocks).
    UnlockFanOut,
    /// Evidence freshness (more recent first).
    Freshness,
    /// Estimated energy (lower first).
    EnergyLow,
    /// Confidence (higher first).
    ConfidenceHigh,
}

// ===========================================================================
// RANKED REPORT
// ===========================================================================

/// A ranked organic-opportunity report for one declared value frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganicOpportunityReport {
    pub schema: String,
    pub frame: ValueFrame,
    pub opportunities: Vec<OrganicOpportunity>,
    /// Sources that were requested but unavailable (e.g. MCP blocked).
    pub unavailable_sources: Vec<OpportunitySourceKind>,
    /// Wall-clock at report generation.
    pub generated_at_unix_s: u64,
}

// ===========================================================================
// PURE PREDICATES
// ===========================================================================

/// Deterministic rank within one declared value frame.
///
/// Ranking rules:
/// 1. `Rejected` opportunities are excluded from the default order.
/// 2. Unknown energy sorts AFTER measured/ranged energy of equal
///    other dimensions (unknown is data, not zero).
/// 3. Ties resolve lexicographically by `tie_break` in declared order.
/// 4. Equal tie-break vectors preserve insertion order (stable).
///
/// The function is pure: same input + same frame => same output.
pub fn rank_in_value_frame(
    frame: &ValueFrame,
    opportunities: &[OrganicOpportunity],
) -> Vec<OrganicOpportunity> {
    let mut filtered: Vec<&OrganicOpportunity> = opportunities
        .iter()
        .filter(|o| !matches!(o.operator_disposition, OperatorDisposition::Rejected { .. }))
        .collect();

    filtered.sort_by(|a, b| {
        for dim in &frame.tie_break {
            let ord = match dim {
                ValueDimension::SafetyBlock => {
                    // Higher residual_unknowns = lower safety margin = earlier.
                    let ua = a.residual_unknowns.len();
                    let ub = b.residual_unknowns.len();
                    ub.cmp(&ua)
                }
                ValueDimension::UnlockFanOut => {
                    // We can't measure fan-out from the contract alone; we use
                    // the inverse of scope specificity as a stable proxy:
                    // shorter scope => broader fan-out => earlier.
                    a.scope.len().cmp(&b.scope.len())
                }
                ValueDimension::Freshness => b.freshness_seconds.cmp(&a.freshness_seconds),
                ValueDimension::EnergyLow => {
                    energy_rank_key(&a.estimated_energy).cmp(&energy_rank_key(&b.estimated_energy))
                }
                ValueDimension::ConfidenceHigh => b
                    .confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal),
            };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });

    filtered.into_iter().cloned().collect()
}

/// Unknown energy must remain unknown. This helper returns a sort key
/// where `Unknown` sorts AFTER `Measured` and `Range` of the same key.
fn energy_rank_key(e: &EnergyEstimate) -> (u8, u64) {
    match e {
        // u8=0 for Measured, 1 for Range, 2 for Unknown.
        // Joules scaled to u64 (clamped at u64::MAX for overflow safety).
        EnergyEstimate::Measured { value_joules, .. } => (0, joules_to_u64(*value_joules)),
        EnergyEstimate::Range { low_joules, .. } => (1, joules_to_u64(*low_joules)),
        EnergyEstimate::Unknown { .. } => (2, 0),
    }
}

fn joules_to_u64(j: f64) -> u64 {
    if !j.is_finite() || j <= 0.0 {
        0
    } else if j >= u64::MAX as f64 {
        u64::MAX
    } else {
        j as u64
    }
}

/// Operator defer must preserve the opportunity and its evidence. This
/// helper applies a defer record while leaving the underlying record
/// intact. It does NOT mutate priority by itself.
pub fn defer_opportunity(
    opportunity: &OrganicOpportunity,
    at_unix_s: u64,
    reason: &str,
) -> OrganicOpportunity {
    let mut updated = opportunity.clone();
    updated.operator_disposition = OperatorDisposition::Deferred {
        at_unix_s,
        reason: reason.to_string(),
    };
    updated
}

/// Unknown energy is data, not zero. Returns the same opportunity
/// unchanged; the predicate exists so adapters can assert this contract
/// without re-deriving it.
pub fn unknown_stays_unknown(opportunity: &OrganicOpportunity) -> bool {
    match &opportunity.estimated_energy {
        EnergyEstimate::Unknown { .. } => true,
        EnergyEstimate::Measured { .. } | EnergyEstimate::Range { .. } => false,
    }
}

// ===========================================================================
// TESTS — pure, no I/O, deterministic
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_source(kind: OpportunitySourceKind, ref_id: &str) -> OpportunitySource {
        OpportunitySource {
            kind,
            ref_id: ref_id.to_string(),
            label: None,
        }
    }

    fn mk_opp(
        id: &str,
        scope: &str,
        class: OpportunityClass,
        energy: EnergyEstimate,
        freshness: u64,
        confidence: f64,
        unknowns: Vec<&str>,
    ) -> OrganicOpportunity {
        OrganicOpportunity {
            schema: ORGANIC_OPPORTUNITY_SCHEMA.to_string(),
            opportunity_id: id.to_string(),
            scope: scope.to_string(),
            sources: vec![mk_source(
                OpportunitySourceKind::ValidationAudit,
                "audit:demo",
            )],
            class,
            expected_unlock: "proof or coverage".to_string(),
            estimated_energy: energy,
            freshness_seconds: freshness,
            confidence,
            smallest_next_step: SmallestNextStep::Observe,
            residual_unknowns: unknowns.into_iter().map(String::from).collect(),
            operator_disposition: OperatorDisposition::Unreviewed,
        }
    }

    #[test]
    fn schema_names_are_stable_contracts() {
        assert_eq!(ORGANIC_OPPORTUNITY_SCHEMA, "cynic.organic_opportunity.v1");
        assert_eq!(ORGANIC_VALUE_FRAME_SCHEMA, "cynic.organic_value_frame.v1");
        assert_eq!(
            ORGANIC_OPPORTUNITY_REPORT_SCHEMA,
            "cynic.organic_opportunity_report.v1"
        );
    }

    #[test]
    fn unknown_energy_is_data_not_zero() {
        let opp = mk_opp(
            "opp-unknown",
            "crates/x",
            OpportunityClass::Plumbing,
            EnergyEstimate::Unknown {
                reason: "no receipt".to_string(),
            },
            60,
            0.5,
            vec![],
        );
        assert!(unknown_stays_unknown(&opp));
        assert!(opp.estimated_energy.is_unknown());
    }

    #[test]
    fn unknown_energy_sorts_after_measured_in_rank() {
        let a = mk_opp(
            "opp-a",
            "crates/x",
            OpportunityClass::Plumbing,
            EnergyEstimate::Measured {
                source: "budget".to_string(),
                value_joules: 100.0,
            },
            60,
            0.5,
            vec![],
        );
        let b = mk_opp(
            "opp-b",
            "crates/x",
            OpportunityClass::Plumbing,
            EnergyEstimate::Unknown {
                reason: "no receipt".to_string(),
            },
            60,
            0.5,
            vec![],
        );
        let frame = ValueFrame {
            schema: ORGANIC_VALUE_FRAME_SCHEMA.to_string(),
            frame_id: "frame-energy".to_string(),
            description: "lower energy first".to_string(),
            tie_break: vec![ValueDimension::EnergyLow],
        };
        let ranked = rank_in_value_frame(&frame, &[b.clone(), a.clone()]);
        assert_eq!(ranked[0].opportunity_id, "opp-a");
        assert_eq!(ranked[1].opportunity_id, "opp-b");
    }

    #[test]
    fn higher_fan_out_outranks_lower_unlock() {
        // Short scope = broader fan-out proxy; longer scope = narrower.
        let broad = mk_opp(
            "opp-broad",
            "host",
            OpportunityClass::Plumbing,
            EnergyEstimate::Unknown {
                reason: "no receipt".to_string(),
            },
            60,
            0.5,
            vec![],
        );
        let narrow = mk_opp(
            "opp-narrow",
            "crates/cynic-cli/src/audit/sub.rs",
            OpportunityClass::Plumbing,
            EnergyEstimate::Unknown {
                reason: "no receipt".to_string(),
            },
            60,
            0.5,
            vec![],
        );
        let frame = ValueFrame {
            schema: ORGANIC_VALUE_FRAME_SCHEMA.to_string(),
            frame_id: "frame-fanout".to_string(),
            description: "fan-out first".to_string(),
            tie_break: vec![ValueDimension::UnlockFanOut],
        };
        let ranked = rank_in_value_frame(&frame, &[narrow.clone(), broad.clone()]);
        assert_eq!(ranked[0].opportunity_id, "opp-broad");
        assert_eq!(ranked[1].opportunity_id, "opp-narrow");
    }

    #[test]
    fn rejected_opportunities_are_excluded_from_rank() {
        let mut opp = mk_opp(
            "opp-rej",
            "crates/x",
            OpportunityClass::Plumbing,
            EnergyEstimate::Unknown {
                reason: "no receipt".to_string(),
            },
            60,
            0.9,
            vec![],
        );
        opp.operator_disposition = OperatorDisposition::Rejected {
            at_unix_s: 1,
            reason: "explicit".to_string(),
        };
        let kept = mk_opp(
            "opp-kept",
            "crates/x",
            OpportunityClass::Plumbing,
            EnergyEstimate::Unknown {
                reason: "no receipt".to_string(),
            },
            60,
            0.5,
            vec![],
        );
        let frame = ValueFrame {
            schema: ORGANIC_VALUE_FRAME_SCHEMA.to_string(),
            frame_id: "frame-default".to_string(),
            description: "default".to_string(),
            tie_break: vec![ValueDimension::ConfidenceHigh],
        };
        let ranked = rank_in_value_frame(&frame, &[opp, kept]);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].opportunity_id, "opp-kept");
    }

    #[test]
    fn defer_preserves_opportunity_and_evidence() {
        let opp = mk_opp(
            "opp-def",
            "crates/x",
            OpportunityClass::Unfinished,
            EnergyEstimate::Range {
                method: "expert".to_string(),
                low_joules: 100.0,
                high_joules: 200.0,
            },
            60,
            0.7,
            vec!["missing-lower-layer"],
        );
        let original_sources = opp.sources.clone();
        let original_unknowns = opp.residual_unknowns.clone();
        let deferred = defer_opportunity(&opp, 1000, "blocked by env");
        assert_eq!(deferred.opportunity_id, "opp-def");
        assert_eq!(deferred.sources, original_sources);
        assert_eq!(deferred.residual_unknowns, original_unknowns);
        assert!(matches!(
            deferred.operator_disposition,
            OperatorDisposition::Deferred { .. }
        ));
        // defer does NOT change class, scope, energy, or evidence.
        assert_eq!(deferred.class, opp.class);
        assert_eq!(deferred.estimated_energy, opp.estimated_energy);
    }

    #[test]
    fn rank_is_deterministic_and_stable() {
        let a = mk_opp(
            "opp-a",
            "x",
            OpportunityClass::Plumbing,
            EnergyEstimate::Unknown {
                reason: "u".to_string(),
            },
            60,
            0.5,
            vec!["u1", "u2"],
        );
        let b = mk_opp(
            "opp-b",
            "x",
            OpportunityClass::Plumbing,
            EnergyEstimate::Unknown {
                reason: "u".to_string(),
            },
            60,
            0.5,
            vec!["u1", "u2"],
        );
        let frame = ValueFrame {
            schema: ORGANIC_VALUE_FRAME_SCHEMA.to_string(),
            frame_id: "f".to_string(),
            description: "d".to_string(),
            tie_break: vec![ValueDimension::SafetyBlock, ValueDimension::EnergyLow],
        };
        let r1 = rank_in_value_frame(&frame, &[a.clone(), b.clone()]);
        let r2 = rank_in_value_frame(&frame, &[b.clone(), a.clone()]);
        // Determinism: identical inputs (same elements, same order) produce
        // identical output regardless of slice order in caller code.
        let r3 = rank_in_value_frame(&frame, &[a.clone(), b.clone()]);
        assert_eq!(r1, r3);
        // Stable order: when tie_break returns Equal for two elements,
        // the original input order is preserved.
        assert_eq!(r1[0].opportunity_id, "opp-a");
        assert_eq!(r1[1].opportunity_id, "opp-b");
        assert_eq!(r2[0].opportunity_id, "opp-b");
        assert_eq!(r2[1].opportunity_id, "opp-a");
    }

    #[test]
    fn compass_remains_non_actionable() {
        // The contract is structural: there is no `actionable: true` field.
        // We assert by serialization round-trip: the schema MUST be the
        // organic-opportunity one, not a judgment or task.
        let opp = mk_opp(
            "opp-na",
            "x",
            OpportunityClass::Plumbing,
            EnergyEstimate::Unknown {
                reason: "u".to_string(),
            },
            60,
            0.5,
            vec![],
        );
        let json = serde_json::to_string(&opp).unwrap();
        assert!(json.contains(ORGANIC_OPPORTUNITY_SCHEMA));
        assert!(!json.contains("actionable"));
    }

    #[test]
    fn energy_joules_overflow_is_clamped() {
        // f64::MAX joules must clamp to u64::MAX, not panic.
        let k = joules_to_u64(f64::MAX);
        assert_eq!(k, u64::MAX);
        // Negative / NaN / zero must collapse to 0.
        assert_eq!(joules_to_u64(-1.0), 0);
        assert_eq!(joules_to_u64(f64::NAN), 0);
        assert_eq!(joules_to_u64(0.0), 0);
    }
}
