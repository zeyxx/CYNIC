use serde::{Deserialize, Serialize};

use super::types::RUNTIME_PROOF_SCHEMA;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProofBundle {
    pub schema: String,
    pub status: String,
    pub proofs: Vec<RuntimeProof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProof {
    pub schema: String,
    pub gate: String,
    pub status: String,
    pub producer: String,
    pub subject: String,
    pub observed_at_unix_ms: u128,
    pub evidence: Vec<RuntimeProofEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProofEvidence {
    pub id: String,
    pub status: String,
    pub detail: String,
}

impl RuntimeProof {
    pub fn gate_satisfied(&self) -> bool {
        self.schema == RUNTIME_PROOF_SCHEMA && self.status == "satisfied" && !self.gate.is_empty()
    }
}

impl RuntimeProofBundle {
    pub fn observed_gates(&self) -> Vec<String> {
        let mut gates = self
            .proofs
            .iter()
            .filter(|proof| proof.gate_satisfied())
            .map(|proof| proof.gate.clone())
            .collect::<Vec<_>>();
        gates.sort();
        gates.dedup();
        gates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::types::{RUNTIME_PROOF_BUNDLE_SCHEMA, RUNTIME_PROOF_SCHEMA};

    #[test]
    fn proof_bundle_exposes_only_satisfied_gates() {
        let bundle = RuntimeProofBundle {
            schema: RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
            status: "mixed".to_string(),
            proofs: vec![
                RuntimeProof {
                    schema: RUNTIME_PROOF_SCHEMA.to_string(),
                    gate: "host_identity_observed".to_string(),
                    status: "satisfied".to_string(),
                    producer: "cynic-host".to_string(),
                    subject: "node-a".to_string(),
                    observed_at_unix_ms: 1,
                    evidence: Vec::new(),
                },
                RuntimeProof {
                    schema: RUNTIME_PROOF_SCHEMA.to_string(),
                    gate: "network_runtime_observed".to_string(),
                    status: "blocked".to_string(),
                    producer: "cynic-node".to_string(),
                    subject: "node-a".to_string(),
                    observed_at_unix_ms: 1,
                    evidence: Vec::new(),
                },
            ],
        };

        assert_eq!(bundle.observed_gates(), vec!["host_identity_observed"]);
    }
}
