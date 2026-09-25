<<<<<<< HEAD
pub type AgentEvidenceRef = crate::evidence::EvidenceRef;
pub type AgentEvidenceLineage = crate::evidence::EvidenceLineage;
pub type AgentEvidenceRelation = crate::evidence::EvidenceRelation;
pub type AgentEvidenceDependency = crate::evidence::EvidenceDependency;
=======
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentEvidenceRef {
    pub kind: String,
    pub subject: String,
    pub digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<AgentEvidenceLineage>,
}

/// Content-addressed provenance used to distinguish corroboration from repetition.
///
/// Identity, origin, derivation, and influence are separate on purpose: two agents
/// can have distinct identities while still repeating evidence from the same origin
/// or from a shared context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentEvidenceLineage {
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
pub enum AgentEvidenceRelation {
    Independent,
    Correlated { reasons: Vec<String> },
    Unknown { missing_evidence: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "dependency", rename_all = "snake_case")]
pub enum AgentEvidenceDependency {
    SameEvidence,
    Derived,
    Influenced,
    Unrelated,
    Unknown { missing_evidence: Vec<String> },
}
>>>>>>> github/main

/// Classifies structural independence from declared, content-addressed lineage.
///
/// Distinct agents, nodes, or models are not sufficient. Independence requires
/// distinct attested producers and disjoint declared origins, derivations, and
/// influences. Missing lineage remains unknown rather than being promoted.
pub fn relate_agent_evidence(
    left: &AgentEvidenceRef,
    right: &AgentEvidenceRef,
) -> AgentEvidenceRelation {
<<<<<<< HEAD
    crate::evidence::relate_evidence(left, right)
=======
    let invalid_evidence = invalid_evidence_digest("left", left)
        .into_iter()
        .chain(invalid_evidence_digest("right", right))
        .collect::<Vec<_>>();
    if !invalid_evidence.is_empty() {
        return AgentEvidenceRelation::Unknown {
            missing_evidence: invalid_evidence,
        };
    }
    if left.digest == right.digest {
        return AgentEvidenceRelation::Correlated {
            reasons: vec!["same_evidence".to_string()],
        };
    }

    let missing_evidence = lineage_unknowns("left", left)
        .into_iter()
        .chain(lineage_unknowns("right", right))
        .collect::<Vec<_>>();
    if !missing_evidence.is_empty() {
        return AgentEvidenceRelation::Unknown { missing_evidence };
    }

    let left_lineage = left.lineage.as_ref().expect("lineage checked above");
    let right_lineage = right.lineage.as_ref().expect("lineage checked above");
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
        AgentEvidenceRelation::Independent
    } else {
        AgentEvidenceRelation::Correlated { reasons }
    }
>>>>>>> github/main
}

/// Reports whether an evidence item declares dependency on another digest.
///
/// A memory layer can use this without learning agent-specific identity rules:
/// `Influenced` or `Derived` evidence must not count as fresh corroboration of
/// the referenced item.
pub fn classify_agent_evidence_dependency(
    evidence: &AgentEvidenceRef,
    candidate_ancestor_digest: &str,
) -> AgentEvidenceDependency {
<<<<<<< HEAD
    crate::evidence::classify_evidence_dependency(evidence, candidate_ancestor_digest)
}

pub(super) fn lineage_unknowns(prefix: &str, evidence: &AgentEvidenceRef) -> Vec<String> {
    crate::evidence::evidence_lineage_unknowns(prefix, evidence)
}

pub(super) fn valid_sha256_digest(value: &str) -> bool {
    crate::evidence::valid_sha256_digest(value)
=======
    if !valid_sha256_digest(candidate_ancestor_digest) {
        return AgentEvidenceDependency::Unknown {
            missing_evidence: vec!["candidate_ancestor.valid_sha256_digest".to_string()],
        };
    }
    if let Some(missing_evidence) = invalid_evidence_digest("evidence", evidence) {
        return AgentEvidenceDependency::Unknown {
            missing_evidence: vec![missing_evidence],
        };
    }
    if evidence.digest == candidate_ancestor_digest {
        return AgentEvidenceDependency::SameEvidence;
    }

    let missing_evidence = lineage_unknowns("evidence", evidence);
    if !missing_evidence.is_empty() {
        return AgentEvidenceDependency::Unknown { missing_evidence };
    }
    let lineage = evidence.lineage.as_ref().expect("lineage checked above");
    if lineage
        .influence_digests
        .iter()
        .any(|digest| digest == candidate_ancestor_digest)
    {
        AgentEvidenceDependency::Influenced
    } else if lineage
        .derivation_digests
        .iter()
        .chain(&lineage.origin_digests)
        .any(|digest| digest == candidate_ancestor_digest)
    {
        AgentEvidenceDependency::Derived
    } else {
        AgentEvidenceDependency::Unrelated
    }
}

pub(super) fn lineage_unknowns(prefix: &str, evidence: &AgentEvidenceRef) -> Vec<String> {
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

fn invalid_evidence_digest(prefix: &str, evidence: &AgentEvidenceRef) -> Option<String> {
    (!valid_sha256_digest(&evidence.digest)).then(|| format!("{prefix}.valid_sha256_digest"))
}

fn shares_digest(left: &[String], right: &[String]) -> bool {
    left.iter().any(|digest| right.contains(digest))
}

pub(super) fn valid_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
>>>>>>> github/main
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(marker: char) -> String {
        format!("sha256:{}", marker.to_string().repeat(64))
    }

    fn evidence(
        marker: char,
        producer: char,
        origins: &[char],
        influences: &[char],
    ) -> AgentEvidenceRef {
        AgentEvidenceRef {
            kind: "runtime_proof".to_string(),
            subject: format!("observation-{marker}"),
            digest: digest(marker),
            lineage: Some(AgentEvidenceLineage {
                producer_id: format!("producer-{producer}"),
                producer_attestation_digest: digest(producer),
                origin_digests: origins.iter().copied().map(digest).collect(),
                derivation_digests: Vec::new(),
                influence_digests: influences.iter().copied().map(digest).collect(),
            }),
        }
    }

    #[test]
    fn same_origin_from_distinct_producers_is_correlated() {
        let left = evidence('1', 'a', &['c'], &[]);
        let right = evidence('2', 'b', &['c'], &[]);
        assert!(matches!(
            relate_agent_evidence(&left, &right),
            AgentEvidenceRelation::Correlated { reasons } if reasons == vec!["shared_origin"]
        ));
    }

    #[test]
    fn same_producer_with_distinct_origins_is_correlated() {
        let left = evidence('1', 'a', &['c'], &[]);
        let right = evidence('2', 'a', &['d'], &[]);
        assert!(matches!(
            relate_agent_evidence(&left, &right),
            AgentEvidenceRelation::Correlated { reasons }
                if reasons == vec!["same_producer", "shared_producer_attestation"]
        ));
    }

    #[test]
    fn rotated_attestation_does_not_turn_one_producer_into_two() {
        let left = evidence('1', 'a', &['c'], &[]);
        let mut right = evidence('2', 'b', &['d'], &[]);
        right.lineage.as_mut().expect("lineage").producer_id =
            left.lineage.as_ref().expect("lineage").producer_id.clone();
        assert!(matches!(
            relate_agent_evidence(&left, &right),
            AgentEvidenceRelation::Correlated { reasons } if reasons == vec!["same_producer"]
        ));
    }

    #[test]
    fn shared_recursive_context_is_correlated() {
        let left = evidence('1', 'a', &['c'], &['e']);
        let right = evidence('2', 'b', &['d'], &['e']);
        assert!(matches!(
            relate_agent_evidence(&left, &right),
            AgentEvidenceRelation::Correlated { reasons } if reasons == vec!["shared_influence"]
        ));
    }

    #[test]
    fn disjoint_lineages_are_independent() {
        let mut left = evidence('1', 'a', &['c'], &['5']);
        let mut right = evidence('2', 'b', &['d'], &['6']);
        left.lineage.as_mut().expect("lineage").derivation_digests = vec![digest('e')];
        right.lineage.as_mut().expect("lineage").derivation_digests = vec![digest('f')];
        assert_eq!(
            relate_agent_evidence(&left, &right),
            AgentEvidenceRelation::Independent
        );
    }

    #[test]
    fn missing_lineage_remains_unknown() {
        let left = AgentEvidenceRef {
            kind: "runtime_proof".to_string(),
            subject: "left".to_string(),
            digest: digest('1'),
            lineage: None,
        };
        let right = evidence('2', 'b', &['d'], &[]);
        assert!(matches!(
            relate_agent_evidence(&left, &right),
            AgentEvidenceRelation::Unknown { missing_evidence }
                if missing_evidence == vec!["left.lineage"]
        ));
    }

    #[test]
    fn declared_influence_is_not_fresh_corroboration() {
        let evidence = evidence('1', 'a', &['c'], &['e']);
        assert_eq!(
            classify_agent_evidence_dependency(&evidence, &digest('e')),
            AgentEvidenceDependency::Influenced
        );
    }
}
