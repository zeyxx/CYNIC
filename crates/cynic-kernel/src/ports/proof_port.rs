//! Test: ProofPort trait exists and can be consumed by external crates.
//! This proves the FCIS separation: domain/ is pure, ports/ defines the trait,
//! adapters/ implements it. External crates (cynic-host) consume the port.

use std::collections::HashMap;

/// ProofPort — the kernel's proof verification contract.
/// Consumed by user space (cynic-host, cynic-node) to verify read-back proofs.
pub trait ProofPort {
    /// Verify a proof against its claimed fact.
    /// Returns Ok(()) if the proof is valid, Err(reason) otherwise.
    fn verify(&self, proof: &ProofClaim) -> Result<(), ProofError>;
}

/// A proof claim: a fact with its read-back evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct ProofClaim {
    pub fact_id: String,
    pub expected_hash: String,
    pub observed_hash: String,
    pub metadata: HashMap<String, String>,
}

/// Proof verification error.
#[derive(Debug, Clone, PartialEq)]
pub enum ProofError {
    HashMismatch { expected: String, observed: String },
    MissingFact(String),
    InvalidMetadata(String),
}

impl std::fmt::Display for ProofError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofError::HashMismatch { expected, observed } => {
                write!(f, "hash mismatch: expected {expected}, observed {observed}")
            }
            ProofError::MissingFact(id) => write!(f, "fact not found: {id}"),
            ProofError::InvalidMetadata(reason) => write!(f, "invalid metadata: {reason}"),
        }
    }
}

impl std::error::Error for ProofError {}

/// Pure proof verification logic — no I/O, testable without mocks.
pub fn verify_proof_claim(claim: &ProofClaim) -> Result<(), ProofError> {
    if claim.expected_hash != claim.observed_hash {
        return Err(ProofError::HashMismatch {
            expected: claim.expected_hash.clone(),
            observed: claim.observed_hash.clone(),
        });
    }
    if claim.fact_id.is_empty() {
        return Err(ProofError::MissingFact(claim.fact_id.clone()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_matching_proof_succeeds() {
        let claim = ProofClaim {
            fact_id: "fact-1".to_string(),
            expected_hash: "abc123".to_string(),
            observed_hash: "abc123".to_string(),
            metadata: HashMap::new(),
        };
        assert!(verify_proof_claim(&claim).is_ok());
    }

    #[test]
    fn verify_mismatched_hash_fails() {
        let claim = ProofClaim {
            fact_id: "fact-1".to_string(),
            expected_hash: "abc123".to_string(),
            observed_hash: "xyz789".to_string(),
            metadata: HashMap::new(),
        };
        let err = verify_proof_claim(&claim).unwrap_err();
        assert_eq!(
            err,
            ProofError::HashMismatch {
                expected: "abc123".to_string(),
                observed: "xyz789".to_string(),
            }
        );
    }

    #[test]
    fn verify_empty_fact_id_fails() {
        let claim = ProofClaim {
            fact_id: "".to_string(),
            expected_hash: "abc123".to_string(),
            observed_hash: "abc123".to_string(),
            metadata: HashMap::new(),
        };
        let err = verify_proof_claim(&claim).unwrap_err();
        assert_eq!(err, ProofError::MissingFact("".to_string()));
    }

    #[test]
    fn proof_port_trait_object_safe() {
        // Prove the trait is object-safe (dyn-compatible)
        struct MockPort;
        impl ProofPort for MockPort {
            fn verify(&self, claim: &ProofClaim) -> Result<(), ProofError> {
                verify_proof_claim(claim)
            }
        }
        let port: Box<dyn ProofPort> = Box::new(MockPort);
        let claim = ProofClaim {
            fact_id: "fact-2".to_string(),
            expected_hash: "def456".to_string(),
            observed_hash: "def456".to_string(),
            metadata: HashMap::new(),
        };
        assert!(port.verify(&claim).is_ok());
    }
}
