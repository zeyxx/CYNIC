//! Portable package distribution authority.
//!
//! A source label is not proof that a package can be acquired from that
//! source.  The selected authority must be explicit before a package mutator
//! can be planned.  Observation and verification adapters will subsequently
//! supply the read-back evidence for the selected variant.

use serde::{Deserialize, Serialize};

pub const PACKAGE_PROVENANCE_POLICY_SCHEMA: &str = "cynic.package_provenance_policy.v1";
pub const PACKAGE_PROMOTION_POLICY_SCHEMA: &str = "cynic.package_promotion_policy.v1";
pub const PACKAGE_REPOSITORY_OBSERVATION_SCHEMA: &str = "cynic.package_repository_observation.v1";

/// Promotion policy for critical host components. Metadata freshness limits
/// replay exposure; incubation limits immediate promotion of a newly published
/// candidate. They are deliberately separate controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackagePromotionPolicy {
    pub schema: String,
    pub max_metadata_age_secs: u64,
    pub min_candidate_age_secs: u64,
    pub normal_human_approvals: u8,
    pub emergency_human_approvals: u8,
}

impl PackagePromotionPolicy {
    pub fn critical_infrastructure() -> Self {
        Self {
            schema: PACKAGE_PROMOTION_POLICY_SCHEMA.to_string(),
            max_metadata_age_secs: 7 * 24 * 60 * 60,
            min_candidate_age_secs: 7 * 24 * 60 * 60,
            normal_human_approvals: 1,
            emergency_human_approvals: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackagePromotionObservation {
    pub metadata_observed_at_unix_s: Option<u64>,
    pub candidate_observed_at_unix_s: Option<u64>,
    pub signature_verified: bool,
    pub human_approval_count: u8,
    pub emergency: bool,
}

impl Default for PackagePromotionObservation {
    fn default() -> Self {
        Self {
            metadata_observed_at_unix_s: None,
            candidate_observed_at_unix_s: None,
            signature_verified: false,
            human_approval_count: 0,
            emergency: false,
        }
    }
}

/// Read-back receipt emitted by a repository-specific observer. A configured
/// source is not evidence: the receipt must bind its source and key identity
/// to a verified signature and durable proof reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageRepositoryObservation {
    pub schema: String,
    pub source_id: String,
    pub trusted_key_fingerprint: String,
    pub signature_verified: bool,
    pub metadata_observed_at_unix_s: Option<u64>,
    pub candidate_observed_at_unix_s: Option<u64>,
    pub human_approval_count: u8,
    pub emergency: bool,
    pub proof_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackagePromotionAssessment {
    pub eligible: bool,
    pub blockers: Vec<String>,
}

pub fn assess_package_promotion(
    policy: &PackagePromotionPolicy,
    observed: &PackagePromotionObservation,
    assessed_at_unix_s: u64,
) -> PackagePromotionAssessment {
    let mut blockers = Vec::new();
    if policy.schema != PACKAGE_PROMOTION_POLICY_SCHEMA
        || policy.max_metadata_age_secs == 0
        || policy.min_candidate_age_secs == 0
        || policy.normal_human_approvals == 0
        || policy.emergency_human_approvals < policy.normal_human_approvals
    {
        blockers.push("package_promotion_policy_invalid".to_string());
    }
    if assessed_at_unix_s == 0 {
        blockers.push("package_promotion_clock_missing".to_string());
    }
    if !observed.signature_verified {
        blockers.push("package_provenance_signature_unverified".to_string());
    }
    match observed.metadata_observed_at_unix_s {
        Some(observed_at) if assessed_at_unix_s >= observed_at => {
            if assessed_at_unix_s - observed_at > policy.max_metadata_age_secs {
                blockers.push("package_metadata_stale".to_string());
            }
        }
        Some(_) => blockers.push("package_metadata_observed_in_future".to_string()),
        None => blockers.push("package_metadata_freshness_unknown".to_string()),
    }
    if !observed.emergency {
        match observed.candidate_observed_at_unix_s {
            Some(observed_at) if assessed_at_unix_s >= observed_at => {
                if assessed_at_unix_s - observed_at < policy.min_candidate_age_secs {
                    blockers.push("package_candidate_incubation_incomplete".to_string());
                }
            }
            Some(_) => blockers.push("package_candidate_observed_in_future".to_string()),
            None => blockers.push("package_candidate_age_unknown".to_string()),
        }
    }
    let required_approvals = if observed.emergency {
        policy.emergency_human_approvals
    } else {
        policy.normal_human_approvals
    };
    if observed.human_approval_count < required_approvals {
        blockers.push("package_human_approval_threshold_unmet".to_string());
    }
    blockers.sort();
    blockers.dedup();
    PackagePromotionAssessment {
        eligible: blockers.is_empty(),
        blockers,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PackageProvenancePolicy {
    /// No distribution authority has been selected. This is deliberately a
    /// blocker, rather than a permissive fallback to the host package manager.
    Unresolved { reason: String },
    /// A repository whose metadata signature, key identity, availability and
    /// freshness must be observed by a repository adapter before apply.
    Repository {
        source_id: String,
        trusted_key_fingerprint: String,
        max_metadata_age_secs: u64,
    },
    /// A standalone, content-addressed release whose digest and signing
    /// identity must be verified by an artifact adapter before apply.
    SignedArtifact {
        artifact_id: String,
        sha256: String,
        signing_identity: String,
    },
}

impl PackageProvenancePolicy {
    pub fn unresolved_distribution_authority() -> Self {
        Self::Unresolved {
            reason: "distribution_authority_not_selected".to_string(),
        }
    }

    pub fn is_selected(&self) -> bool {
        !matches!(self, Self::Unresolved { .. })
    }

    pub fn blocker(&self) -> Option<String> {
        match self {
            Self::Unresolved { reason } => Some(reason.clone()),
            Self::Repository {
                source_id,
                trusted_key_fingerprint,
                max_metadata_age_secs,
            } if source_id.is_empty()
                || trusted_key_fingerprint.is_empty()
                || *max_metadata_age_secs == 0 =>
            {
                Some("repository_provenance_policy_invalid".to_string())
            }
            Self::SignedArtifact {
                artifact_id,
                sha256,
                signing_identity,
            } if artifact_id.is_empty() || !is_sha256(sha256) || signing_identity.is_empty() => {
                Some("artifact_provenance_policy_invalid".to_string())
            }
            _ => None,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Unresolved { reason } => format!("unresolved:{reason}"),
            Self::Repository { source_id, .. } => format!("repository:{source_id}"),
            Self::SignedArtifact { artifact_id, .. } => format!("signed_artifact:{artifact_id}"),
        }
    }
}

/// Bind a repository receipt to the selected distribution authority before
/// using it in generic freshness and promotion policy.
pub fn promotion_observation_for_repository(
    policy: &PackageProvenancePolicy,
    observed: &PackageRepositoryObservation,
) -> Result<PackagePromotionObservation, String> {
    let PackageProvenancePolicy::Repository {
        source_id,
        trusted_key_fingerprint,
        ..
    } = policy
    else {
        return Err("package_repository_provenance_not_selected".to_string());
    };
    if observed.schema != PACKAGE_REPOSITORY_OBSERVATION_SCHEMA {
        return Err("package_repository_observation_schema_invalid".to_string());
    }
    if observed.source_id.as_str() != source_id.as_str() {
        return Err("package_repository_source_mismatch".to_string());
    }
    if observed.trusted_key_fingerprint.as_str() != trusted_key_fingerprint.as_str() {
        return Err("package_repository_key_fingerprint_mismatch".to_string());
    }
    if observed.proof_refs.is_empty()
        || observed
            .proof_refs
            .iter()
            .any(|proof| proof.trim().is_empty())
    {
        return Err("package_repository_proof_missing".to_string());
    }
    Ok(PackagePromotionObservation {
        metadata_observed_at_unix_s: observed.metadata_observed_at_unix_s,
        candidate_observed_at_unix_s: observed.candidate_observed_at_unix_s,
        signature_verified: observed.signature_verified,
        human_approval_count: observed.human_approval_count,
        emergency: observed.emergency,
    })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::{
        PACKAGE_REPOSITORY_OBSERVATION_SCHEMA, PackagePromotionObservation, PackagePromotionPolicy,
        PackageProvenancePolicy, PackageRepositoryObservation, assess_package_promotion,
        promotion_observation_for_repository,
    };

    #[test]
    fn unresolved_authority_is_an_explicit_blocker() {
        let policy = PackageProvenancePolicy::unresolved_distribution_authority();
        assert_eq!(
            policy.blocker().as_deref(),
            Some("distribution_authority_not_selected")
        );
    }

    #[test]
    fn repository_requires_identity_and_freshness_window() {
        let policy = PackageProvenancePolicy::Repository {
            source_id: "internal-apt".to_string(),
            trusted_key_fingerprint: String::new(),
            max_metadata_age_secs: 0,
        };
        assert_eq!(
            policy.blocker().as_deref(),
            Some("repository_provenance_policy_invalid")
        );
    }

    #[test]
    fn artifact_requires_a_full_digest_and_identity() {
        let policy = PackageProvenancePolicy::SignedArtifact {
            artifact_id: "coredns".to_string(),
            sha256: "0".repeat(64),
            signing_identity: "release@example.invalid".to_string(),
        };
        assert_eq!(policy.blocker(), None);
    }

    #[test]
    fn critical_candidate_requires_seven_day_incubation_and_human_approval() {
        let policy = PackagePromotionPolicy::critical_infrastructure();
        let assessed_at = 2_000_000;
        let candidate = PackagePromotionObservation {
            metadata_observed_at_unix_s: Some(assessed_at),
            candidate_observed_at_unix_s: Some(assessed_at - policy.min_candidate_age_secs + 1),
            signature_verified: true,
            human_approval_count: 0,
            emergency: false,
        };
        let assessment = assess_package_promotion(&policy, &candidate, assessed_at);
        assert!(!assessment.eligible);
        assert!(
            assessment
                .blockers
                .contains(&"package_candidate_incubation_incomplete".to_string())
        );
        assert!(
            assessment
                .blockers
                .contains(&"package_human_approval_threshold_unmet".to_string())
        );
    }

    #[test]
    fn emergency_bypasses_incubation_but_requires_two_human_approvals() {
        let policy = PackagePromotionPolicy::critical_infrastructure();
        let assessed_at = 2_000_000;
        let candidate = PackagePromotionObservation {
            metadata_observed_at_unix_s: Some(assessed_at),
            candidate_observed_at_unix_s: Some(assessed_at),
            signature_verified: true,
            human_approval_count: 2,
            emergency: true,
        };
        assert!(assess_package_promotion(&policy, &candidate, assessed_at).eligible);
    }

    #[test]
    fn repository_receipt_must_match_selected_source_and_key() {
        let policy = PackageProvenancePolicy::Repository {
            source_id: "cynic-apt".to_string(),
            trusted_key_fingerprint: "0123456789abcdef".to_string(),
            max_metadata_age_secs: 86_400,
        };
        let receipt = PackageRepositoryObservation {
            schema: PACKAGE_REPOSITORY_OBSERVATION_SCHEMA.to_string(),
            source_id: "cynic-apt".to_string(),
            trusted_key_fingerprint: "0123456789abcdef".to_string(),
            signature_verified: true,
            metadata_observed_at_unix_s: Some(1),
            candidate_observed_at_unix_s: Some(1),
            human_approval_count: 1,
            emergency: false,
            proof_refs: vec!["proof://apt/release".to_string()],
        };

        assert!(promotion_observation_for_repository(&policy, &receipt).is_ok());

        let mut wrong_key = receipt;
        wrong_key.trusted_key_fingerprint = "other".to_string();
        assert_eq!(
            promotion_observation_for_repository(&policy, &wrong_key),
            Err("package_repository_key_fingerprint_mismatch".to_string())
        );
    }
}
