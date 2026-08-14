use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use super::types::{
    RUNTIME_PROOF_BUNDLE_SCHEMA, RUNTIME_PROOF_PERSISTENCE_SCHEMA, RUNTIME_PROOF_SCHEMA,
};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProofPersistenceRecord {
    pub schema: String,
    pub node: String,
    pub revision: String,
    pub timestamp_unix_ms: u128,
    pub status: String,
    pub unknowns: Vec<String>,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub bundle: RuntimeProofBundle,
}

pub fn runtime_proof_persistence_record(
    node: String,
    revision: String,
    timestamp_unix_ms: u128,
    status: String,
    unknowns: Vec<String>,
    blockers: Vec<String>,
    evidence_refs: Vec<String>,
    bundle: RuntimeProofBundle,
) -> RuntimeProofPersistenceRecord {
    RuntimeProofPersistenceRecord {
        schema: RUNTIME_PROOF_PERSISTENCE_SCHEMA.to_string(),
        node,
        revision,
        timestamp_unix_ms,
        status,
        unknowns,
        blockers,
        evidence_refs,
        bundle,
    }
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

    pub fn validate_for_subject(
        &self,
        subject: &str,
        now_unix_ms: u128,
        max_age_ms: u128,
        future_tolerance_ms: u128,
    ) -> Result<(), String> {
        if self.schema != RUNTIME_PROOF_BUNDLE_SCHEMA {
            return Err("runtime proof bundle schema is not canonical".into());
        }
        if self.status != "satisfied" {
            return Err("runtime proof bundle status must be satisfied".into());
        }
        if self.proofs.is_empty() {
            return Err("runtime proof bundle must contain at least one proof".into());
        }
        let mut gates = BTreeSet::new();
        for proof in &self.proofs {
            if !proof.gate_satisfied() {
                return Err(format!("proof gate `{}` is not satisfied", proof.gate));
            }
            if proof.producer.is_empty() || proof.subject != subject {
                return Err(format!(
                    "proof gate `{}` has an invalid producer or subject",
                    proof.gate
                ));
            }
            if !gates.insert(proof.gate.as_str()) {
                return Err(format!("proof gate `{}` is duplicated", proof.gate));
            }
            if proof.observed_at_unix_ms == 0
                || proof.observed_at_unix_ms.saturating_add(max_age_ms) < now_unix_ms
                || proof.observed_at_unix_ms > now_unix_ms.saturating_add(future_tolerance_ms)
            {
                return Err(format!(
                    "proof gate `{}` is stale or future-dated",
                    proof.gate
                ));
            }
            if proof.evidence.is_empty()
                || proof.evidence.iter().any(|evidence| {
                    evidence.id.is_empty()
                        || evidence.status.is_empty()
                        || evidence.detail.is_empty()
                })
            {
                return Err(format!(
                    "proof gate `{}` has missing or incomplete evidence",
                    proof.gate
                ));
            }
        }
        Ok(())
    }

    pub fn normalized_json(&self) -> Result<String, serde_json::Error> {
        let mut normalized = self.clone();
        normalized.proofs.sort_by(|left, right| {
            left.gate
                .cmp(&right.gate)
                .then(left.producer.cmp(&right.producer))
        });
        for proof in &mut normalized.proofs {
            proof.evidence.sort_by(|left, right| left.id.cmp(&right.id));
        }
        serde_json::to_string(&normalized)
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
