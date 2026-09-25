use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::digest_json;

pub const EVIDENCE_ENVELOPE_SCHEMA: &str = "cynic.evidence.envelope.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
<<<<<<< HEAD
pub struct EvidenceRef {
    pub kind: String,
    pub subject: String,
    pub digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<EvidenceLineage>,
}

/// Content-addressed provenance shared by runtime, host, node, and agent evidence.
/// Distinct producers do not imply independent evidence when their origins,
/// derivations, or influences overlap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceLineage {
    pub producer_id: String,
    pub producer_attestation_digest: String,
    #[serde(default)]
    pub origin_digests: Vec<String>,
    #[serde(default)]
    pub derivation_digests: Vec<String>,
    #[serde(default)]
    pub influence_digests: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "relation", rename_all = "snake_case")]
pub enum EvidenceRelation {
    Independent,
    Correlated { reasons: Vec<String> },
    Unknown { missing_evidence: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "dependency", rename_all = "snake_case")]
pub enum EvidenceDependency {
    SameEvidence,
    Derived,
    Influenced,
    Unrelated,
    Unknown { missing_evidence: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
=======
>>>>>>> github/main
pub struct EvidenceEnvelope {
    pub schema: String,
    pub digest: String,
    pub payload_digest: String,
    pub subject: String,
    pub producer: String,
    pub observed_at_unix_ms: u128,
    pub provenance: Vec<String>,
<<<<<<< HEAD
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<EvidenceLineage>,
=======
>>>>>>> github/main
    pub payload: Value,
}

pub fn build_evidence_envelope<T: Serialize>(
    subject: &str,
    producer: &str,
    observed_at_unix_ms: u128,
    provenance: Vec<String>,
    payload: &T,
) -> Result<EvidenceEnvelope, String> {
<<<<<<< HEAD
    build_evidence_envelope_with_lineage(
        subject,
        producer,
        observed_at_unix_ms,
        provenance,
        None,
        payload,
    )
}

pub fn build_evidence_envelope_with_lineage<T: Serialize>(
    subject: &str,
    producer: &str,
    observed_at_unix_ms: u128,
    provenance: Vec<String>,
    lineage: Option<EvidenceLineage>,
    payload: &T,
) -> Result<EvidenceEnvelope, String> {
=======
>>>>>>> github/main
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
<<<<<<< HEAD
    let mut identity = serde_json::json!({
=======
    let identity = serde_json::json!({
>>>>>>> github/main
        "schema": EVIDENCE_ENVELOPE_SCHEMA,
        "payload_digest": payload_digest,
        "subject": subject.trim(),
        "producer": producer.trim(),
        "observed_at_unix_ms": observed_at_unix_ms,
        "provenance": provenance,
    });
<<<<<<< HEAD
    if let Some(lineage) = &lineage {
        let lineage_value = serde_json::to_value(lineage)
            .map_err(|error| format!("serialize evidence lineage: {error}"))?;
        identity
            .as_object_mut()
            .ok_or_else(|| "evidence_identity_not_object".to_string())?
            .insert("lineage".to_string(), lineage_value);
    }
=======
>>>>>>> github/main
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
<<<<<<< HEAD
        lineage,
=======
>>>>>>> github/main
        payload,
    })
}

<<<<<<< HEAD
pub fn relate_evidence(left: &EvidenceRef, right: &EvidenceRef) -> EvidenceRelation {
    let invalid_evidence = invalid_evidence_digest("left", left)
        .into_iter()
        .chain(invalid_evidence_digest("right", right))
        .collect::<Vec<_>>();
    if !invalid_evidence.is_empty() {
        return EvidenceRelation::Unknown {
            missing_evidence: invalid_evidence,
        };
    }
    if left.digest == right.digest {
        return EvidenceRelation::Correlated {
            reasons: vec!["same_evidence".to_string()],
        };
    }

    let missing_evidence = evidence_lineage_unknowns("left", left)
        .into_iter()
        .chain(evidence_lineage_unknowns("right", right))
        .collect::<Vec<_>>();
    if !missing_evidence.is_empty() {
        return EvidenceRelation::Unknown { missing_evidence };
    }

    let Some(left_lineage) = left.lineage.as_ref() else {
        return EvidenceRelation::Unknown {
            missing_evidence: vec!["left.lineage".to_string()],
        };
    };
    let Some(right_lineage) = right.lineage.as_ref() else {
        return EvidenceRelation::Unknown {
            missing_evidence: vec!["right.lineage".to_string()],
        };
    };
    let mut reasons = Vec::new();
    if left_lineage.producer_id == right_lineage.producer_id {
        reasons.push("same_producer".to_string());
    }
    if left_lineage.producer_attestation_digest == right_lineage.producer_attestation_digest {
        reasons.push("shared_producer_attestation".to_string());
    }
    if shares_digest(&left_lineage.origin_digests, &right_lineage.origin_digests) {
        reasons.push("shared_origin".to_string());
    }
    if shares_digest(
        &left_lineage.derivation_digests,
        &right_lineage.derivation_digests,
    ) {
        reasons.push("shared_derivation".to_string());
    }
    if shares_digest(
        &left_lineage.influence_digests,
        &right_lineage.influence_digests,
    ) {
        reasons.push("shared_influence".to_string());
    }
    if left_lineage.derivation_digests.contains(&right.digest)
        || right_lineage.derivation_digests.contains(&left.digest)
    {
        reasons.push("direct_derivation".to_string());
    }
    if left_lineage.influence_digests.contains(&right.digest)
        || right_lineage.influence_digests.contains(&left.digest)
    {
        reasons.push("cross_influence".to_string());
    }

    if reasons.is_empty() {
        EvidenceRelation::Independent
    } else {
        EvidenceRelation::Correlated { reasons }
    }
}

pub fn classify_evidence_dependency(
    evidence: &EvidenceRef,
    candidate_ancestor_digest: &str,
) -> EvidenceDependency {
    if !valid_sha256_digest(candidate_ancestor_digest) {
        return EvidenceDependency::Unknown {
            missing_evidence: vec!["candidate_ancestor.valid_sha256_digest".to_string()],
        };
    }
    if let Some(missing_evidence) = invalid_evidence_digest("evidence", evidence) {
        return EvidenceDependency::Unknown {
            missing_evidence: vec![missing_evidence],
        };
    }
    if evidence.digest == candidate_ancestor_digest {
        return EvidenceDependency::SameEvidence;
    }

    let missing_evidence = evidence_lineage_unknowns("evidence", evidence);
    if !missing_evidence.is_empty() {
        return EvidenceDependency::Unknown { missing_evidence };
    }
    let Some(lineage) = evidence.lineage.as_ref() else {
        return EvidenceDependency::Unknown {
            missing_evidence: vec!["evidence.lineage".to_string()],
        };
    };
    if lineage
        .influence_digests
        .iter()
        .any(|digest| digest == candidate_ancestor_digest)
    {
        EvidenceDependency::Influenced
    } else if lineage
        .derivation_digests
        .iter()
        .chain(&lineage.origin_digests)
        .any(|digest| digest == candidate_ancestor_digest)
    {
        EvidenceDependency::Derived
    } else {
        EvidenceDependency::Unrelated
    }
}

pub(crate) fn evidence_lineage_unknowns(prefix: &str, evidence: &EvidenceRef) -> Vec<String> {
    let Some(lineage) = evidence.lineage.as_ref() else {
        return vec![format!("{prefix}.lineage")];
    };
    let mut unknowns = Vec::new();
    if lineage.producer_id.trim().is_empty() {
        unknowns.push(format!("{prefix}.lineage.producer_id"));
    }
    if !valid_sha256_digest(&lineage.producer_attestation_digest) {
        unknowns.push(format!("{prefix}.lineage.producer_attestation_digest"));
    }
    if lineage.origin_digests.is_empty()
        || lineage
            .origin_digests
            .iter()
            .any(|digest| !valid_sha256_digest(digest))
    {
        unknowns.push(format!("{prefix}.lineage.origin_digests"));
    }
    if lineage
        .derivation_digests
        .iter()
        .any(|digest| !valid_sha256_digest(digest))
    {
        unknowns.push(format!("{prefix}.lineage.derivation_digests"));
    }
    if lineage
        .influence_digests
        .iter()
        .any(|digest| !valid_sha256_digest(digest))
    {
        unknowns.push(format!("{prefix}.lineage.influence_digests"));
    }
    unknowns
}

fn invalid_evidence_digest(prefix: &str, evidence: &EvidenceRef) -> Option<String> {
    (!valid_sha256_digest(&evidence.digest)).then(|| format!("{prefix}.valid_sha256_digest"))
}

fn shares_digest(left: &[String], right: &[String]) -> bool {
    left.iter().any(|digest| right.contains(digest))
}

pub fn valid_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

=======
>>>>>>> github/main
#[cfg(test)]
mod tests {
    use super::*;

<<<<<<< HEAD
    fn digest(marker: char) -> String {
        format!("sha256:{}", marker.to_string().repeat(64))
    }

    fn evidence(marker: char, producer: char, origin: char) -> EvidenceRef {
        EvidenceRef {
            kind: "runtime_proof".to_string(),
            subject: "local-kernel/process".to_string(),
            digest: digest(marker),
            lineage: Some(EvidenceLineage {
                producer_id: format!("producer-{producer}"),
                producer_attestation_digest: digest(producer),
                origin_digests: vec![digest(origin)],
                derivation_digests: Vec::new(),
                influence_digests: Vec::new(),
            }),
        }
    }

=======
>>>>>>> github/main
    #[test]
    fn payload_identity_is_stable_while_observation_identity_changes() {
        let first = build_evidence_envelope("node", "probe", 1, vec!["p".into()], &42).unwrap();
        let second = build_evidence_envelope("node", "probe", 2, vec!["p".into()], &42).unwrap();
        assert_eq!(first.payload_digest, second.payload_digest);
        assert_ne!(first.digest, second.digest);
    }
<<<<<<< HEAD

    #[test]
    fn same_origin_is_correlated_across_runtime_producers() {
        let left = evidence('1', 'a', 'c');
        let right = evidence('2', 'b', 'c');

        assert!(matches!(
            relate_evidence(&left, &right),
            EvidenceRelation::Correlated { reasons } if reasons == vec!["shared_origin"]
        ));
    }

    #[test]
    fn lineage_is_bound_into_envelope_identity() {
        let first = build_evidence_envelope_with_lineage(
            "local-kernel/process",
            "cynic-host",
            1,
            vec!["procfs".into()],
            evidence('1', 'a', 'c').lineage,
            &42,
        )
        .unwrap();
        let second = build_evidence_envelope_with_lineage(
            "local-kernel/process",
            "cynic-host",
            1,
            vec!["procfs".into()],
            evidence('1', 'b', 'c').lineage,
            &42,
        )
        .unwrap();

        assert_eq!(first.payload_digest, second.payload_digest);
        assert_ne!(first.digest, second.digest);
    }
=======
>>>>>>> github/main
}
