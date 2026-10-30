use serde::{Deserialize, Serialize};

use super::*;
use crate::evidence;
use serde_json;

// ── Coordination error codes ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofKind {
    Claim,
    Heartbeat,
    Release,
    Mutation,
    Conflict,
    Sync,
}

/// Envelope for any coordination proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofEnvelope {
    pub schema: String,
    pub proof_id: String,
    pub kind: ProofKind,
    pub subject_id: String,
    pub produced_at_unix_ms: u64,
    pub producer_id: String,
    pub evidence_refs: Vec<String>,
    pub digest: String,
}

/// Proof emitted after a successful claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimProof {
    pub schema: String,
    pub claim_id: String,
    pub lease_id: String,
    pub evidence_refs: Vec<String>,
    pub proven_at_unix_ms: u64,
}

/// Proof emitted after a successful release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseProof {
    pub schema: String,
    pub lease_id: String,
    pub released_at_unix_ms: u64,
    pub proof_refs: Vec<String>,
    pub final_state_digest: String,
}

/// Proof emitted after a successful heartbeat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatProof {
    pub schema: String,
    pub lease_id: String,
    pub previous_expiry_ms: u64,
    pub new_expiry_ms: u64,
    pub beats_at_unix_ms: u64,
}

/// Proof emitted after each bounded mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationProof {
    pub schema: String,
    pub claim_id: String,
    pub mutation_id: String,
    pub target_scope: ClaimScope,
    pub argv: Vec<String>,
    pub exit_code: i32,
    pub evidence_refs: Vec<String>,
    pub mutated_at_unix_ms: u64,
}

// ── Conversion to canonical EvidenceEnvelope ─────────────────────────────

impl ProofEnvelope {
    pub fn into_evidence_envelope(self) -> Result<evidence::EvidenceEnvelope, String> {
        let provenance = if self.evidence_refs.is_empty() {
            vec!["cynic.agent_coordination".to_string()]
        } else {
            self.evidence_refs.clone()
        };
        evidence::build_evidence_envelope(
            &self.subject_id,
            &self.producer_id,
            self.produced_at_unix_ms as u128,
            provenance,
            &self,
        )
    }
}

impl TryFrom<evidence::EvidenceEnvelope> for ProofEnvelope {
    type Error = String;

    fn try_from(env: evidence::EvidenceEnvelope) -> Result<Self, Self::Error> {
        serde_json::from_value(env.payload)
            .map_err(|error| format!("decode ProofEnvelope from evidence payload: {error}"))
    }
}

impl ClaimProof {
    pub fn into_evidence_envelope(
        &self,
        producer: &str,
    ) -> Result<evidence::EvidenceEnvelope, String> {
        let provenance = if self.evidence_refs.is_empty() {
            vec!["cynic.agent_coordination".to_string()]
        } else {
            self.evidence_refs.clone()
        };
        evidence::build_evidence_envelope(
            &self.claim_id,
            producer,
            self.proven_at_unix_ms as u128,
            provenance,
            self,
        )
    }
}

impl ReleaseProof {
    pub fn into_evidence_envelope(
        &self,
        producer: &str,
    ) -> Result<evidence::EvidenceEnvelope, String> {
        let provenance = if self.proof_refs.is_empty() {
            vec!["cynic.agent_coordination".to_string()]
        } else {
            self.proof_refs.clone()
        };
        evidence::build_evidence_envelope(
            &self.lease_id,
            producer,
            self.released_at_unix_ms as u128,
            provenance,
            self,
        )
    }
}

impl HeartbeatProof {
    pub fn into_evidence_envelope(
        &self,
        producer: &str,
    ) -> Result<evidence::EvidenceEnvelope, String> {
        evidence::build_evidence_envelope(
            &self.lease_id,
            producer,
            self.beats_at_unix_ms as u128,
            vec!["cynic.agent_coordination".to_string()],
            self,
        )
    }
}

impl MutationProof {
    pub fn into_evidence_envelope(
        &self,
        producer: &str,
    ) -> Result<evidence::EvidenceEnvelope, String> {
        let provenance = if self.evidence_refs.is_empty() {
            vec!["cynic.agent_coordination".to_string()]
        } else {
            self.evidence_refs.clone()
        };
        evidence::build_evidence_envelope(
            &self.mutation_id,
            producer,
            self.mutated_at_unix_ms as u128,
            provenance,
            self,
        )
    }
}
