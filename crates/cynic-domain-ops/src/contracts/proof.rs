//! Proof construction and serialization for domain operations.
//!
//! Builds `DomainOperationProof` envelopes from domain operation results,
//! with deterministic idempotency keys and machine-readable JSON output
//! suitable for archiving in `cynic-askesis`.

use cynic_protocol::domain_ops::{
    DOMAIN_OPS_PROOF_SCHEMA, DomainOperation, DomainOperationProof, ProofStatus,
};

use super::error::DomainOpsError;

/// Build a `DomainOperationProof` envelope.
///
/// The proof is deterministic in structure (schema, domain, operation,
/// idempotency key, provider read-back). The timestamp is provided by
/// the caller to keep this function pure (no `SystemTime` in contracts).
///
/// # Arguments
/// * `domain` — the domain this proof relates to
/// * `operation` — the domain operation type
/// * `status` — proof verdict (Success, Blocked, Violates)
/// * `idempotency_key` — deterministic key from `generate_idempotency_key()`
/// * `provider_read_back` — raw provider API response (never contains secrets)
/// * `timestamp_unix_s` — Unix timestamp of proof emission
pub fn build_proof(
    domain: &str,
    operation: DomainOperation,
    status: ProofStatus,
    idempotency_key: &str,
    provider_read_back: serde_json::Value,
    timestamp_unix_s: u64,
) -> DomainOperationProof {
    DomainOperationProof {
        schema: DOMAIN_OPS_PROOF_SCHEMA.to_string(),
        domain: domain.to_string(),
        operation,
        status,
        idempotency_key: idempotency_key.to_string(),
        timestamp_unix_s,
        provider_read_back,
    }
}

/// Serialize a proof to a pretty-printed JSON string.
///
/// Returns an error if serialization fails (extremely unlikely for
/// `DomainOperationProof` which derives `Serialize`).
pub fn proof_to_json(proof: &DomainOperationProof) -> Result<String, DomainOpsError> {
    serde_json::to_string_pretty(proof).map_err(|e| DomainOpsError::json_parse("proof_to_json", e))
}

/// Serialize multiple proofs as a JSON array string.
///
/// Used at the end of a domain operations run to emit the complete
/// proof chain for archiving or MCP read-back.
pub fn proof_chain_to_json(proofs: &[DomainOperationProof]) -> Result<String, DomainOpsError> {
    serde_json::to_string_pretty(proofs)
        .map_err(|e| DomainOpsError::json_parse("proof_chain_to_json", e))
}

/// Parse a proof JSON string back into a `DomainOperationProof`.
///
/// Validates the schema field matches `DOMAIN_OPS_PROOF_SCHEMA`.
pub fn parse_proof(json: &str) -> Result<DomainOperationProof, DomainOpsError> {
    let proof: DomainOperationProof =
        serde_json::from_str(json).map_err(|e| DomainOpsError::json_parse("proof", e))?;

    if proof.schema != DOMAIN_OPS_PROOF_SCHEMA {
        return Err(DomainOpsError::unexpected_proof_schema(&proof.schema));
    }

    Ok(proof)
}

/// Parse a proof chain (JSON array) into a `Vec<DomainOperationProof>`.
pub fn parse_proof_chain(json: &str) -> Result<Vec<DomainOperationProof>, DomainOpsError> {
    let chain: Vec<DomainOperationProof> =
        serde_json::from_str(json).map_err(|e| DomainOpsError::json_parse("proof_chain", e))?;

    for proof in &chain {
        if proof.schema != DOMAIN_OPS_PROOF_SCHEMA {
            return Err(DomainOpsError::unexpected_proof_schema(&proof.schema));
        }
    }

    Ok(chain)
}

/// Build a "blocked" proof — emitted when an operation cannot proceed
/// (e.g. domain not available, consent not given, provider error).
///
/// The `provider_read_back` carries the error detail for forensic analysis.
pub fn blocked_proof(
    domain: &str,
    operation: DomainOperation,
    idempotency_key: &str,
    reason: &str,
    timestamp_unix_s: u64,
) -> DomainOperationProof {
    build_proof(
        domain,
        operation,
        ProofStatus::Blocked,
        idempotency_key,
        serde_json::json!({ "error": reason }),
        timestamp_unix_s,
    )
}

/// Build a "violation" proof — emitted when an operation completed but
/// read-back shows divergence from the intended state.
pub fn violation_proof(
    domain: &str,
    operation: DomainOperation,
    idempotency_key: &str,
    expected: serde_json::Value,
    observed: serde_json::Value,
    timestamp_unix_s: u64,
) -> DomainOperationProof {
    build_proof(
        domain,
        operation,
        ProofStatus::Violates,
        idempotency_key,
        serde_json::json!({ "expected": expected, "observed": observed }),
        timestamp_unix_s,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::domain_ops::{DomainOperation, ProofStatus};

    #[test]
    fn build_success_proof_roundtrips() {
        let proof = build_proof(
            "example.com",
            DomainOperation::Register,
            ProofStatus::Success,
            "sha3-512:abc123",
            serde_json::json!({ "domain": "example.com", "status": "registered" }),
            1788267750,
        );

        assert_eq!(proof.schema, DOMAIN_OPS_PROOF_SCHEMA);
        assert_eq!(proof.domain, "example.com");
        assert_eq!(proof.operation, DomainOperation::Register);
        assert_eq!(proof.status, ProofStatus::Success);
        assert_eq!(proof.idempotency_key, "sha3-512:abc123");
        assert_eq!(proof.timestamp_unix_s, 1788267750);
    }

    #[test]
    fn proof_to_json_serializes_correctly() {
        let proof = build_proof(
            "example.com",
            DomainOperation::CheckAvailability,
            ProofStatus::Success,
            "sha3-512:abc123",
            serde_json::json!({ "available": true }),
            1788267750,
        );

        let json = proof_to_json(&proof).unwrap();
        assert!(json.contains("cynic.domain_ops.proof.v1"));
        assert!(json.contains("example.com"));
        assert!(json.contains("check_availability"));
        assert!(json.contains("success"));

        // Round-trip: parse back
        let parsed = parse_proof(&json).unwrap();
        assert_eq!(parsed.domain, "example.com");
        assert_eq!(parsed.operation, DomainOperation::CheckAvailability);
        assert_eq!(parsed.status, ProofStatus::Success);
    }

    #[test]
    fn proof_chain_to_json_and_back() {
        let proofs = vec![
            build_proof(
                "example.com",
                DomainOperation::CheckAvailability,
                ProofStatus::Success,
                "sha3-512:key1",
                serde_json::json!({ "available": true }),
                1788267750,
            ),
            build_proof(
                "example.com",
                DomainOperation::Register,
                ProofStatus::Success,
                "sha3-512:key2",
                serde_json::json!({ "status": "registered" }),
                1788267751,
            ),
        ];

        let json = proof_chain_to_json(&proofs).unwrap();
        let parsed = parse_proof_chain(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].operation, DomainOperation::CheckAvailability);
        assert_eq!(parsed[1].operation, DomainOperation::Register);
    }

    #[test]
    fn blocked_proof_carries_error_reason() {
        let proof = blocked_proof(
            "example.com",
            DomainOperation::Register,
            "sha3-512:blocked",
            "domain not available",
            1788267750,
        );

        assert_eq!(proof.status, ProofStatus::Blocked);
        let read_back = proof.provider_read_back.as_object().unwrap();
        assert_eq!(read_back.get("error").unwrap(), "domain not available");
    }

    #[test]
    fn violation_proof_carries_expected_and_observed() {
        let proof = violation_proof(
            "example.com",
            DomainOperation::VerifyEdge,
            "sha3-512:viol",
            serde_json::json!({ "status": 200 }),
            serde_json::json!({ "status": 404 }),
            1788267750,
        );

        assert_eq!(proof.status, ProofStatus::Violates);
        let read_back = proof.provider_read_back.as_object().unwrap();
        assert_eq!(
            read_back.get("expected").unwrap().get("status").unwrap(),
            200
        );
        assert_eq!(
            read_back.get("observed").unwrap().get("status").unwrap(),
            404
        );
    }

    #[test]
    fn parse_proof_rejects_wrong_schema() {
        let json = r#"{
            "schema": "cynic.domain_ops.proof.v0",
            "domain": "example.com",
            "operation": "register",
            "status": "success",
            "idempotency_key": "abc",
            "timestamp_unix_s": 1788267750,
            "provider_read_back": {}
        }"#;
        let result = parse_proof(json);
        assert!(result.is_err());
        match result.unwrap_err() {
            DomainOpsError::UnexpectedProof(s) => {
                assert!(s.contains("v0"));
            }
            _ => panic!("expected UnexpectedProof error"),
        }
    }

    #[test]
    fn parse_proof_rejects_malformed_json() {
        let result = parse_proof("{ broken json");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DomainOpsError::JsonParse { .. }
        ));
    }
}
