//! Pattern ingest usecase: distilled patterns -> sovereign store (v1).
//!
//! The pure gate lives in `cynic_protocol::pattern_distill`
//! (`decide_pattern_ingest` + `reconcile_pattern_ingest`). This usecase is
//! the orchestration shell: it injects the known-digest set through
//! [`PatternStorePort`], runs the gate, and stores only accepted,
//! non-duplicate patterns. No filesystem, no network here — the durable
//! adapter implements the port downstream.

use cynic_protocol::pattern_distill::{
    decide_pattern_ingest, reconcile_pattern_ingest, PatternIngestDecision, PatternIngestRequest,
};

/// Storage port for distilled patterns. The only store access the usecase
/// needs: the known digest set (for idempotence) and a single insert.
pub trait PatternStorePort {
    /// Digests already ingested. Returned as observed; absence of a digest
    /// means "not ingested", never an error.
    fn known_digests(&self) -> Vec<String>;
    /// Persist an accepted, non-duplicate pattern. Called at most once per
    /// ingest; never called for rejected, review-pending, or duplicate
    /// decisions.
    fn store(&mut self, request: &PatternIngestRequest) -> Result<(), String>;
}

/// Ingest one distilled pattern: gate, reconcile against known digests,
/// store when accepted and fresh. Returns the reconciled decision; the
/// store error, if any, is reported as an explicit unknown in the returned
/// decision's reasons with a `Rejected` verdict — never a silent pass.
pub fn ingest_pattern(
    port: &mut dyn PatternStorePort,
    request: &PatternIngestRequest,
) -> PatternIngestDecision {
    let decided = decide_pattern_ingest(request);
    let reconciled = reconcile_pattern_ingest(&decided, &port.known_digests());
    if reconciled.verdict == cynic_protocol::pattern_distill::PatternIngestVerdict::Accepted
        && !reconciled.duplicate
    {
        if let Err(error) = port.store(request) {
            let mut failed = reconciled.clone();
            failed.verdict = cynic_protocol::pattern_distill::PatternIngestVerdict::Rejected;
            failed.reasons = vec![format!("store_failed:{error}")];
            return failed;
        }
    }
    reconciled
}

/// In-memory port for tests and dry-run read-back. No I/O.
#[derive(Debug, Default)]
pub struct InMemoryPatternStore {
    stored: Vec<PatternIngestRequest>,
}

impl InMemoryPatternStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.stored.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stored.is_empty()
    }
}

impl PatternStorePort for InMemoryPatternStore {
    fn known_digests(&self) -> Vec<String> {
        self.stored
            .iter()
            .map(|request| request.pattern.digest.clone())
            .collect()
    }

    fn store(&mut self, request: &PatternIngestRequest) -> Result<(), String> {
        self.stored.push(request.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use cynic_protocol::pattern_distill::{
        seal_pattern, DistilledPattern, PatternDomain, PatternIngestVerdict, PatternSource,
        PatternSourceKind, PATTERN_INGEST_REQUEST_SCHEMA, PATTERN_SCHEMA,
    };

    use super::*;

    fn mk_request(id: &str, confidence: f64) -> PatternIngestRequest {
        let pattern = seal_pattern(DistilledPattern {
            schema: PATTERN_SCHEMA.to_string(),
            pattern_id: id.to_string(),
            domain: PatternDomain::Agency,
            title: format!("title {id}"),
            statement: format!("recurrent observation {id}"),
            sources: vec![PatternSource {
                kind: PatternSourceKind::SessionTrace,
                ref_id: "sha3-512:evidence".to_string(),
                label: None,
            }],
            confidence,
            residual_unknowns: vec![],
            digest: String::new(),
        });
        PatternIngestRequest {
            schema: PATTERN_INGEST_REQUEST_SCHEMA.to_string(),
            pattern,
            session_id: "session-001".to_string(),
            submitted_by: "agent:test".to_string(),
        }
    }

    #[test]
    fn accepted_pattern_is_stored_once() {
        let mut port = InMemoryPatternStore::new();
        let decision = ingest_pattern(&mut port, &mk_request("p-1", 0.8));
        assert_eq!(decision.verdict, PatternIngestVerdict::Accepted);
        assert!(!decision.duplicate);
        assert_eq!(port.len(), 1);
    }

    #[test]
    fn reingest_same_digest_is_duplicate_and_not_stored_twice() {
        let mut port = InMemoryPatternStore::new();
        let request = mk_request("p-2", 0.8);
        let first = ingest_pattern(&mut port, &request);
        assert_eq!(first.verdict, PatternIngestVerdict::Accepted);
        let second = ingest_pattern(&mut port, &request);
        assert_eq!(second.verdict, PatternIngestVerdict::Accepted);
        assert!(second.duplicate);
        assert_eq!(port.len(), 1);
    }

    #[test]
    fn rejected_pattern_is_never_stored() {
        let mut port = InMemoryPatternStore::new();
        let decision = ingest_pattern(
            &mut port,
            &mk_request("p-3", 0.9).clone_with_empty_statement(),
        );
        assert_eq!(decision.verdict, PatternIngestVerdict::Rejected);
        assert!(port.is_empty());
    }

    #[test]
    fn review_pattern_is_never_stored() {
        let mut port = InMemoryPatternStore::new();
        let decision = ingest_pattern(&mut port, &mk_request("p-4", 0.2));
        assert_eq!(decision.verdict, PatternIngestVerdict::NeedsReview);
        assert!(port.is_empty());
    }

    trait WithEmptyStatement {
        fn clone_with_empty_statement(&self) -> Self;
    }

    impl WithEmptyStatement for PatternIngestRequest {
        fn clone_with_empty_statement(&self) -> Self {
            let mut cloned = self.clone();
            cloned.pattern.statement = String::new();
            cloned.pattern.digest =
                cynic_protocol::pattern_distill::pattern_digest(&cloned.pattern);
            cloned
        }
    }
}
