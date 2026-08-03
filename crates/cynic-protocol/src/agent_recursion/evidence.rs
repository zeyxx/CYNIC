pub type AgentEvidenceRef = crate::evidence::EvidenceRef;
pub type AgentEvidenceLineage = crate::evidence::EvidenceLineage;
pub type AgentEvidenceRelation = crate::evidence::EvidenceRelation;
pub type AgentEvidenceDependency = crate::evidence::EvidenceDependency;

/// Classifies structural independence from declared, content-addressed lineage.
///
/// Distinct agents, nodes, or models are not sufficient. Independence requires
/// distinct attested producers and disjoint declared origins, derivations, and
/// influences. Missing lineage remains unknown rather than being promoted.
pub fn relate_agent_evidence(
    left: &AgentEvidenceRef,
    right: &AgentEvidenceRef,
) -> AgentEvidenceRelation {
    crate::evidence::relate_evidence(left, right)
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
    crate::evidence::classify_evidence_dependency(evidence, candidate_ancestor_digest)
}

pub(super) fn lineage_unknowns(prefix: &str, evidence: &AgentEvidenceRef) -> Vec<String> {
    crate::evidence::evidence_lineage_unknowns(prefix, evidence)
}

pub(super) fn valid_sha256_digest(value: &str) -> bool {
    crate::evidence::valid_sha256_digest(value)
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
