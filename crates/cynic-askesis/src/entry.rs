//! Askesis entry types and content-addressing hashing.

use serde::{Deserialize, Serialize};

pub const ASKESIS_ENTRY_SCHEMA: &str = "cynic.askesis_entry.v1";
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

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
        sha256_hex(&bytes)
    }

    /// Verify entry integrity: recompute and compare.
    pub fn verify(&self) -> bool {
        self.entry_hash == self.compute_hash()
    }
}

pub(crate) fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}
