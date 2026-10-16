//! Askesis entry types and content-addressing hashing.
//!
//! Uses the unified digest law from `cynic_protocol::digest` (SHA-256).

use serde::{Deserialize, Serialize};

use cynic_protocol::digest::digest_bytes;

pub const ASKESIS_ENTRY_SCHEMA: &str = "cynic.askesis_entry.v1";
pub const GENESIS_HASH: &str = "genesis";

/// Askesis entry kind — what category of proof this entry records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AskesisKind {
    /// Constitutional court audit report
    CourtAudit,
    /// Forensics SOC observation
    Forensics,
    /// Release artifact proof
    ReleaseProof,
    /// Agent action evidence
    AgentEvidence,
    /// Custom (extension point)
    Custom,
}

impl AskesisKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CourtAudit => "court_audit",
            Self::Forensics => "forensics",
            Self::ReleaseProof => "release_proof",
            Self::AgentEvidence => "agent_evidence",
            Self::Custom => "custom",
        }
    }
}

impl std::fmt::Display for AskesisKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// One immutable, content-addressed, hash-chained entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskesisEntry {
    /// Schema version
    pub schema: String,
    /// Entry kind
    pub kind: AskesisKind,
    /// Logical timestamp (unix seconds, observed)
    pub observed_at_unix_s: u64,
    /// Entry sequence (1-indexed, monotonic)
    pub sequence: u64,
    /// Hash of the previous entry (all zeros for genesis)
    pub prev_hash: String,
    /// Content-addressed hash of this entry (SHA-256)
    pub entry_hash: String,
    /// Subject (what this evidence is about)
    pub subject: String,
    /// Evidence payload (kind-specific)
    pub evidence: serde_json::Value,
}

impl AskesisEntry {
    pub fn new(
        kind: AskesisKind,
        observed_at_unix_s: u64,
        sequence: u64,
        prev_hash: impl Into<String>,
        subject: impl Into<String>,
        evidence: serde_json::Value,
    ) -> Self {
        let mut entry = Self {
            schema: ASKESIS_ENTRY_SCHEMA.to_string(),
            kind,
            observed_at_unix_s,
            sequence,
            prev_hash: prev_hash.into(),
            subject: subject.into(),
            evidence,
            entry_hash: String::new(),
        };
        entry.entry_hash = entry.compute_hash();
        entry
    }

    /// Compute content-addressed hash (SHA-256 of canonical fields).
    ///
    /// Uses the unified digest law (`digest_bytes`).
    pub fn compute_hash(&self) -> String {
        let canonical = serde_json::json!({
            "schema": self.schema,
            "kind": self.kind.as_str(),
            "observed_at_unix_s": self.observed_at_unix_s,
            "sequence": self.sequence,
            "prev_hash": self.prev_hash,
            "subject": self.subject,
            "evidence": self.evidence,
        });
        let bytes = canonical.to_string();
        digest_bytes(bytes.as_bytes())
    }

    /// Verify entry integrity: recompute and compare.
    pub fn verify(&self) -> bool {
        self.entry_hash == self.compute_hash()
    }
}


