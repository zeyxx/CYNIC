use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::digest_json;

pub const EVIDENCE_ENVELOPE_SCHEMA: &str = "cynic.evidence.envelope.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceEnvelope {
    pub schema: String,
    pub digest: String,
    pub payload_digest: String,
    pub subject: String,
    pub producer: String,
    pub observed_at_unix_ms: u128,
    pub provenance: Vec<String>,
    pub payload: Value,
}

pub fn build_evidence_envelope<T: Serialize>(
    subject: &str,
    producer: &str,
    observed_at_unix_ms: u128,
    provenance: Vec<String>,
    payload: &T,
) -> Result<EvidenceEnvelope, String> {
    if subject.trim().is_empty() {
        return Err("evidence_subject_missing".to_string());
    }
    if producer.trim().is_empty() {
        return Err("evidence_producer_missing".to_string());
    }
    if provenance.is_empty() {
        return Err("evidence_provenance_missing".to_string());
    }
    let payload = serde_json::to_value(payload)
        .map_err(|error| format!("serialize evidence payload: {error}"))?;
    let payload_digest =
        digest_json(&payload).map_err(|error| format!("digest evidence payload: {error}"))?;
    let identity = serde_json::json!({
        "schema": EVIDENCE_ENVELOPE_SCHEMA,
        "payload_digest": payload_digest,
        "subject": subject.trim(),
        "producer": producer.trim(),
        "observed_at_unix_ms": observed_at_unix_ms,
        "provenance": provenance,
    });
    let digest =
        digest_json(&identity).map_err(|error| format!("digest evidence identity: {error}"))?;
    Ok(EvidenceEnvelope {
        schema: EVIDENCE_ENVELOPE_SCHEMA.to_string(),
        digest,
        payload_digest,
        subject: subject.trim().to_string(),
        producer: producer.trim().to_string(),
        observed_at_unix_ms,
        provenance,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_identity_is_stable_while_observation_identity_changes() {
        let first = build_evidence_envelope("node", "probe", 1, vec!["p".into()], &42).unwrap();
        let second = build_evidence_envelope("node", "probe", 2, vec!["p".into()], &42).unwrap();
        assert_eq!(first.payload_digest, second.payload_digest);
        assert_ne!(first.digest, second.digest);
    }
}
