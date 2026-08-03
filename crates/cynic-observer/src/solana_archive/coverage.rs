use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageGap {
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    InProgress,
    Exhausted,
    Incomplete,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageLedger {
    pub target: String,
    pub cluster: String,
    pub requested_start_slot: Option<u64>,
    pub requested_end_slot: Option<u64>,
    pub pages: u64,
    pub observations_seen: u64,
    pub unique_signatures: u64,
    pub next_cursor: Option<String>,
    pub status: CoverageStatus,
    pub gaps: Vec<CoverageGap>,
    #[serde(skip)]
    signatures: BTreeSet<String>,
}

impl CoverageLedger {
    pub fn new(target: impl Into<String>, cluster: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            cluster: cluster.into(),
            requested_start_slot: None,
            requested_end_slot: None,
            pages: 0,
            observations_seen: 0,
            unique_signatures: 0,
            next_cursor: None,
            status: CoverageStatus::InProgress,
            gaps: Vec::new(),
            signatures: BTreeSet::new(),
        }
    }

    pub fn record_page<I, S>(&mut self, signatures: I, next_cursor: Option<S>)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.pages += 1;
        for signature in signatures {
            self.observations_seen += 1;
            self.signatures.insert(signature.into());
        }
        self.unique_signatures = self.signatures.len() as u64;
        self.next_cursor = next_cursor.map(Into::into);
    }

    pub fn record_observation(&mut self, signature: impl Into<String>) {
        self.observations_seen += 1;
        self.signatures.insert(signature.into());
        self.unique_signatures = self.signatures.len() as u64;
    }

    pub fn mark_exhausted(&mut self) {
        self.next_cursor = None;
        self.status = if self.gaps.is_empty() {
            CoverageStatus::Exhausted
        } else {
            CoverageStatus::Incomplete
        };
    }

    pub fn record_gap(&mut self, code: impl Into<String>, detail: impl Into<String>) {
        self.gaps.push(CoverageGap {
            code: code.into(),
            detail: detail.into(),
        });
        self.status = CoverageStatus::Incomplete;
    }

    pub fn contains_signature(&self, signature: &str) -> bool {
        self.signatures.contains(signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_counts_unique_signatures_and_duplicates() {
        let mut ledger = CoverageLedger::new("mint", "mainnet");
        ledger.record_page(["a", "b"], Some("b"));
        ledger.record_page(["b", "c"], None::<&str>);
        ledger.mark_exhausted();

        assert_eq!(ledger.observations_seen, 4);
        assert_eq!(ledger.unique_signatures, 3);
        assert_eq!(ledger.status, CoverageStatus::Exhausted);
        assert!(ledger.contains_signature("c"));
    }

    #[test]
    fn gaps_prevent_exhausted_claim() {
        let mut ledger = CoverageLedger::new("mint", "mainnet");
        ledger.record_gap("malformed_response", "provider returned invalid JSON");
        ledger.mark_exhausted();

        assert_eq!(ledger.status, CoverageStatus::Incomplete);
    }
}
