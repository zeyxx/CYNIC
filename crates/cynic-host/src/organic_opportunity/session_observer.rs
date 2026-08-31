//! Session pattern opportunity observer.
//!
//! Observes agent session traces to surface recurring friction opportunities.
//!
//! # Privacy & Minimisation Invariant
//! Raw session prompts, tool arguments, and assistant responses are NEVER
//! included in the emitted `OrganicOpportunity`. Only bucket identifiers,
//! session occurrence counts, and content-addressed trace digests are retained.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use cynic_protocol::organic_opportunity::{
    EnergyEstimate, ORGANIC_OPPORTUNITY_SCHEMA, OpportunityClass, OpportunitySource,
    OpportunitySourceKind, OrganicOpportunity, SmallestNextStep,
};

use super::port::{OrganicOpportunityObserver, SourceObservationStatus};

/// Configuration for session pattern observation.
#[derive(Debug, Clone)]
pub struct SessionObserverConfig {
    /// Injected root directory where session traces live.
    pub traces_dir: PathBuf,
    /// Minimum distinct sessions a friction pattern must appear in to become an opportunity.
    pub min_recurring_sessions: usize,
}

impl Default for SessionObserverConfig {
    fn default() -> Self {
        Self {
            traces_dir: PathBuf::from(".hermes/sessions"),
            min_recurring_sessions: 2,
        }
    }
}

/// Host adapter observing session trace files without retaining raw text.
pub struct SessionPatternObserver {
    config: SessionObserverConfig,
}

impl SessionPatternObserver {
    pub fn new(config: SessionObserverConfig) -> Self {
        Self { config }
    }

    /// Pure parser: turns parsed session line summaries into opportunities.
    /// Injected with structured inputs to guarantee testability without filesystem IO.
    pub fn analyze_session_buckets(
        session_hits: &[(String, String)], // (session_id, bucket_name)
        min_recurring_sessions: usize,
        now_unix_s: u64,
    ) -> Vec<OrganicOpportunity> {
        let mut bucket_sessions: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for (session_id, bucket) in session_hits {
            bucket_sessions
                .entry(bucket.as_str())
                .or_default()
                .insert(session_id.as_str());
        }

        let mut opportunities = Vec::new();

        for (bucket, sessions) in bucket_sessions {
            if sessions.len() >= min_recurring_sessions {
                let digest = cynic_protocol::digest::digest_json(&serde_json::json!({
                    "bucket": bucket,
                    "session_count": sessions.len(),
                }))
                .unwrap_or_else(|_| format!("sha256:{}", bucket));

                let opp = OrganicOpportunity {
                    schema: ORGANIC_OPPORTUNITY_SCHEMA.to_string(),
                    opportunity_id: format!("opp:session_friction:{}", &digest[..16]),
                    scope: format!("session_friction/{}", bucket),
                    sources: vec![OpportunitySource {
                        kind: OpportunitySourceKind::SessionTrace,
                        ref_id: format!("pattern_bucket:{}", bucket),
                        label: Some(format!("recurring in {} sessions", sessions.len())),
                    }],
                    class: OpportunityClass::RecurringFriction,
                    expected_unlock: format!(
                        "Resolve recurring friction in '{}' across {} observed sessions",
                        bucket,
                        sessions.len()
                    ),
                    estimated_energy: EnergyEstimate::Unknown {
                        reason: "session friction repair energy depends on root cause diagnosis"
                            .to_string(),
                    },
                    freshness_seconds: 0, // young/live aggregate
                    confidence: (sessions.len() as f64 / (sessions.len() as f64 + 2.0)).min(0.95), // bounded confidence, < 1.0 (Wisdom rule)
                    smallest_next_step: SmallestNextStep::Observe,
                    residual_unknowns: vec![
                        "root_cause_unisolated".to_string(),
                        format!("observed_in_{}_sessions", sessions.len()),
                    ],
                    operator_disposition: Default::default(),
                };

                opportunities.push(opp);
            }
        }

        opportunities
    }

    /// Read trace files from a directory and extract bucket hits.
    fn scan_traces_dir(dir: &Path) -> Result<Vec<(String, String)>, String> {
        if !dir.exists() {
            return Err(format!(
                "traces directory does not exist: {}",
                dir.display()
            ));
        }

        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("failed to read traces directory {}: {}", dir.display(), e))?;

        let mut hits = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                let session_id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                if let Ok(file) = std::fs::File::open(&path) {
                    use std::io::{BufRead, BufReader};
                    let reader = BufReader::new(file);
                    for line in reader.lines().map_while(Result::ok) {
                        for bucket in crate::bucket_signatures::matching_buckets(&line) {
                            hits.push((session_id.clone(), bucket.to_string()));
                        }
                    }
                }
            }
        }

        Ok(hits)
    }
}

impl OrganicOpportunityObserver for SessionPatternObserver {
    fn observer_name(&self) -> &'static str {
        "session_pattern_observer"
    }

    fn source_kind(&self) -> OpportunitySourceKind {
        OpportunitySourceKind::SessionTrace
    }

    fn observe(&self) -> SourceObservationStatus<Vec<OrganicOpportunity>> {
        let hits = match Self::scan_traces_dir(&self.config.traces_dir) {
            Ok(h) => h,
            Err(reason) => {
                return SourceObservationStatus::Unavailable {
                    kind: OpportunitySourceKind::SessionTrace,
                    reason,
                };
            }
        };

        if hits.is_empty() {
            return SourceObservationStatus::Empty;
        }

        let opps = Self::analyze_session_buckets(&hits, self.config.min_recurring_sessions, 0);

        if opps.is_empty() {
            SourceObservationStatus::Empty
        } else {
            SourceObservationStatus::Observed { data: opps }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_occurrence_below_threshold_is_empty() {
        let hits = vec![("session_1".to_string(), "toolchain_ice".to_string())];
        let opps = SessionPatternObserver::analyze_session_buckets(&hits, 2, 0);
        assert!(opps.is_empty());
    }

    #[test]
    fn recurring_pattern_creates_opportunity_with_redacted_text() {
        let hits = vec![
            ("session_1".to_string(), "toolchain_ice".to_string()),
            ("session_2".to_string(), "toolchain_ice".to_string()),
            ("session_3".to_string(), "toolchain_ice".to_string()),
        ];
        let opps = SessionPatternObserver::analyze_session_buckets(&hits, 2, 0);
        assert_eq!(opps.len(), 1);

        let opp = &opps[0];
        assert_eq!(opp.class, OpportunityClass::RecurringFriction);
        assert_eq!(opp.scope, "session_friction/toolchain_ice");
        assert!(opp.confidence < 1.0); // wisdom rule
        assert!(opp.confidence > 0.5);

        // Verify privacy invariant: no session IDs or raw text in scope/id
        assert!(!opp.opportunity_id.contains("session_1"));
        assert_eq!(opp.sources.len(), 1);
        assert_eq!(opp.sources[0].kind, OpportunitySourceKind::SessionTrace);
        assert_eq!(opp.sources[0].ref_id, "pattern_bucket:toolchain_ice");
    }

    #[test]
    fn unavailable_when_traces_dir_missing() {
        let observer = SessionPatternObserver::new(SessionObserverConfig {
            traces_dir: PathBuf::from("/nonexistent/traces/path/12345"),
            min_recurring_sessions: 2,
        });

        let status = observer.observe();
        assert!(status.is_unavailable());
    }
}
