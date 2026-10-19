//! Pure distilled-pattern contract — the agent <-> distillation bridge (v1).
//!
//! A **distilled pattern** is a recurrent, attributable observation extracted
//! from agent sessions (memory, skills, decisions, frictions), normalised so
//! agents and the sovereign runtime can consume it without re-reading raw
//! session traces. Raw traces stay behind the adapter; only the normalised
//! pattern crosses this boundary (privacy: minimise and redact at the edge).
//!
//! Domains are the three agent-OS use cases: ownership, privacy, agency.
//!
//! No I/O lives here. Session reading, store mutation, and transport live in
//! adapters (`cynic-node` usecases, CLI/MCP surfaces). This module only types
//! the pattern, seals it with a `sha3-512:` digest, and decides ingestion
//! (`decide_pattern_ingest`) with explicit, typed reason codes — never a
//! free-form string verdict.

use serde::{Deserialize, Serialize};

// ===========================================================================
// SCHEMAS
// ===========================================================================

/// Stable schema for a sealed distilled pattern.
pub const PATTERN_SCHEMA: &str = "cynic.pattern_distill.v1";
/// Stable schema for an ingestion request (pattern + submitting session).
pub const PATTERN_INGEST_REQUEST_SCHEMA: &str = "cynic.pattern_ingest_request.v1";
/// Stable schema for the ingestion decision.
pub const PATTERN_INGEST_DECISION_SCHEMA: &str = "cynic.pattern_ingest_decision.v1";

// ===========================================================================
// DOMAIN — the three agent-OS use cases, derived not declared
// ===========================================================================

/// Agent-OS use-case domain a pattern belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternDomain {
    /// Material claim, exit rights, contribution-derived governance weight.
    Ownership,
    /// Constituent data control, scoped context, no implicit exfiltration.
    Privacy,
    /// Bounded, reviewable agent authority with read-back proof.
    Agency,
}

// ===========================================================================
// SOURCE — content-addressed evidence references, never raw traces
// ===========================================================================

/// Where the pattern evidence was observed. Adapters populate these;
/// the protocol never reads a session file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternSourceKind {
    /// Observed in an agent session trace (adapters minimise and redact).
    SessionTrace,
    /// Observed in a validation or audit result (gate failure, debt tier).
    ValidationAudit,
    /// Manual operator note with a typed digest.
    OperatorNote,
}

/// Content-addressed evidence reference behind a distilled pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternSource {
    pub kind: PatternSourceKind,
    /// Content-addressed reference (digest, receipt id, or audit ref).
    pub ref_id: String,
    /// Optional human-readable label; never the only identifier.
    #[serde(default)]
    pub label: Option<String>,
}

// ===========================================================================
// PATTERN — the sealed unit that crosses the bridge
// ===========================================================================

/// A distilled pattern: one recurrent observation, normalised and sealed.
///
/// `digest` seals every field above it: it is `sha3-512:` over the canonical
/// JSON of the pattern with the digest field cleared. A pattern whose
/// recomputed digest differs from its declared digest is rejected on ingest —
/// the seal, not prose, carries integrity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistilledPattern {
    pub schema: String,
    pub pattern_id: String,
    pub domain: PatternDomain,
    pub title: String,
    pub statement: String,
    #[serde(default)]
    pub sources: Vec<PatternSource>,
    /// Calibrated confidence in [0.0, 1.0]. Below 0.5 the pattern needs
    /// human review before it may feed skills or policy.
    #[serde(default)]
    pub confidence: f64,
    /// Open questions the distillation could not close. Preserved
    /// explicitly: ingest never coerces them away.
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
    /// Integrity seal (`sha3-512:` prefix, constitutional digest law).
    #[serde(default)]
    pub digest: String,
}

/// Canonical digest input: the pattern with its seal cleared, so the digest
/// never covers itself.
#[derive(Serialize)]
struct PatternDigestInput<'a> {
    schema: &'a str,
    pattern_id: &'a str,
    domain: &'a PatternDomain,
    title: &'a str,
    statement: &'a str,
    sources: &'a [PatternSource],
    confidence: f64,
    residual_unknowns: &'a [String],
}

/// Seal a pattern: compute the `sha3-512:` digest over its canonical JSON
/// and store it in `digest`. Returns the sealed pattern.
pub fn seal_pattern(mut pattern: DistilledPattern) -> DistilledPattern {
    pattern.digest = pattern_digest(&pattern);
    pattern
}

/// Compute the integrity seal for a pattern (digest field excluded).
/// Returns `sha3-512:<hex>` or `sha3-512:invalid_canonical_json` when the
/// canonical form cannot be built (never panics, never unprefixed).
pub fn pattern_digest(pattern: &DistilledPattern) -> String {
    let input = PatternDigestInput {
        schema: &pattern.schema,
        pattern_id: &pattern.pattern_id,
        domain: &pattern.domain,
        title: &pattern.title,
        statement: &pattern.statement,
        sources: &pattern.sources,
        confidence: pattern.confidence,
        residual_unknowns: &pattern.residual_unknowns,
    };
    crate::digest::digest_json(&input)
        .unwrap_or_else(|_| "sha3-512:invalid_canonical_json".to_string())
}

/// True when the declared seal matches the recomputed seal.
pub fn pattern_seal_valid(pattern: &DistilledPattern) -> bool {
    !pattern.digest.is_empty() && pattern.digest == pattern_digest(pattern)
}

// ===========================================================================
// INGEST — request, verdict, pure decision
// ===========================================================================

/// Ingestion request: a sealed pattern plus the submitting session identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternIngestRequest {
    pub schema: String,
    pub pattern: DistilledPattern,
    /// Session that produced the distillation (attribution, not authority).
    pub session_id: String,
    /// Agent or operator principal submitting (e.g. `agent:hermes-3`).
    pub submitted_by: String,
}

/// Ingestion verdict. `Accepted` is idempotent: re-ingesting a known digest
/// is accepted as a duplicate, never stored twice (see `reconcile`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternIngestVerdict {
    Accepted,
    NeedsReview,
    Rejected,
}

/// Machine-readable ingestion decision: verdict plus typed reason codes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternIngestDecision {
    pub schema: String,
    pub pattern_id: String,
    pub verdict: PatternIngestVerdict,
    /// Typed reason codes (`empty_statement`, `no_source`, `low_confidence`,
    /// `seal_mismatch`, `duplicate_digest`, `ok`). Never prose.
    pub reasons: Vec<String>,
    /// True when the digest was already ingested (idempotent re-ingest).
    pub duplicate: bool,
    pub digest: String,
}

/// Pure ingestion gate: content checks only, no store access.
/// Duplicate detection lives in `reconcile_pattern_ingest` (needs the known
/// digest set, injected by the caller — the protocol never reads a store).
pub fn decide_pattern_ingest(request: &PatternIngestRequest) -> PatternIngestDecision {
    let pattern = &request.pattern;
    let mut reasons: Vec<String> = Vec::new();

    if pattern.schema != PATTERN_SCHEMA {
        reasons.push("schema_mismatch".to_string());
    }
    if pattern.pattern_id.trim().is_empty() {
        reasons.push("empty_pattern_id".to_string());
    }
    if pattern.title.trim().is_empty() {
        reasons.push("empty_title".to_string());
    }
    if pattern.statement.trim().is_empty() {
        reasons.push("empty_statement".to_string());
    }
    if request.session_id.trim().is_empty() {
        reasons.push("empty_session_id".to_string());
    }
    if request.submitted_by.trim().is_empty() {
        reasons.push("empty_submitted_by".to_string());
    }
    if pattern.sources.is_empty() {
        reasons.push("no_source".to_string());
    }
    if !pattern.confidence.is_finite() || pattern.confidence < 0.0 || pattern.confidence > 1.0 {
        reasons.push("confidence_out_of_range".to_string());
    }
    if !pattern_seal_valid(pattern) {
        reasons.push("seal_mismatch".to_string());
    }

    if !reasons.is_empty() {
        return PatternIngestDecision {
            schema: PATTERN_INGEST_DECISION_SCHEMA.to_string(),
            pattern_id: pattern.pattern_id.clone(),
            verdict: PatternIngestVerdict::Rejected,
            reasons,
            duplicate: false,
            digest: pattern.digest.clone(),
        };
    }

    if pattern.confidence < 0.5 {
        return PatternIngestDecision {
            schema: PATTERN_INGEST_DECISION_SCHEMA.to_string(),
            pattern_id: pattern.pattern_id.clone(),
            verdict: PatternIngestVerdict::NeedsReview,
            reasons: vec!["low_confidence".to_string()],
            duplicate: false,
            digest: pattern.digest.clone(),
        };
    }

    PatternIngestDecision {
        schema: PATTERN_INGEST_DECISION_SCHEMA.to_string(),
        pattern_id: pattern.pattern_id.clone(),
        verdict: PatternIngestVerdict::Accepted,
        reasons: vec!["ok".to_string()],
        duplicate: false,
        digest: pattern.digest.clone(),
    }
}

/// Reconcile a decision against the already-ingested digest set (injected by
/// the caller). A known digest is an idempotent accepted duplicate — stored
/// once, never twice. A fresh digest keeps the gate verdict unchanged.
pub fn reconcile_pattern_ingest(
    decision: &PatternIngestDecision,
    known_digests: &[String],
) -> PatternIngestDecision {
    if decision.verdict != PatternIngestVerdict::Accepted {
        return decision.clone();
    }
    if known_digests.contains(&decision.digest) {
        let mut dup = decision.clone();
        dup.duplicate = true;
        dup.reasons = vec!["duplicate_digest".to_string()];
        return dup;
    }
    decision.clone()
}

/// Residual unknowns are data, not zero: ingest preserves them verbatim.
/// Returns true when the pattern carries at least one open question.
pub fn pattern_has_open_unknowns(pattern: &DistilledPattern) -> bool {
    !pattern.residual_unknowns.is_empty()
}

// ===========================================================================
// TESTS — pure, no I/O, deterministic
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    pub fn mk_pattern(id: &str, domain: PatternDomain, confidence: f64) -> DistilledPattern {
        seal_pattern(DistilledPattern {
            schema: PATTERN_SCHEMA.to_string(),
            pattern_id: id.to_string(),
            domain,
            title: format!("title for {id}"),
            statement: format!(
                "recurrent observation {id}: agents re-derive instead of reading the contract"
            ),
            sources: vec![PatternSource {
                kind: PatternSourceKind::SessionTrace,
                ref_id: "sha3-512:session-evidence".to_string(),
                label: None,
            }],
            confidence,
            residual_unknowns: vec![],
            digest: String::new(),
        })
    }

    fn mk_request(pattern: DistilledPattern) -> PatternIngestRequest {
        PatternIngestRequest {
            schema: PATTERN_INGEST_REQUEST_SCHEMA.to_string(),
            pattern: pattern,
            session_id: "session-001".to_string(),
            submitted_by: "agent:hermes-test".to_string(),
        }
    }

    #[test]
    fn schema_names_are_stable_contracts() {
        assert_eq!(PATTERN_SCHEMA, "cynic.pattern_distill.v1");
        assert_eq!(
            PATTERN_INGEST_REQUEST_SCHEMA,
            "cynic.pattern_ingest_request.v1"
        );
        assert_eq!(
            PATTERN_INGEST_DECISION_SCHEMA,
            "cynic.pattern_ingest_decision.v1"
        );
    }

    #[test]
    fn seal_uses_constitutional_prefix() {
        let pattern = mk_pattern("p-001", PatternDomain::Agency, 0.8);
        assert!(pattern.digest.starts_with("sha3-512:"));
        assert!(pattern_seal_valid(&pattern));
    }

    #[test]
    fn tampered_pattern_breaks_seal() {
        let mut pattern = mk_pattern("p-002", PatternDomain::Privacy, 0.9);
        pattern.statement = "rewritten after sealing".to_string();
        assert!(!pattern_seal_valid(&pattern));
        let decision = decide_pattern_ingest(&mk_request(pattern));
        assert_eq!(decision.verdict, PatternIngestVerdict::Rejected);
        assert!(decision.reasons.contains(&"seal_mismatch".to_string()));
    }

    #[test]
    fn healthy_pattern_is_accepted() {
        for domain in [
            PatternDomain::Ownership,
            PatternDomain::Privacy,
            PatternDomain::Agency,
        ] {
            let decision = decide_pattern_ingest(&mk_request(mk_pattern("p-ok", domain, 0.8)));
            assert_eq!(
                decision.verdict,
                PatternIngestVerdict::Accepted,
                "domain: {domain:?}"
            );
            assert_eq!(decision.reasons, vec!["ok".to_string()]);
        }
    }

    #[test]
    fn low_confidence_needs_review_not_rejected() {
        let decision = decide_pattern_ingest(&mk_request(mk_pattern(
            "p-low",
            PatternDomain::Ownership,
            0.3,
        )));
        assert_eq!(decision.verdict, PatternIngestVerdict::NeedsReview);
        assert_eq!(decision.reasons, vec!["low_confidence".to_string()]);
    }

    #[test]
    fn empty_statement_and_missing_source_reject() {
        let mut pattern = mk_pattern("p-bad", PatternDomain::Agency, 0.9);
        pattern.statement = "   ".to_string();
        pattern.sources.clear();
        pattern.digest = pattern_digest(&pattern);
        let decision = decide_pattern_ingest(&mk_request(pattern));
        assert_eq!(decision.verdict, PatternIngestVerdict::Rejected);
        assert!(decision.reasons.contains(&"empty_statement".to_string()));
        assert!(decision.reasons.contains(&"no_source".to_string()));
    }

    #[test]
    fn confidence_out_of_range_rejects() {
        let decision = decide_pattern_ingest(&mk_request(mk_pattern(
            "p-nan",
            PatternDomain::Privacy,
            f64::NAN,
        )));
        assert_eq!(decision.verdict, PatternIngestVerdict::Rejected);
        assert!(decision
            .reasons
            .contains(&"confidence_out_of_range".to_string()));
    }

    #[test]
    fn duplicate_digest_is_idempotent_accept() {
        let decision =
            decide_pattern_ingest(&mk_request(mk_pattern("p-dup", PatternDomain::Agency, 0.8)));
        assert_eq!(decision.verdict, PatternIngestVerdict::Accepted);
        let reconciled = reconcile_pattern_ingest(&decision, &[decision.digest.clone()]);
        assert_eq!(reconciled.verdict, PatternIngestVerdict::Accepted);
        assert!(reconciled.duplicate);
        assert_eq!(reconciled.reasons, vec!["duplicate_digest".to_string()]);
        let fresh = reconcile_pattern_ingest(&decision, &[]);
        assert!(!fresh.duplicate);
        assert_eq!(fresh.verdict, PatternIngestVerdict::Accepted);
    }

    #[test]
    fn rejected_decision_survives_reconcile_unchanged() {
        let decision = decide_pattern_ingest(&mk_request(mk_pattern(
            "p-low2",
            PatternDomain::Privacy,
            0.1,
        )));
        let reconciled = reconcile_pattern_ingest(&decision, &[]);
        assert_eq!(reconciled, decision);
    }

    #[test]
    fn residual_unknowns_are_preserved_data() {
        let mut pattern = mk_pattern("p-unk", PatternDomain::Ownership, 0.8);
        pattern.residual_unknowns = vec!["calibration gap: no production trace sink".to_string()];
        pattern.digest = pattern_digest(&pattern);
        assert!(pattern_has_open_unknowns(&pattern));
        let decision = decide_pattern_ingest(&mk_request(pattern));
        assert_eq!(decision.verdict, PatternIngestVerdict::Accepted);
    }
}
